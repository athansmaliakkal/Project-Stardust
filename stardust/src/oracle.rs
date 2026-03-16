/*
 * The Oracle: Kernel Logging and Diagnostics.
 *
 * This module provides a simple UART-based serial output mechanism. It is
 * used for early boot diagnostics and system-wide logging before a graphical
 * console or complex driver is available.
 *
 * I/O Port: 0x3F8 (COM1)
 */

use core::arch::asm;

/// Writes a single byte to the COM1 serial port.
fn serial_write_byte(data: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") 0x3F8_u16,
            in("al") data,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Broadcasts a string message to the serial debugger.
pub fn speak(message: &str) {
    for byte in message.bytes() {
        serial_write_byte(byte);
    }
}

/// Converts and prints a 64-bit integer as a decimal string.
pub fn speak_u64(mut num: u64) {
    if num == 0 {
        serial_write_byte(b'0');
        return;
    }
    let mut buffer = [0u8; 20];
    let mut i = 0;
    while num > 0 {
        buffer[i] = (num % 10) as u8 + b'0';
        num /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        serial_write_byte(buffer[i]);
    }
}

/// Converts and prints a 64-bit integer as a hexadecimal string.
pub fn speak_hex(mut num: u64) {
    if num == 0 {
        speak("0");
        return;
    }
    let hex_chars = b"0123456789ABCDEF";
    let mut buffer = [0u8; 16];
    let mut i = 0;
    while num > 0 {
        buffer[i] = hex_chars[(num % 16) as usize];
        num /= 16;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        serial_write_byte(buffer[i]);
    }
}
