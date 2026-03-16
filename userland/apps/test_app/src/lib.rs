/*
 * Stardust WebAssembly User Interface Prototype
 * 
 * This module implements a high-level UI component running within the
 * sandboxed WebAssembly execution environment. It demonstrates the
 * portability of the Stardust userland by decoupling application logic
 * from the underlying kernel architecture.
 *
 * Design:
 * - Event-Driven: The UI reacts to input messages received via the 
 *   host's IPC bridge.
 * - Software Rendering: All drawing operations are proxied through
 *   the 'draw_rect' host function.
 * - State Management: Handles window positioning, cursor tracking,
 *   and simple interaction states (e.g., dragging).
 */

#![no_std]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/*
 * External Host Imports
 * 
 * These functions are provided by the 'Marshal' process (the Wasm host).
 * They represent the narrow ABI through which sandboxed applications
 * interact with the Stardust system services.
 */
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    /*
     * draw_rect: Fills a rectangular region in the system framebuffer.
     * x, y: Top-left coordinates.
     * width, height: Dimensions of the rectangle.
     * color: 32-bit ARGB color value.
     */
    fn draw_rect(x: i32, y: i32, width: i32, height: i32, color: i32);

    /*
     * get_ipc_message: Polls for the next pending IPC message.
     * Returns a 64-bit encoded message or 0 if no message is available.
     */
    fn get_ipc_message() -> i64;
}

/*
 * draw_cursor: Renders a hardware-independent mouse pointer.
 */
fn draw_cursor(x: i32, y: i32) {
    unsafe {
        draw_rect(x, y, 6, 6, 0xFFFFFF_u32 as i32);
        draw_rect(x + 1, y + 1, 4, 4, 0x000000_u32 as i32);
    }
}

/*
 * draw_window: Renders a composite window structure.
 * 
 * Demonstrates a classic "Z-order" rendering technique where
 * components are drawn from back to front.
 */
fn draw_window(x: i32, y: i32, w: i32, h: i32, title_color: u32) {
    unsafe {
        // Shadow effect for depth perception
        draw_rect(x + 6, y + 6, w, h, 0x111111_u32 as i32);
        
        // Window Frame
        draw_rect(x, y, w, h, 0xDDDDDD_u32 as i32);
        
        // Title Bar
        draw_rect(x, y, w, 24, title_color as i32);
        
        // Close Button (Mock)
        draw_rect(x + w - 24, y + 4, 16, 16, 0xFF4444_u32 as i32);
        
        // Interior Content Area
        draw_rect(x + 10, y + 34, w - 20, h - 44, 0xFFFFFF_u32 as i32);
        
        // Placeholder UI Elements
        draw_rect(x + 30, y + 60, 100, 20, 0x3366FF_u32 as i32); 
        draw_rect(x + 30, y + 100, 200, 10, 0xAAAAAA_u32 as i32);
        draw_rect(x + 30, y + 120, 180, 10, 0xAAAAAA_u32 as i32);
        draw_rect(x + 30, y + 140, 220, 10, 0xAAAAAA_u32 as i32);
    }
}

/*
 * run_ui_loop: Primary Application Entry Point
 * 
 * Implements the main UI loop, handling input translation and 
 * view updates. 
 *
 * Protocol:
 * - Mouse Packet (ID 0x5E): [ID:8][Flags:8][dX:8][dY:8]
 * - Keyboard Packet (ID 0x11): [ID:8][Unused:16][Scancode:8]
 */
#[unsafe(no_mangle)]
pub extern "C" fn run_ui_loop(fb_width: i32, fb_height: i32) {
    let mut mouse_x = fb_width / 2;
    let mut mouse_y = fb_height / 2;
    let mut win_x = 200;
    let mut win_y = 150;
    let win_w = 400;
    let win_h = 250;
    let mut is_dragging = false;
    let mut drag_offset_x = 0;
    let mut drag_offset_y = 0;
    let mut win_color = 0x0055AA; 

    let desktop_color = 0x223344;
    
    // Initial Full Screen Render
    unsafe { draw_rect(0, 0, fb_width, fb_height, desktop_color as i32); }
    draw_window(win_x, win_y, win_w, win_h, win_color);
    draw_cursor(mouse_x, mouse_y);

    loop {
        let msg = unsafe { get_ipc_message() } as u64;
        
        if msg != 0 {
            let id = (msg >> 24) & 0xFF;
            let old_mx = mouse_x;
            let old_my = mouse_y;
            let old_wx = win_x;
            let old_wy = win_y;
            let mut needs_redraw = false;

            /*
             * Handle HID Input: Mouse
             */
            if id == 0x5E { 
                let flags = ((msg >> 16) & 0xFF) as u8;
                let dx = ((msg >> 8) & 0xFF) as i8 as i32;
                let dy = (msg & 0xFF) as i8 as i32;
                let left_click = (flags & 0x01) != 0;

                mouse_x += dx;
                mouse_y -= dy; 

                // Boundary Enforcement
                if mouse_x < 0 { mouse_x = 0; }
                if mouse_y < 0 { mouse_y = 0; }
                if mouse_x > fb_width - 6 { mouse_x = fb_width - 6; }
                if mouse_y > fb_height - 6 { mouse_y = fb_height - 6; }

                /*
                 * Drag Logic: Move the window if the left button is held
                 * within the title bar area.
                 */
                if left_click {
                    if !is_dragging {
                        if mouse_x >= win_x && mouse_x <= win_x + win_w &&
                           mouse_y >= win_y && mouse_y <= win_y + 24 {
                            is_dragging = true;
                            drag_offset_x = mouse_x - win_x;
                            drag_offset_y = mouse_y - win_y;
                        }
                    } else {
                        win_x = mouse_x - drag_offset_x;
                        win_y = mouse_y - drag_offset_y;
                    }
                } else {
                    is_dragging = false;
                }
                needs_redraw = true;

            /*
             * Handle HID Input: Keyboard
             */
            } else if id == 0x11 { 
                let scancode = msg & 0xFF;
                // Update title color on key press to demonstrate interactivity
                if scancode < 0x80 {
                    win_color = (win_color + 0x001133) & 0xFFFFFF;
                    needs_redraw = true;
                }
            }

            /*
             * Optimization: Only redraw when the state changes.
             * Note: In a production environment, this would use dirty rects
             * rather than clearing entire component bounds.
             */
            if needs_redraw {
                unsafe { 
                    draw_rect(old_mx, old_my, 6, 6, desktop_color as i32);
                    draw_rect(old_wx, old_wy, win_w + 6, win_h + 6, desktop_color as i32);
                }
                draw_window(win_x, win_y, win_w, win_h, win_color);
                draw_cursor(mouse_x, mouse_y);
            }
        }
    }
}
