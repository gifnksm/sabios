use bootloader_locator::locate_bootloader;
use locate_cargo_manifest::locate_manifest;
use std::{
    env,
    path::{Path, PathBuf},
    process::{self, Command, ExitStatus},
    time::Duration,
};

const RUN_ARGS: &[&str] = &[
    "-m",
    "1G",
    "-serial",
    "stdio",
    "-device",
    "nec-usb-xhci,id=xhci",
    "-device",
    "usb-mouse",
    "-device",
    "usb-kbd",
    "-gdb",
    "tcp::1234",
    "-no-reboot",
];
const TEST_ARGS: &[&str] = &[
    "-m",
    "1G",
    "-serial",
    "stdio",
    "-device",
    "nec-usb-xhci,id=xhci",
    "-device",
    "usb-mouse",
    "-device",
    "usb-kbd",
    "-gdb",
    "tcp::1234",
    "-device",
    "isa-debug-exit,iobase=0xf4,iosize=0x04",
    "-display",
    "none",
    "-no-reboot",
];
const TEST_TIMEOUT_SECS: u64 = 30;
const OVMF_PATH: &str = "/usr/share/OVMF/x64/OVMF.fd";

fn main() {
    let mut args = env::args().skip(1); // skip executable name

    let kernel_binary_path = {
        let path = PathBuf::from(args.next().unwrap());
        path.canonicalize().unwrap()
    };

    println!("use kernel executable: {}", kernel_binary_path.display());
    let image = create_disk_image(&kernel_binary_path);

    let mut run_cmd = Command::new("qemu-system-x86_64");
    run_cmd
        .arg("-drive")
        .arg(format!("format=raw,file={}", image.display()))
        .arg("-bios")
        .arg(OVMF_PATH);

    let binary_kind = runner_utils::binary_kind(&kernel_binary_path);
    if binary_kind.is_test() {
        run_cmd.args(TEST_ARGS);

        let exit_status = run_test_command(run_cmd);
        match exit_status.code() {
            Some(33) => {} // success
            other => panic!("Test failed (exit code: {:?})", other),
        }
    } else {
        run_cmd.args(RUN_ARGS);
        let exit_status = run_cmd.status().unwrap();
        if !exit_status.success() {
            process::exit(exit_status.code().unwrap_or(1));
        }
    }
}

fn run_test_command(mut cmd: Command) -> ExitStatus {
    runner_utils::run_with_timeout(&mut cmd, Duration::from_secs(TEST_TIMEOUT_SECS)).unwrap()
}

fn create_disk_image(kernel_binary_path: &Path) -> PathBuf {
    let bootloader_manifest_path = locate_bootloader("bootloader").unwrap();
    let kernel_manifest_path = locate_manifest().unwrap();

    let mut build_cmd = Command::new(env!("CARGO"));
    build_cmd.current_dir(bootloader_manifest_path.parent().unwrap());
    build_cmd.arg("builder");
    build_cmd.arg("--firmware").arg("uefi");
    build_cmd
        .arg("--kernel-manifest")
        .arg(&kernel_manifest_path);
    build_cmd.arg("--kernel-binary").arg(&kernel_binary_path);
    build_cmd
        .arg("--target-dir")
        .arg(kernel_manifest_path.parent().unwrap().join("target"));
    build_cmd
        .arg("--out-dir")
        .arg(kernel_binary_path.parent().unwrap());

    if !build_cmd.status().unwrap().success() {
        panic!("bootloader build failed");
    }

    let kernel_binary_name = kernel_binary_path.file_name().unwrap().to_str().unwrap();
    let disk_image = kernel_binary_path
        .parent()
        .unwrap()
        .join(format!("boot-uefi-{}.img", kernel_binary_name));
    if !disk_image.exists() {
        panic!(
            "Disk image does not exist at {} after bootloader build",
            disk_image.display()
        );
    }
    disk_image
}
