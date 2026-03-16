/*
 * Stardust Userland Marshal (The Supervisor Process)
 * 
 * The Marshal is the first process executed by the kernel after the initial
 * boot sequence. Its primary responsibility is to transition the system from
 * a minimal kernel-provided environment into a fully functional userland.
 *
 * Architecture:
 * 1. Bootstrap: Initializes a local heap and basic runtime services.
 * 2. Initramfs Parsing: Extracts system drivers and services from the
 *    embedded initramfs (TAR format).
 * 3. Service Orchestration: Spawns essential drivers (PS/2 HID, GPU) into
 *    their own isolated address spaces.
 * 4. Runtime Environment: Sets up a WebAssembly (wasmi) execution stage
 *    to run high-level applications in a sandboxed, portable environment.
 * 5. Graphics/IPC Bridging: Provides the Wasm environment with access to
 *    system primitives like the framebuffer and IPC.
 */

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;
use wasmi::{Engine, Module, Store, Linker};

/*
 * Global Heap Allocator
 *
 * Uses a linked-list allocator for userland memory management.
 * The heap is backed by shared memory granted by the kernel via
 * sys_grant_shared_memory, effectively mapping physical frames
 * into the process's virtual address space at HEAP_START.
 */
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x0000_4000_0000_0000;
pub const HEAP_SIZE: usize = 16 * 1024 * 1024; // 16 MB Regional Heap

/*
 * init_heap: Prepares the userland dynamic memory region.
 *
 * Iterates through the virtual address range and requests the kernel to 
 * map physical frames. This is a demand-paging alternative where the 
 * loader explicitly populates its memory map before usage.
 */
pub fn init_heap() {
    let pages = HEAP_SIZE / 4096;
    for i in 0..pages {
        let vaddr = (HEAP_START + (i * 4096)) as u64;
        let _ = libstardust::sys_grant_shared_memory(0, vaddr);
    }
    unsafe { 
        memset(HEAP_START as *mut u8, 0, HEAP_SIZE);
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE); 
    }
}

/*
 * trace_checkpoint: Diagnostic Output Mechanism
 *
 * Communicates progress or failure states to the kernel or hardware
 * debugger by executing a batched command with a specific magic ID.
 */
fn trace_checkpoint(id: u64) {
    let cmd = [0x0000_DB60_0000_0000 | id];
    libstardust::sys_batch_execute(cmd.as_ptr(), 1);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { 
    trace_checkpoint(0xDEADBEEF);
    loop {} 
}

/*
 * Symbols defined by the linker script (linker.ld)
 * Used to zero-initialize the BSS section during bootstrap.
 */
unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

/*
 * Embedded Initramfs
 * Contains the ELFs and assets required for the initial userland state.
 */
static INITRAMFS_DATA: &[u8] = include_bytes!("initramfs.tar");

/*
 * Global Graphics Context
 * Cached metadata for the kernel-provided framebuffer.
 */
static mut FB_PTR: *mut u32 = core::ptr::null_mut();
static mut FB_WIDTH: u64 = 0;
static mut FB_HEIGHT: u64 = 0;

static mut WASM_DATA_PTR: *const u8 = core::ptr::null();
static mut WASM_DATA_LEN: usize = 0;

/*
 * _start: Userland Entry Point
 * 
 * Performs low-level runtime initialization, parses the initramfs,
 * and orchestrates the spawning of system drivers before pivoting 
 * to the WebAssembly execution stage.
 */
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        let bss_start = &raw mut __bss_start as *mut u8;
        let bss_end = &raw mut __bss_end as *mut u8;
        memset(bss_start, 0, bss_end as usize - bss_start as usize);
    }

    init_heap();
    let object_table = libstardust::tar::parse(INITRAMFS_DATA);

    /*
     * Driver Initialization Phase: PS/2 HID
     */
    for object in &object_table {
        if object.name == "ps2_hid" {
            if let Some(entry_point) = libstardust::elf::load(object.data) {
                let stack_base = 0x0000_6000_0000_0000;
                for p in 0..16 { let _ = libstardust::sys_grant_shared_memory(0, stack_base + (p * 4096)); }
                libstardust::sys_spawn_process(entry_point, stack_base + (16 * 4096));
            }
        }
    }
    
    /*
     * Driver Initialization Phase: GPU Driver
     */
    for object in &object_table {
        if object.name == "gpu_driver" {
            if let Some(entry_point) = libstardust::elf::load(object.data) {
                let stack_base = 0x0000_6000_0010_0000; 
                for p in 0..16 { let _ = libstardust::sys_grant_shared_memory(0, stack_base + (p * 4096)); }
                libstardust::sys_spawn_process(entry_point, stack_base + (16 * 4096));
            }
        }
    }

    /*
     * Graphics Subsystem Setup
     * Maps the framebuffer provided by the kernel into the Marshal's address space.
     */
    let _ = libstardust::sys_grant_shared_memory(0, 0x4000_0000);
    libstardust::sys_iommu_lockdown(0x10DE, 0xA000_0000);
    let fb_meta = libstardust::sys_map_framebuffer(0xB000_0000);

    if fb_meta != 0 && fb_meta != u64::MAX {
        unsafe {
            let pitch = fb_meta >> 32;
            FB_HEIGHT = fb_meta & 0xFFFFFFFF;
            FB_WIDTH = pitch / 4; 
            FB_PTR = 0xB000_0000 as *mut u32;
        }
    }

    for object in &object_table {
        if object.name == "test_app.wasm" {
            unsafe {
                WASM_DATA_PTR = object.data.as_ptr();
                WASM_DATA_LEN = object.data.len();
            }
        }
    }

    /*
     * Application Stack Pivot
     * 
     * The Wasm engine requires a significant amount of stack space. We allocate
     * a large (16MB) region and perform an assembly pivot to ensure correct
     * alignment (16-byte) for the SysV ABI before jumping into the execution stage.
     */
    trace_checkpoint(0x7500); 
    let deep_stack_base = 0x0000_6000_00F0_0000;
    let deep_stack_size = 16 * 1024 * 1024;
    let pages = deep_stack_size / 4096;
    
    for p in 0..pages { 
        let _ = libstardust::sys_grant_shared_memory(0, deep_stack_base + (p * 4096)); 
    }
    
    unsafe { memset(deep_stack_base as *mut u8, 0, deep_stack_size as usize); }
    
    let deep_stack_aligned = (deep_stack_base + deep_stack_size) & !0xF; 

    unsafe {
        core::arch::asm!(
            "mov rsp, {0}",
            "call {1}",
            in(reg) deep_stack_aligned,
            sym wasm_execution_stage,
            options(noreturn)
        );
    }
}

