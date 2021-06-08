# sabios - 錆OS

"sabios" is a Toy OS written in Rust.

* [My blog post series (Japanese).][my-blog]

[my-blog]: http://gifnksm.hatenablog.jp/archive/category/%E3%82%BC%E3%83%AD%E3%81%8B%E3%82%89%E3%81%AEOS%E8%87%AA%E4%BD%9C%E5%85%A5%E9%96%80

Inspired by following great pioneers:

* [Writing an OS in Rust][blog-os] by [@phil-opp]
* [ゼロからのOS自作入門][zero-os] and [MikanOS] by [@uchan-nos]

[blog-os]: https://os.phil-opp.com/
[@phil-opp]: https://github.com/phil-opp
[zero-os]: https://zero.osdev.jp/
[MikanOS]: https://github.com/uchan-nos/mikanos
[@uchan-nos]: https://github.com/uchan-nos

## Instructions

```console
# Boot sabios with UEFI bootloader
$ cargo krun --release
```

## Requirements

Following tools are required:

* [rustup]
* [QEMU]
* OVMF (for Arch Linux users, install [edk2-ovmf] package)
* [clang] (for compiling C++ USB drive stack)

[rustup]: https://rustup.rs/
[QEMU]: https://www.qemu.org/
[edk2-ovmf]: https://archlinux.org/packages/extra/any/edk2-ovmf/
[clang]: https://clang.llvm.org/

[`boot` crate] assumes that OVMF is installed in `/usr/share/OVMF/x64/OVMF.fd`.

[`boot` crate]: boot

## References

* [Microsoft Extensible Firmware Initiative FAT32 File System Specification][FAT32]
  * [Japanese Translation][FAT32-JA]

[FAT32]: https://download.microsoft.com/download/1/6/1/161ba512-40e2-4cc9-843a-923143f3456c/fatgen103.doc
[FAT32-JA]: https://docs.google.com/document/d/1ba8Jyfm4GmNgADNscqOgS9gZ9CPUgvCZSt1xRDuFV24/edit?usp=sharing

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE] or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license
  ([LICENSE-MIT] or <http://opensource.org/licenses/MIT>)

[LICENSE-APACHE]: LICENSE-APACHE
[LICENSE-MIT]: LICENSE-MIT

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
