use flate2::read::GzDecoder;
use std::{env, fs, os::unix, path::PathBuf};
use tar::Archive as TarArchive;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

async fn build_lib() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let unpacked_dir = out_dir.join("x86_64-elf");

    if unpacked_dir.exists() {
        fs::remove_dir_all(&unpacked_dir)?;
    }

    let resp = reqwest::get(
        "https://github.com/uchan-nos/mikanos-build/releases/download/v2.0/x86_64-elf.tar.gz",
    )
    .await?
    .bytes()
    .await?;

    let tar = GzDecoder::new(&*resp);
    let mut archive = TarArchive::new(tar);
    archive.unpack(&out_dir)?;

    env::set_var("CC", "clang");
    env::set_var("CXX", "clang++");

    let files = glob::glob("./cxx_src/**/*.cpp")?.collect::<std::result::Result<Vec<_>, _>>()?;

    cc::Build::new()
        .cpp(true)
        .include(unpacked_dir.join("include"))
        .include(unpacked_dir.join("include/c++/v1"))
        .include("./cxx_src/")
        .files(files)
        .define("__ELF__", None)
        .define("_LDBL_EQ_DBL", None)
        .define("_GNU_SOURCE", None)
        .define("_POSIX_TIMERS", None)
        .flag("-nostdlibinc")
        .flag("-ffreestanding")
        .flag("-mno-red-zone")
        .flag("-fno-exceptions")
        .flag("-fno-rtti")
        .flag("-std=c++17")
        .extra_warnings(false)
        .cpp_link_stdlib(None)
        .target("x86_64-elf")
        .compile("mikanos_usb");

    for lib in &["c", "c++", "c++abi"] {
        let filename = format!("lib{}.a", lib);
        let dest = out_dir.join(&filename);
        let src = unpacked_dir.join(format!("lib/{}", filename));
        if dest.exists() {
            fs::remove_file(&dest)?;
        }
        unix::fs::symlink(&src, &dest)?;
        println!("cargo:rustc-link-lib=static={}", lib);
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    build_lib().await.unwrap();
}
