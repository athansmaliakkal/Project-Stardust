import os
import sys
import subprocess
import shutil
import urllib.request
import tarfile
import platform
import stat

# ==============================================================================
# STARDUST OS - MASTER ORCHESTRATOR
# A cross-platform, dependency-isolated Meta-Build System.
# ==============================================================================

# Ensure execution from the Project Root
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, ".."))
os.chdir(PROJECT_ROOT)

ENV_FILE = os.path.join(SCRIPT_DIR, ".env")
BOOTLOADER_DIR = os.path.join(SCRIPT_DIR, "bootloader")
COMPILER_DIR = os.path.join(SCRIPT_DIR, "compiler")
CARGO_HOME = os.path.join(COMPILER_DIR, "cargo")
RUSTUP_HOME = os.path.join(COMPILER_DIR, "rustup")

LIMINE_URL = "https://raw.githubusercontent.com/limine-bootloader/limine/v8.x-binary/BOOTX64.EFI"

def is_admin():
    if os.name == 'nt':
        try:
            import ctypes
            return ctypes.windll.shell32.IsUserAnAdmin() != 0
        except Exception:
            return False
    else:
        return os.getuid() == 0

def load_env():
    env = {"QEMU_PATH": "qemu-system-x86_64", "SETUP_COMPLETE": "False"}
    if os.path.exists(ENV_FILE):
        with open(ENV_FILE, "r") as f:
            for line in f:
                if "=" in line:
                    k, v = line.strip().split("=", 1)
                    env[k] = v
    return env

def save_env(env):
    with open(ENV_FILE, "w") as f:
        for k, v in env.items():
            f.write(f"{k}={v}\n")

def get_rust_env():
    """Injects our isolated compiler paths so we don't touch the user's system."""
    env = os.environ.copy()
    env["CARGO_HOME"] = CARGO_HOME
    env["RUSTUP_HOME"] = RUSTUP_HOME
    cargo_bin = os.path.join(CARGO_HOME, "bin")
    env["PATH"] = f"{cargo_bin}{os.pathsep}{env.get('PATH', '')}"
    return env

def get_cargo_bin():
    exe = "cargo.exe" if os.name == "nt" else "cargo"
    return os.path.join(CARGO_HOME, "bin", exe)

def clean_directory():
    print("\n[*] Initiating Deep Clean...")
    dirs_to_nuke = ["Platform", "Userland/fs_root"]
    files_to_nuke = ["Userland/marshal/src/initramfs.tar"]
    
    for d in dirs_to_nuke:
        if os.path.exists(d):
            shutil.rmtree(d, ignore_errors=True)
            print(f"  [-] Removed {d}/")
            
    for f in files_to_nuke:
        if os.path.exists(f):
            os.remove(f)
            print(f"  [-] Removed {f}")

    # Recursively hunt down build artifacts EXCEPT in Orchestrator/compiler
    for root, dirs, files in os.walk(PROJECT_ROOT):
        if "Orchestrator" in root:
            continue
        if "target" in dirs:
            shutil.rmtree(os.path.join(root, "target"), ignore_errors=True)
            print(f"  [-] Purged target in {root}")
        if "Cargo.lock" in files:
            os.remove(os.path.join(root, "Cargo.lock"))
            print(f"  [-] Purged Cargo.lock in {root}")
            
    print("[+] Clean Complete. (Isolated compiler preserved)")

