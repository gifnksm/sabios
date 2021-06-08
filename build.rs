use color_eyre::eyre::{eyre, Result};
use fatfs::{FileSystem, FormatVolumeOptions, FsOptions};
use fscommon::BufStream;
use llvm_tools::LlvmTools;
use std::{
    env,
    fs::{File, OpenOptions},
    io::{prelude::*, BufReader, BufWriter},
    path::Path,
    process::Command,
};

fn build_ascii_font() -> Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();

    let input_path = Path::new("assets/ascii_font.txt");
    let output_path = Path::new(&out_dir).join("ascii_font.rs");

    println!("cargo:rerun-if-changed={}", input_path.display());

    let input = File::open(input_path)?;
    let input = BufReader::new(input);
    let output = File::create(output_path)?;
    let mut output = BufWriter::new(output);

    writeln!(
        &mut output,
        "pub(crate) const ASCII_FONT: [[u8; 16]; 256] = ["
    )?;

    let mut last_index = None;
    let mut lines = input.lines();
    while let Some(line) = lines.next() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("0x") {
            let (index_str, ch_str) = rest.split_at(2);
            let index = usize::from_str_radix(index_str, 16).unwrap();
            assert!(index == 0 || Some(index - 1) == last_index);
            last_index = Some(index);

            writeln!(&mut output, "    // 0x{}{}", index_str, ch_str)?;
            writeln!(&mut output, "    [")?;
            for line in lines.by_ref() {
                let line = line?;
                let line = line.trim();
                if !line.starts_with(&['.', '@'][..]) {
                    break;
                }
                let mut output_num = 0;
                for ch in line.chars() {
                    let bit = match ch {
                        '.' => 0,
                        '@' => 1,
                        _ => panic!("invalid char: {:?}", ch),
                    };
                    output_num = (output_num << 1) | bit;
                }
                writeln!(&mut output, "        0b{:08b},", output_num)?;
            }
            writeln!(&mut output, "    ],")?;
        }
    }

    writeln!(&mut output, "];")?;

    Ok(())
}

fn build_fs() -> Result<()> {
    let llvm_tools = LlvmTools::new().map_err(|err| eyre!("{:?}", err))?;
    let objcopy = llvm_tools
        .tool(&llvm_tools::exe("llvm-objcopy"))
        .ok_or_else(|| eyre!("llvm0objcopy not found"))?;
    let ar = llvm_tools
        .tool(&llvm_tools::exe("llvm-ar"))
        .ok_or_else(|| eyre!("llvm-ar not found"))?;

    let fs_size = 16 * 1024 * 1024; // 16MiB
    let out_dir = env::var("OUT_DIR")?;
    let fat_path = Path::new(&out_dir).join("fs.fat");
    let obj_path = Path::new(&out_dir).join("fs.o");
    let lib_path = Path::new(&out_dir).join("libfs.a");
    let fat_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&fat_path)?;
    fat_file.set_len(fs_size)?;
    let mut fat_file = BufStream::new(fat_file);

    // create new FAT partition
    let format_options = FormatVolumeOptions::new().volume_label(*b"sabios     ");
    fatfs::format_volume(&mut fat_file, format_options)?;

    // copy files to FAT filesystem
    let partition = FileSystem::new(&mut fat_file, FsOptions::new())?;
    let root_dir = partition.root_dir();
    root_dir.create_dir("bin")?;
    let mut sabios = root_dir.create_file("sabios.txt")?;
    sabios.truncate()?;
    writeln!(&mut sabios, "hello sabios!")?;

    // create object file
    let mut objcopy_cmd = Command::new(objcopy);
    objcopy_cmd
        .current_dir(&out_dir)
        .arg("-I")
        .arg("binary")
        .arg("-O")
        .arg("elf64-x86-64")
        .arg("-B")
        .arg("i386:x86-64")
        .arg(fat_path.file_name().unwrap())
        .arg(obj_path.file_name().unwrap());
    let objcopy_status = objcopy_cmd.status()?;
    assert!(objcopy_status.success(), "objcopy failed");

    let mut ar_cmd = Command::new(ar);
    ar_cmd
        .current_dir(&out_dir)
        .arg("crus")
        .arg(lib_path)
        .arg(obj_path);
    let ar_status = ar_cmd.status()?;
    assert!(ar_status.success(), "ar failed");

    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=fs");

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    build_ascii_font()?;
    build_fs()?;
    Ok(())
}
