#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;
use wasmi::{Engine, Module, Store, Linker};

struct AlignedAllocator {
    inner: LockedHeap,
}

unsafe impl GlobalAlloc for AlignedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = core::cmp::max(layout.align(), 16);
        let size = (layout.size() + align - 1) & !(align - 1);
        if let Ok(aligned_layout) = Layout::from_size_align(size, align) {
            unsafe { self.inner.alloc(aligned_layout) }
        } else {
            core::ptr::null_mut()
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let align = core::cmp::max(layout.align(), 16);
        let size = (layout.size() + align - 1) & !(align - 1);
        if let Ok(aligned_layout) = Layout::from_size_align(size, align) {
            unsafe { self.inner.dealloc(ptr, aligned_layout); }
        }
    }
}

#[global_allocator]
static ALLOCATOR: AlignedAllocator = AlignedAllocator { inner: LockedHeap::empty() };

pub const HEAP_START: usize = 0x0000_4000_0000_0000;
pub const HEAP_SIZE: usize = 16 * 1024 * 1024; 

pub fn init_heap() {
    let pages = HEAP_SIZE / 4096;
    for i in 0..pages {
        let vaddr = (HEAP_START + (i * 4096)) as u64;
        let _ = libstardust::sys_grant_shared_memory(0, vaddr);
    }
    unsafe { 
        memset(HEAP_START as *mut u8, 0, HEAP_SIZE);
        ALLOCATOR.inner.lock().init(HEAP_START as *mut u8, HEAP_SIZE); 
    }
}

fn trace_checkpoint(id: u64) {
    let cmd = [0x0000_DB60_0000_0000 | id];
    libstardust::sys_batch_execute(cmd.as_ptr(), 1);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { 
    trace_checkpoint(0xDEADBEEF);
    loop {} 
}

unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

static INITRAMFS_DATA: &[u8] = include_bytes!("initramfs.tar");

static mut FB_PTR: *mut u32 = core::ptr::null_mut();
static mut FB_WIDTH: u64 = 0;
static mut FB_HEIGHT: u64 = 0;

static mut WASM_DATA_PTR: *const u8 = core::ptr::null();
static mut WASM_DATA_LEN: usize = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        let bss_start = &raw mut __bss_start as *mut u8;
        let bss_end = &raw mut __bss_end as *mut u8;
        memset(bss_start, 0, bss_end as usize - bss_start as usize);
    }

    init_heap();
    let object_table = libstardust::tar::parse(INITRAMFS_DATA);

    // =========================================================================
    // DYNAMIC INIT PROTOCOL (THE SYSTEMD OF STARDUST)
    // Parses `init.rc` line-by-line. Dynamically offsets the memory layout
    // for every Native Driver found, ensuring zero collisions forever.
    // =========================================================================
    let mut current_stack_base = 0x0000_6000_0000_0000;

    for object in &object_table {
        if object.name == "init.rc" {
            if let Ok(config_str) = core::str::from_utf8(object.data) {
                for line in config_str.lines() {
                    let line = line.trim();
                    if line.starts_with("driver:") {
                        let driver_name = &line[7..];
                        for drv_obj in &object_table {
                            if drv_obj.name == driver_name {
                                if let Some(entry_point) = libstardust::elf::load(drv_obj.data) {
                                    let stack_base = current_stack_base;
                                    for p in 0..16 { 
                                        let _ = libstardust::sys_grant_shared_memory(0, stack_base + (p * 4096)); 
                                    }
                                    libstardust::sys_spawn_process(entry_point, stack_base + (16 * 4096));
                                    
                                    // Step 1MB forward in Virtual Space for the next driver
                                    current_stack_base += 0x10_0000; 
                                }
                            }
                        }
                    } else if line.starts_with("app:") {
                        let app_name = &line[4..];
                        for app_obj in &object_table {
                            if app_obj.name == app_name {
                                unsafe {
                                    WASM_DATA_PTR = app_obj.data.as_ptr();
                                    WASM_DATA_LEN = app_obj.data.len();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

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

#[unsafe(no_mangle)]
pub extern "C" fn wasm_execution_stage() -> ! {
    let wasm_data = unsafe { core::slice::from_raw_parts(WASM_DATA_PTR, WASM_DATA_LEN) };
    
    let mut config = wasmi::Config::default();
    config.consume_fuel(false); 
    let engine = Engine::new(&config);
    
    if let Ok(module) = Module::new(&engine, wasm_data) {
        let mut store = Store::new(&engine, ());
        let mut linker = <Linker<()>>::new(&engine);
        
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