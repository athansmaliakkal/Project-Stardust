# Stardust OS

Stardust OS is a modern, experimental operating system kernel and userland environment built in Rust. It emphasizes a secure capability-based model, an efficient task scheduling system with multi-core support, and an advanced user-space hierarchy centered around a God-Process ("Marshal").

## Architectural Overview

Stardust is divided into a classic Ring 0 kernel (`stardust/`) and a Ring 3 userland (`userland/`).

### The Kernel (Ring 0)
- **Architecture**: x86_64-specific low-level abstraction (`src/arch/x86_64/`).
- **Task Management**: Implements rigorous threading, scheduling, context switching, and ELF loader capabilities. Transitions from Ring 0 to Ring 3 occur seamlessly via CPU exception return (`IRETQ`).
- **Memory Management**: Sophisticated physical frame allocation, paging, virtual memory mapping, and shared memory structures.
- **Security**: Utilizes a capability-based security model managed via token rights and a Capability Distribution Tree (CDT).
- **Symmetric Multiprocessing (SMP)**: Core scheduling is built to handle dispatching on multiple CPU cores.
- **HAL**: An extensible Hardware Abstraction Layer mapping device drivers and CPU initialization routines.

### Userland (Ring 3)
- **Marshal (The God-Process)**: Acts as the primary system supervisor running in user space. It unpacks the InitRAMFS and manages system services and dynamic loading.
- **Hardware Drivers**: Drivers like `gpu_driver` and `ps2_hid` operate largely isolated from the core kernel space to prevent critical panics. 
- **WebAssembly Subsystem**: Supports running portable WebAssembly (WASM) payload modules in userland (`test_app`), providing an extra boundary of process isolation and flexibility.

## Features

- **Robust Boot Pipeline**: Uses the UEFI boot mechanism via the Limine bootloader structure wrapped inside an EFI System Partition (ESP).
- **Built-in initramfs**: Packages system applications (WASM), native hardware drivers, and configuration files into an embedded tar archive dynamically constructed at build time.
- **Microkernel-like Modularity**: Maintains isolation between the core OS scheduling and actual hardware driver stacks, leaning toward userland driver support.
- **WebAssembly Engine**: First-class support for WASM modules executing securely in userland context.

## Dependencies

- **Rust Nightly**: Required for building `#![no_std]` targets, experimental allocator features, and un-stable inline assembly.
- **Cargo**: Used to compile the entire project. Included with Rust.
- **QEMU**: (`qemu-system-x86_64`) utilized as the hypervisor to simulate the environment.
- **PowerShell**: Used for cross-platform orchestration via `run.ps1`.
- **Target Toolchains**:
  - `x86_64-unknown-none` (Kernel & Drivers)
  - `wasm32-unknown-unknown` (WASM apps)

## How to Build & Run

Stardust includes a PowerShell orchestration script that safely configures the ESP, compiles the core components, layers the file system, and launches QEMU.

1. Ensure your Rust toolchain supports the required targets:
   ```bash
   rustup target add x86_64-unknown-none
   rustup target add wasm32-unknown-unknown
   ```
2. Make sure you have `qemu-system-x86_64` installed and available in your environment's PATH.
3. Execute the `run.ps1` deployment script:
   ```powershell
   ./run.ps1
   ```

The script will:
- Silence compiler warnings and build artifacts.
- Compile userland WebAssembly payload modules (`test_app`).
- Compile native drivers (`ps2_hid`, `gpu_driver`).
- Package drivers and modules into an InitRAMFS tar archive.
- Compile the Ring 3 God-Process (`stardust-marshal`).
- Compile the core kernel payload (`stardust-kernel`).
- Boot into the simulated QEMU test bed using OVMF firmware (`ovmf.fd`) in SMP configuration with 8 virtual processors and 8GB RAM.

## License & Contribution

Stardust OS is released under a **Personal Use Only** license. 

- **Personal Use**: You are free to use, modify, and study the code for personal and educational purposes.
- **No Commercial Use**: Use for business, commercial, or for-profit purposes is strictly prohibited.
- **Attribution Required**: Proper credit must be given to the original author in all copies or derivatives.
- **No Renaming**: You may not rename the OS or remove the "Stardust OS" branding from boot headers or documentation.

For the full legal text, please refer to the `LICENSE` file in the root directory.

---
*Note: This project is an experimental research kernel. Use at your own risk. Synchronization semantics (specifically spinlocks and atomics) conform to professional, architectural documentation standards typical of low-level systems programming.*
