$ErrorActionPreference = "Stop"

# ==============================================================================
# Stardust OS Build and Launch Pipeline
#
# Orchestrates the compilation of the kernel (Ring 0), userland Supervisor (Ring 3),
# native drivers, and WebAssembly applications. Packages the filesystem into an InitRAMFS
# and provisions the EFI System Partition (ESP) for QEMU deployment.
# ==============================================================================

Write-Host "[*] Debug Layer Active. Starting Build Pipeline..." -ForegroundColor Cyan

# Phase 0: Pre-build source sanitization
# Locates and masks repetitive debug output regarding zero-copy structures from the kernel source tree.
Write-Host "[0/8] Silencing verbose debug telemetry..." -ForegroundColor Yellow
Get-ChildItem -Path "stardust/src" -Recurse -Filter "*.rs" | ForEach-Object {
    $lines = Get-Content $_.FullName
    $changed = $false
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match "Zero-Copy") {
            $lines[$i] = "// Silenced spam"
            $changed = $true
        }
    }
    if ($changed) { $lines | Set-Content $_.FullName }
}

# Phase 1: Artifact cleanup
Write-Host "[1/8] Purging legacy build artifacts..." -ForegroundColor Yellow
Remove-Item -Recurse -Force esp/kernel -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force esp/marshal -ErrorAction SilentlyContinue

# Phase 2: Userland WebAssembly Payload
Write-Host "[2/8] Compiling WebAssembly payload (test_app)..." -ForegroundColor Yellow
Push-Location userland/apps/test_app
cargo build --target wasm32-unknown-unknown --release
if ($LASTEXITCODE -ne 0) { Write-Host "[!] Wasm compilation failed." -ForegroundColor Red; exit 1 }
Pop-Location

# Phase 3: Hardware Drivers (PS/2)
Write-Host "[3/8] Compiling native driver (ps2_hid)..." -ForegroundColor Yellow
Push-Location userland/drivers/ps2_hid
cargo build --target x86_64-unknown-none --release
if ($LASTEXITCODE -ne 0) { Write-Host "[!] PS/2 driver compilation failed." -ForegroundColor Red; exit 1 }
Pop-Location

# Phase 4: Hardware Drivers (GPU)
Write-Host "[4/8] Compiling native driver (gpu_driver)..." -ForegroundColor Yellow
Push-Location userland/drivers/gpu_driver
cargo build --target x86_64-unknown-none --release
if ($LASTEXITCODE -ne 0) { Write-Host "[!] GPU driver compilation failed." -ForegroundColor Red; exit 1 }
Pop-Location

# Phase 5: Initial RAM Filesystem (InitRAMFS) assembly
Write-Host "[5/8] Assembling InitRAMFS archive..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path userland/fs_root > $null
Copy-Item "userland/apps/test_app/target/wasm32-unknown-unknown/release/test_app.wasm" -Destination "userland/fs_root/" -Force
Copy-Item "userland/drivers/ps2_hid/target/x86_64-unknown-none/release/ps2_hid" -Destination "userland/fs_root/" -Force
Copy-Item "userland/drivers/gpu_driver/target/x86_64-unknown-none/release/gpu_driver" -Destination "userland/fs_root/" -Force
Set-Content -Path userland/fs_root/driver_config.txt -Value "driver=intel_igpu`nmode=polling"

Push-Location userland/fs_root
tar.exe -cf ../marshal/src/initramfs.tar *
Pop-Location

# Phase 6: EFI System Partition (ESP) provisioning
Write-Host "[6/8] Provisioning EFI System Partition (ESP)..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path esp/EFI/BOOT > $null
@("FS0:", "\EFI\BOOT\BOOTX64.EFI") | Set-Content -Path "esp/startup.nsh"

# Phase 7: System Supervisor (Marshal)
Write-Host "[7/8] Compiling Ring 3 Supervisor (Marshal)..." -ForegroundColor Yellow
Push-Location userland/marshal
cargo build --release
if ($LASTEXITCODE -ne 0) { Write-Host "[!] System supervisor compilation failed." -ForegroundColor Red; exit 1 }
Pop-Location
Copy-Item "userland/marshal/target/x86_64-unknown-none/release/stardust-marshal" -Destination "esp/marshal" -Force

# Phase 8: Core Kernel Compilation
Write-Host "[8/8] Compiling Ring 0 Kernel (Stardust)..." -ForegroundColor Yellow
Push-Location stardust
cargo build
if ($LASTEXITCODE -ne 0) { Write-Host "[!] Kernel compilation failed." -ForegroundColor Red; exit 1 }
Pop-Location
Copy-Item "stardust/target/x86_64-unknown-none/debug/stardust-kernel" -Destination "esp/kernel" -Force

# Pre-flight environment diagnostics
Write-Host "[*] Executing Pre-Flight Diagnostics..." -ForegroundColor Yellow
$qemu = "qemu-system-x86_64"
if (-Not (Get-Command $qemu -ErrorAction SilentlyContinue)) {
    $qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"
}

# Boot initialization
Write-Host "[*] Deployment successful. Booting Stardust OS..." -ForegroundColor Green
& $qemu -m 8G -smp 8 -cpu max -drive if=pflash,format=raw,readonly=on,file=ovmf.fd -drive format=raw,file=fat:rw:esp -serial stdio