def setup_environment():
    print("\n[*] Verifying Stardust Isolated Environment...")
    env = load_env()
    os.makedirs(COMPILER_DIR, exist_ok=True)
    
    cargo_path = get_cargo_bin()
    if not os.path.exists(cargo_path):
        print("  [*] Local compiler not found. Downloading isolated Rust Toolchain...")
        sys_os = platform.system()
        rustup_init = os.path.join(COMPILER_DIR, "rustup-init.exe" if sys_os == "Windows" else "rustup-init")
        
        if sys_os == "Windows":
            url = "https://win.rustup.rs/x86_64"
        elif sys_os == "Linux":
            url = "https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"
        elif sys_os == "Darwin":
            if platform.machine() == "arm64":
                url = "https://static.rust-lang.org/rustup/dist/aarch64-apple-darwin/rustup-init"
            else:
                url = "https://static.rust-lang.org/rustup/dist/x86_64-apple-darwin/rustup-init"
        else:
            print("  [!] Unsupported OS for automated Rust installation.")
            return

        urllib.request.urlretrieve(url, rustup_init)
        if sys_os != "Windows":
            os.chmod(rustup_init, os.stat(rustup_init).st_mode | stat.S_IEXEC)
            
        print("  [*] Installing Rust Nightly silently...")
        subprocess.run([rustup_init, "-y", "--default-toolchain", "nightly"], env=get_rust_env(), check=True)
        print("  [+] Isolated Compiler Installed.")
    else:
        print("  [+] Isolated Compiler found.")

    rustup_bin = os.path.join(CARGO_HOME, "bin", "rustup.exe" if os.name == "nt" else "rustup")
    print("  [*] Adding bare-metal and WebAssembly targets...")
    subprocess.run([rustup_bin, "target", "add", "wasm32-unknown-unknown", "x86_64-unknown-none"], env=get_rust_env(), check=False, stdout=subprocess.DEVNULL)
    subprocess.run([rustup_bin, "component", "add", "rust-src"], env=get_rust_env(), check=False, stdout=subprocess.DEVNULL)

    os.makedirs(BOOTLOADER_DIR, exist_ok=True)
    bootloader_path = os.path.join(BOOTLOADER_DIR, "BOOTX64.EFI")
    if not os.path.exists(bootloader_path):
        print("  [*] Downloading Limine Bootloader...")
        try:
            urllib.request.urlretrieve(LIMINE_URL, bootloader_path)
            print("  [+] Bootloader cached successfully.")
        except Exception as e:
            print(f"  [!] Failed to download Limine: {e}")
            return
    else:
        print("  [+] Bootloader already cached.")

    qemu = env.get("QEMU_PATH", "qemu-system-x86_64")
    if shutil.which(qemu) is None:
        if platform.system() == "Windows" and os.path.exists(r"C:\Program Files\qemu\qemu-system-x86_64.exe"):
            env["QEMU_PATH"] = r"C:\Program Files\qemu\qemu-system-x86_64.exe"
            print("  [+] Found QEMU in Program Files.")
        else:
            print(f"  [!] QEMU not found. Please install QEMU or update Orchestrator/.env")
            return
    else:
        print(f"  [+] QEMU found ({qemu}).")

    env["SETUP_COMPLETE"] = "True"
    save_env(env)
    print("[+] Environment Setup Complete.")

