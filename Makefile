SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
.DEFAULT_GOAL := help
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules

# commands
CARGO := cargo
QEMU  := qemu-system-x86_64

# configs
TARGET := x86_64-sabios

# artifacts
OVMF_FILE := /usr/share/OVMF/x64/OVMF.fd

BIOS_IMAGE_DEBUG        := target/$(TARGET)/debug/boot-bios-sabios.img
BIOS_IMAGE_RELEASE      := target/$(TARGET)/release/boot-bios-sabios.img
UEFI_IMAGE_DEBUG        := target/$(TARGET)/debug/boot-uefi-sabios.img
UEFI_IMAGE_RELEASE      := target/$(TARGET)/release/boot-uefi-sabios.img
UEFI_EXECUTABLE_DEBUG   := target/$(TARGET)/debug/boot-uefi-sabios.efi
UEFI_EXECUTABLE_RELEASE := target/$(TARGET)/release/boot-uefi-sabios.efi
UEFI_PARTITION_DEBUG    := target/$(TARGET)/debug/boot-uefi-sabios.fat
UEFI_PARTITION_RELEASE  := target/$(TARGET)/release/boot-uefi-sabios.fat

KERNEL_DEBUG := target/$(TARGET)/debug/sabios
KERNEL_RELEASE := target/$(TARGET)/release/sabios

# targets
run-bios: qemu-bios-debug ## Run sabios BIOS disk image with QEMU (debug build)
.PHONY: run-bios

run-bios-release: qemu-bios-release ## Run sabios BIOS disk image with QEMU (release build)
.PHONY: run-bios-release

run-uefi-debug: qemu-uefi-debug ## Run sabios UEFI disk image with QEMU (debug build)
.PHONY: run-uefi

run-uefi-release: qemu-uefi-release ## Run sabios UEFI disk image with QEMU (release build)
.PHONY: run-uefi-release

build-debug: ## Build all artifacts (debug build)
.PHONY: build-debug

build-release: ## Build all artifacts (release build)
.PHONY: build-release

build-image-bios-debug: $(BIOS_IMAGE_DEBUG) ## Build saibos BIOS disk image (debug build)
.PHONY: build-image-bios-debug
build-debug: build-image-bios-debug

build-image-bios-release: $(BIOS_IMAGE_RELEASE) ## Build saibos BIOS disk image (release build)
.PHONY: build-image-bios-release
build-release: build-image-bios-release

build-image-uefi-debug: $(UEFI_IMAGE_DEBUG) ## Build sabios UEFI disk image (debug build)
.PHONY: build-image-uefi-debug
build-debug: build-image-uefi-debug

build-image-uefi-release: $(UEFI_IMAGE_RELEASE) ## Build sabios UEFI disk image (release build)
.PHONY: build-image-uefi-release
build-release: build-image-uefi-release

build-kernel-debug: $(KERNEL_DEBUG) ## Build sabios kernel executable (debug build)
.PHONY: build-kernel-debug
build-debug: build-kernel-debug

build-kernel-release: $(KERNEL_RELEASE) ## Build sabios kernel executable (release build)
.PHONY: build-kernel-release
build-release: build-kernel-release

clean: ## Clean build directory
	cargo clean
.PHONY: clean

help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "} {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
.PHONY: help

# commands
qemu-bios-debug:   BIOS_IMAGE=$(BIOS_IMAGE_DEBUG)
qemu-bios-release: BIOS_IMAGE=$(BIOS_IMAGE_RELEASE)
qemu-bios-%: cargo-run-boot-%
	$(QEMU) \
	    -drive format=raw,file=$(BIOS_IMAGE)

qemu-uefi-debug:   UEFI_IMAGE:=$(UEFI_IMAGE_DEBUG)
qemu-uefi-release: UEFI_IMAGE:=$(UEFI_IMAGE_RELEASE)
qemu-uefi-%: cargo-run-boot-% | $(OVMF_FILE)
	$(QEMU) \
	    -drive format=raw,file=$(UEFI_IMAGE) \
	    -bios $(OVMF_FILE)

%-debug:   CARGO_BUILD_MODE_OPTIONS:=
%-release: CARGO_BUILD_MODE_OPTIONS:=--release

cargo-run-boot-%: build-kernel-%
	$(CARGO) run $(CARGO_BUILD_MODE_OPTIONS) \
	    -p boot
.PHONY: cargo-run-boot-%

cargo-build-sabios-%:
	$(CARGO) build $(CARGO_BUILD_MODE_OPTIONS) \
	    -p sabios \
	    --target $(TARGET).json \
	    -Z build-std=core \
	    -Z build-std-features=compiler-builtins-mem
.PHONY: cargo-build-sabios-%


# files
$(BIOS_IMAGE_DEBUG):        cargo-run-boot-debug
$(BIOS_IMAGE_RELEASE):      cargo-run-boot-release
$(UEFI_IMAGE_DEBUG):        cargo-run-boot-debug
$(UEFI_IMAGE_RELEASE):      cargo-run-boot-release
$(UEFI_EXECUTABLE_DEBUG):   cargo-run-boot-debug
$(UEFI_EXECUTABLE_RELEASE): cargo-run-boot-release
$(UEFI_PARTITION_DEBUG):    cargo-run-boot-debug
$(UEFI_PARTITION_RELEASE):  cargo-run-boot-release

$(KERNEL_DEBUG): cargo-build-sabios-debug
$(KERNEL_RELEASE): cargo-build-sabios-release

