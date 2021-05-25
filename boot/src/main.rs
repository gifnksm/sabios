use bootloader_locator::locate_bootloader;
use locate_cargo_manifest::locate_manifest;
use std::{
    env,
    path::{Path, PathBuf},
    process::{self, Command},
};

const RUN_ARGS: &[&str] = &[
    "-m",
    "1G",
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

const OVMF_PATH: &str = "/usr/share/OVMF/x64/OVMF.fd";

fn main() {
    let mut args = env::args().skip(1); // skip executable name

    let kernel_binary_path = {
        let path = PathBuf::from(args.next().unwrap());
        path.canonicalize().unwrap()
    };
    let no_boot = if let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-run" => true,
            other => panic!("unexpected argument `{}`", other),
        }
    } else {
        false
    };

    println!("use kernel executable: {}", kernel_binary_path.display());
    let image = create_disk_image(&kernel_binary_path);

    if no_boot {
        println!("Created disk image at `{}`", image.display());
        return;
    }

    let mut run_cmd = Command::new("qemu-system-x86_64");
    run_cmd
        .arg("-drive")
        .arg(format!("format=raw,file={}", image.display()))
        .arg("-bios")
        .arg(OVMF_PATH);
    run_cmd.args(RUN_ARGS);

    let exit_status = run_cmd.status().unwrap();
    if !exit_status.success() {
        process::exit(exit_status.code().unwrap_or(1));
    }
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