/*
 * wasm_execution_stage: WebAssembly Runtime Initialization
 * 
 * This stage initializes the 'wasmi' engine, sets up the sandbox,
 * and exports system primitives to the guest WebAssembly environment.
 */
#[unsafe(no_mangle)]
pub extern "C" fn wasm_execution_stage() -> ! {
    let wasm_data = unsafe { core::slice::from_raw_parts(WASM_DATA_PTR, WASM_DATA_LEN) };
    
    let mut config = wasmi::Config::default();
    config.consume_fuel(false); 
    let engine = Engine::new(&config);
    
    if let Ok(module) = Module::new(&engine, wasm_data) {
        let mut store = Store::new(&engine, ());
        let mut linker = <Linker<()>>::new(&engine);
        
        /*
         * Exported Host Function: draw_rect
         * Allows the Wasm application to perform volatile writes to the 
         * system framebuffer with boundary checking.
         */
        let _ = linker.func_wrap("env", "draw_rect", |x: i32, y: i32, width: i32, height: i32, color: i32| {
            unsafe {
                if FB_PTR.is_null() || x < 0 || y < 0 { return; }
                let mut draw_w = width; let mut draw_h = height;
                let c = color as u32;
                if (x as u64) + (draw_w as u64) > FB_WIDTH { draw_w = (FB_WIDTH - x as u64) as i32; }
                if (y as u64) + (draw_h as u64) > FB_HEIGHT { draw_h = (FB_HEIGHT - y as u64) as i32; }
                for row in 0..draw_h {
                    for col in 0..draw_w {
                        let offset = ((y + row) as u64 * FB_WIDTH) + (x + col) as u64;
                        core::ptr::write_volatile(FB_PTR.add(offset as usize), c);
                    }
                }
            }
        });

        /*
         * Exported Host Function: get_ipc_message
         * Provides the Wasm application with access to the Stardust IPC mechanism.
         */
        let _ = linker.func_wrap("env", "get_ipc_message", || -> i64 {
            let msg = libstardust::sys_ipc_receive(1);
            if msg == 0 { core::hint::spin_loop(); }
            msg as i64
        });

        if let Ok(instance) = linker.instantiate(&mut store, &module) {
            if let Ok(started) = instance.start(&mut store) {
                if let Ok(run_func) = started.get_typed_func::<(i32, i32), ()>(&store, "run_ui_loop") {
                    
                    trace_checkpoint(0x8060); 
                    unsafe { let _ = run_func.call(&mut store, (FB_WIDTH as i32, FB_HEIGHT as i32)); }
                }
            }
        }
    }

    loop { core::hint::spin_loop(); }
}

/*
 * Low-level memory intrinsics.
 * These are implemented using volatile operations to ensure the compiler
 * does not optimize away required memory accesses in this environment.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0; while i < n { unsafe { core::ptr::write_volatile(s.add(i), c as u8); } i += 1; } s
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0; while i < n { unsafe { core::ptr::write_volatile(dest.add(i), core::ptr::read_volatile(src.add(i))); } i += 1; } dest
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if src < dest as *const u8 {
        let mut i = n; while i > 0 { i -= 1; unsafe { core::ptr::write_volatile(dest.add(i), core::ptr::read_volatile(src.add(i))); } }
    } else {
        let mut i = 0; while i < n { unsafe { core::ptr::write_volatile(dest.add(i), core::ptr::read_volatile(src.add(i))); } i += 1; }
    } dest
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        unsafe {
            let a = core::ptr::read_volatile(s1.add(i)); let b = core::ptr::read_volatile(s2.add(i));
            if a != b { return (a as i32) - (b as i32); }
        } i += 1;
    } 0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 { unsafe { memcmp(s1, s2, n) } }