def run_system():
    env = load_env()
    if env.get("SETUP_COMPLETE") != "True":
        print("\n[!] Essential components are missing.")
        print("[!] Please run Option 2 (Install / Verify Build Environment) first.")
        return

    print("\n[*] Stardust OS Build Pipeline Active...")
    cargo_bin = get_cargo_bin()
    rust_env = get_rust_env()

    print("[0/6] Silencing Kernel Spam...")
    src_dir = "Stardust/src"
    if os.path.exists(src_dir):
        for root, _, files in os.walk(src_dir):
            for file in files:
                if file.endswith(".rs"):
                    fpath = os.path.join(root, file)
                    with open(fpath, "r", encoding='utf-8') as f:
                        lines = f.readlines()
                    changed = False
                    for i, line in enumerate(lines):
                        if "Zero-Copy" in line and not line.strip().startswith("//"):
                            lines[i] = "// " + line
                            changed = True
                    if changed:
                        with open(fpath, "w", encoding='utf-8') as f:
                            f.writelines(lines)

    os.makedirs("Platform/EFI/BOOT", exist_ok=True)
    os.makedirs("Userland/fs_root", exist_ok=True)
    
    shutil.copy(os.path.join(BOOTLOADER_DIR, "BOOTX64.EFI"), "Platform/EFI/BOOT/BOOTX64.EFI")
    
    with open("Platform/startup.nsh", "w", newline="\n") as f:
        f.write("FS0:\n\\EFI\\BOOT\\BOOTX64.EFI\n")
        
    limine_conf = (
        "timeout: 3\n"
        "\n"
        "/Stardust OS (x86_64)\n"
        "    protocol: limine\n"
        "    kernel_path: boot():/kernel\n"
        "    module_path: boot():/marshal\n"
    )
    
    with open("Platform/limine.conf", "w", newline="\n") as f:
        f.write(limine_conf)
    with open("Platform/EFI/BOOT/limine.conf", "w", newline="\n") as f:
        f.write(limine_conf)

    init_rc_lines = []

    print("[1/6] Orchestrating WebAssembly Apps...")
    apps_dir = "Userland/apps"
    if os.path.exists(apps_dir):
        for app in os.listdir(apps_dir):
            app_path = os.path.join(apps_dir, app)
            if os.path.isdir(app_path):
                print(f"  -> Building App: {app}")
                res = subprocess.run([cargo_bin, "build", "--target", "wasm32-unknown-unknown", "--release"], cwd=app_path, env=rust_env)
                if res.returncode != 0: sys.exit(f"[!] Build failed for {app}")
                shutil.copy(os.path.join(app_path, f"target/wasm32-unknown-unknown/release/{app}.wasm"), "Userland/fs_root/")
                init_rc_lines.append(f"app:{app}.wasm\n")

    print("[2/6] Orchestrating Native Drivers...")
    drv_dir = "Userland/drivers"
    if os.path.exists(drv_dir):
        for drv in os.listdir(drv_dir):
            drv_path = os.path.join(drv_dir, drv)
            if os.path.isdir(drv_path):
                print(f"  -> Building Driver: {drv}")
                res = subprocess.run([cargo_bin, "build", "--target", "x86_64-unknown-none", "--release"], cwd=drv_path, env=rust_env)
                if res.returncode != 0: sys.exit(f"[!] Build failed for {drv}")
                shutil.copy(os.path.join(drv_path, f"target/x86_64-unknown-none/release/{drv}"), "Userland/fs_root/")
                init_rc_lines.append(f"driver:{drv}\n")

    with open("Userland/fs_root/init.rc", "w", newline="\n") as f:
        f.writelines(init_rc_lines)

    print("[3/6] Packaging InitRAMFS natively...")
    with tarfile.open("Userland/marshal/src/initramfs.tar", "w") as tar:
        for item in os.listdir("Userland/fs_root"):
            tar.add(os.path.join("Userland/fs_root", item), arcname=item)

    print("[4/6] Compiling Ring 3 Supervisor (Marshal)...")
    res = subprocess.run([cargo_bin, "build", "--release"], cwd="Userland/marshal", env=rust_env)
    if res.returncode != 0: sys.exit("[!] Marshal Build Failed")
    shutil.copy("Userland/marshal/target/x86_64-unknown-none/release/stardust-marshal", "Platform/marshal")

    print("[5/6] Compiling Ring 0 Kernel (Stardust)...")
    res = subprocess.run([cargo_bin, "build"], cwd="Stardust", env=rust_env)
    if res.returncode != 0: sys.exit("[!] Kernel Build Failed")
    shutil.copy("Stardust/target/x86_64-unknown-none/debug/stardust-kernel", "Platform/kernel")

    print("[6/6] All Systems Green. Igniting World Engine...")
    
    # FIX: Absolute paths to prevent relative path resolution failures in QEMU!
    ovmf_path = os.path.join(PROJECT_ROOT, "ovmf.fd")
    platform_path = os.path.join(PROJECT_ROOT, "Platform")
    
    qemu_cmd = [
        env["QEMU_PATH"], "-m", "8G", "-smp", "8", "-cpu", "max",
        "-no-reboot", 
        "-drive", f"if=pflash,format=raw,readonly=on,file={ovmf_path}",
        "-drive", f"format=raw,file=fat:rw:{platform_path}", 
        "-serial", "stdio"
    ]
    
    try:
        subprocess.run(qemu_cmd)
    except KeyboardInterrupt:
        print("\n[*] Engine shutdown requested. Terminating Stardust.")

if __name__ == "__main__":
    if not is_admin():
        print("\n[!] Notice: Administrator/Root privileges are highly recommended.")
        print("[!] Please restart your terminal with elevated permissions if build fails.")
        print("-" * 60)

    while True:
        print("\n==============================================")
        print("  STARDUST OS ORCHESTRATOR")
        print("==============================================")
        print("[1] Run Stardust OS (Build & Emulate) [Default]")
        print("[2] Install / Verify Build Environment")
        print("[3] Clean Build Artifacts (Deep Clean)")
        print("[4] Exit")
        
        try:
            choice = input("\nSelect an option [1]: ").strip()
        except KeyboardInterrupt:
            print("\n[*] Exiting Orchestrator.")
            sys.exit(0)

        if not choice:
            choice = '1'

        if choice == '1':
            run_system()
            break
        elif choice == '2':
            setup_environment()
        elif choice == '3':
            clean_directory()
        elif choice == '4':
            sys.exit(0)
        else:
            print("Invalid selection.")