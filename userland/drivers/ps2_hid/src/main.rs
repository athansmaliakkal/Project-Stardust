/*
 * Stardust PS/2 Human Interface Device (HID) Driver
 * 
 * This driver manages the legacy i8042 PS/2 controller, providing support 
 * for both keyboard (Port 1) and mouse (Port 2) input. It translates raw 
 * hardware scancodes and data packets into structured system events 
 * delivered via the Stardust IPC mechanism.
 *
 * Architecture:
 * 1. Controller Initialization: Enables the auxiliary mouse port and 
 *    configures the device for data reporting.
 * 2. Event Polling Loop: Monitors the status register (Port 0x64) for 
 *    the Output Buffer Full (OBF) flag.
 * 3. Protocol Decoding: Handles the 3-byte standard PS/2 mouse protocol 
 *    and variable-length keyboard scancodes.
 * 4. IPC Dispatch: Packages HID state changes into 64-bit messages for 
 *    the system UI supervisor.
 */

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { loop {} }

/*
 * _start: Driver Entry Point
 * 
 * Performs hardware initialization and enters the primary event loop.
 */
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    /*
     * Hardware Initialization Sequence
     * 0xA8: Enable Second PS/2 Port (Mouse)
     * 0xD4: Write Next Byte to Mouse Port
     * 0xF4: Enable Data Reporting (Mouse Command)
     */
    libstardust::sys_port_out(0x64, 0xA8); 
    libstardust::sys_port_out(0x64, 0xD4); 
    libstardust::sys_port_out(0x60, 0xF4); 

    /*
     * Wait for Acknowledge (0xFA) from the mouse controller.
     */
    let mut timeout = 100000;
    while timeout > 0 {
        let status = libstardust::sys_port_in(0x64, 1);
        if (status & 0x01) != 0 {
            let data = libstardust::sys_port_in(0x60, 1);
            if data == 0xFA { break; } 
        }
        timeout -= 1;
        core::hint::spin_loop();
    }

    let mut mouse_cycle = 0;
    let mut mouse_packet: [u8; 3] = [0; 3];

    loop {
        let status = libstardust::sys_port_in(0x64, 1);
        
        // Check if data is available in the output buffer
        if (status & 0x01) != 0 {
            let data = libstardust::sys_port_in(0x60, 1) as u8;
            let is_mouse = (status & 0x20) != 0;

            if is_mouse {
                /*
                 * Mouse Packet Assembly
                 * The standard PS/2 mouse protocol uses 3-byte packets:
                 * Byte 0: Flags (Buttons, Overflow, Sign Bits)
                 * Byte 1: X Delta
                 * Byte 2: Y Delta
                 */
                mouse_packet[mouse_cycle] = data;
                mouse_cycle += 1;

                if mouse_cycle == 3 {
                    mouse_cycle = 0;
                    
                    let flags = mouse_packet[0];
                    let dx = mouse_packet[1];
                    let dy = mouse_packet[2];

                    // Protocol: ID 0x5E for Mouse Events
                    let payload = (0x5E_u64 << 24) 
                                | ((flags as u64) << 16) 
                                | ((dx as u64) << 8) 
                                | (dy as u64);
                                
                    // Dispatch to the UI Supervisor (PID 1)
                    libstardust::sys_ipc_send(1, payload);
                }
            } else {
                /*
                 * Keyboard Event Dispatch
                 * Protocol: ID 0x11 for Keyboard Events
                 */
                let payload = (0x11_u64 << 24) | (data as u64);
                libstardust::sys_ipc_send(1, payload);
            }
        } else {
            core::hint::spin_loop();
        }
    }
}

/*
 * Low-level memory intrinsics for the no_std environment.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        unsafe { core::ptr::write_volatile(s.add(i), c as u8); }
        i += 1;
    }
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        unsafe { core::ptr::write_volatile(dest.add(i), core::ptr::read_volatile(src.add(i))); }
        i += 1;
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if src < dest as *const u8 {
        let mut i = n;
        while i > 0 {
            i -= 1;
            unsafe { core::ptr::write_volatile(dest.add(i), core::ptr::read_volatile(src.add(i))); }
        }
    } else {
        let mut i = 0;
        while i < n {
            unsafe { core::ptr::write_volatile(dest.add(i), core::ptr::read_volatile(src.add(i))); }
            i += 1;
        }
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        unsafe {
            let a = core::ptr::read_volatile(s1.add(i));
            let b = core::ptr::read_volatile(s2.add(i));
            if a != b { return (a as i32) - (b as i32); }
        }
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe { memcmp(s1, s2, n) }
}
