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
DEBUG_BIOS_IMAGE := target/$(TARGET)/debug/boot-bios-sabios.img
DEBUG_UEFI_IMAGE := target/$(TARGET)/debug/boot-uefi-sabios.img
DEBUG_UEFI_EXECUTABLE := target/$(TARGET)/debug/boot-uefi-sabios.efi
DEBUG_UEFI_PARTITION := target/$(TARGET)/debug/boot-uefi-sabios.fat

OVMF_FILE := /usr/share/OVMF/x64/OVMF.fd

DEBUG_KERNEL := target/$(TARGET)/debug/sabios

# targets
run-bios: debug-qemu-bios ## Run sabios BIOS disk image with QEMU
.PHONY: run-bios

run-uefi: debug-qemu-uefi ## Run sabios UEFI disk image with QEMU
.PHONY: run-uefi

build: ## Build all artifacts (debug build)
.PHONY: build

build-image-bios: $(DEBUG_BIOS_IMAGE) ## Build saibos BIOS disk image (debug build)
.PHONY: build-image-bios
build: build-image-bios

build-image-uefi: $(DEBUG_UEFI_IMAGE) ## Build sabios UEFI disk image (debug build)
.PHONY: build-image-uefi
build: build-image-uefi

build-kernel: $(DEBUG_KERNEL) ## Build sabios kernel executable (debug build)
.PHONY: build-kernel
build: build-kernel

help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "} {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
.PHONY: help

# commands
debug-qemu-bios: $(DEBUG_BIOS_IMAGE)
	$(QEMU) \
	    -drive format=raw,file=$<

debug-qemu-uefi: $(DEBUG_UEFI_IMAGE) | $(OVMF_FILE)
	$(QEMU) \
	    -drive format=raw,file=$< \
	    -bios $(OVMF_FILE)

debug-cargo-run-boot: build-kernel
	$(CARGO) run -p boot
.PHONY: debug-cargo-run-boot

debug-cargo-build-sabios:
	$(CARGO) build \
	    -p sabios \
	    --target $(TARGET).json \
	    -Z build-std=core \
	    -Z build-std-features=compiler-builtins-mem
.PHONY: debug-cargo-build-sabios

# files
$(DEBUG_BIOS_IMAGE): debug-cargo-run-boot
$(DEBUG_UEFI_IMAGE): debug-cargo-run-boot
$(DEBUG_UEFI_EXECUTABLE): debug-cargo-run-boot
$(DEBUG_UEFI_PARTITION): debug-cargo-run-boot

$(DEBUG_KERNEL): debug-cargo-build-sabios

