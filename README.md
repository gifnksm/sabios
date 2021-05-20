# sabios - 錆OS

"sabios" is a Toy OS written in Rust.

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
# Boot sabios with BIOS bootloader
$ make run-bios-release

# Boot sabios with UEFI bootloader
$ make run-uefi-release
```

Other instructions are shown with `make help`.

## Requirements

Following tools are required:

* [rustup]
* [GNU Make]
* [QEMU]
* OVMF (for Arch Linux users, install [edk2-ovmf] package)
* [clang] (for compiling C++ USB drive stack)

[rustup]: https://rustup.rs/
[GNU Make]: https://www.gnu.org/software/make/
[QEMU]: https://www.qemu.org/
[edk2-ovmf]: https://archlinux.org/packages/extra/any/edk2-ovmf/
[clang]: https://clang.llvm.org/

[Makefile] assumes that OVMF is installed in `/usr/share/OVMF/x64/OVMF.fd`.
If it is installed in a different path in your environment, please specify the installation path as follow:

```console
$ make OVMF_FILE=/path/to/OVMF.fd
...
```

[Makefile]: Makefile

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
