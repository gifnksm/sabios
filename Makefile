# misc configuration
SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
.DEFAULT_GOAL := help
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules


# goals declaration
run-bios-debug:       ## Run sabios BIOS disk image with QEMU (debug build)
run-bios-release:     ## Run sabios BIOS disk image with QEMU (release build)
run-uefi-debug:       ## Run sabios UEFI disk image with QEMU (debug build)
run-uefi-release:     ## Run sabios UEFI disk image with QEMU (release build)
build-debug:          ## Build all artifacts (debug build)
build-release:        ## Build all artifacts (release build)
build-image-debug:    ## Build sabios BIOS/UEFI disk image (debug build)
build-image-release:  ## Build saibos BIOS/UEFI disk image (release build)
build-kernel-debug:   ## Build sabios kernel executable (debug build)
build-kernel-release: ## Build sabios kernel executable (release build)
clean:                ## Clean build directory
help:                 ## Show this help message


# commands
CARGO := cargo
QEMU  := qemu-system-x86_64

# configs
TARGET := x86_64-sabios
OVMF_FILE := /usr/share/OVMF/x64/OVMF.fd

# artifacts
BIOS_IMAGE_DEBUG        := target/$(TARGET)/debug/boot-bios-sabios.img
BIOS_IMAGE_RELEASE      := target/$(TARGET)/release/boot-bios-sabios.img

UEFI_IMAGE_DEBUG        := target/$(TARGET)/debug/boot-uefi-sabios.img
UEFI_IMAGE_RELEASE      := target/$(TARGET)/release/boot-uefi-sabios.img
UEFI_EXECUTABLE_DEBUG   := target/$(TARGET)/debug/boot-uefi-sabios.efi
UEFI_EXECUTABLE_RELEASE := target/$(TARGET)/release/boot-uefi-sabios.efi
UEFI_PARTITION_DEBUG    := target/$(TARGET)/debug/boot-uefi-sabios.fat
UEFI_PARTITION_RELEASE  := target/$(TARGET)/release/boot-uefi-sabios.fat

KERNEL_DEBUG   := target/$(TARGET)/debug/sabios
KERNEL_RELEASE := target/$(TARGET)/release/sabios


# goald definition
run-bios-debug:   qemu-bios-debug
run-bios-release: qemu-bios-release
.PHONY: run-bios-debug run-bios-release

run-uefi-debug:   qemu-uefi-debug
run-uefi-release: qemu-uefi-release
.PHONY: run-uefi-debug run-uefi-release

build-debug:   build-image-debug   build-kernel-debug
build-release: build-image-release build-image-release
.PHONY: build-debug build-release

build-image-debug:   $(BIOS_IMAGE_DEBUG)   $(UEFI_IMAGE_DEBUG)
build-image-release: $(BIOS_IMAGE_RELEASE) $(UEFI_IMAGE_RELEASE)
.PHONY: build-image-debug
.PHONY: build-image-elease

build-kernel-debug:   $(KERNEL_DEBUG)
build-kernel-release: $(KERNEL_RELEASE)
.PHONY: build-kernel-debug
.PHONY: build-kernel-release

clean: cargo-clean
.PHONY: clean

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "} {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
.PHONY: help

# commands
QEMU_OPTS =
ifdef QEMU_DEBUG
    QEMU_OPTS += -d int --no-reboot --no-shutdown
endif

qemu-bios-debug:   BIOS_IMAGE:=$(BIOS_IMAGE_DEBUG)
qemu-bios-release: BIOS_IMAGE:=$(BIOS_IMAGE_RELEASE)
qemu-bios-%: cargo-run-boot-%
	$(QEMU) -drive format=raw,file=$(BIOS_IMAGE) $(QEMU_OPTS)

qemu-uefi-debug:   UEFI_IMAGE:=$(UEFI_IMAGE_DEBUG)
qemu-uefi-release: UEFI_IMAGE:=$(UEFI_IMAGE_RELEASE)
qemu-uefi-%: cargo-run-boot-% | $(OVMF_FILE)
	$(QEMU) -drive format=raw,file=$(UEFI_IMAGE) -bios $(OVMF_FILE) $(QEMU_OPTS)

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

cargo-clean:
	cargo clean
.PHONY: cargo-clean

# files
$(BIOS_IMAGE_DEBUG):        cargo-run-boot-debug
$(BIOS_IMAGE_RELEASE):      cargo-run-boot-release
$(UEFI_IMAGE_DEBUG):        cargo-run-boot-debug
$(UEFI_IMAGE_RELEASE):      cargo-run-boot-release
$(UEFI_EXECUTABLE_DEBUG):   cargo-run-boot-debug
$(UEFI_EXECUTABLE_RELEASE): cargo-run-boot-release
$(UEFI_PARTITION_DEBUG):    cargo-run-boot-debug
$(UEFI_PARTITION_RELEASE):  cargo-run-boot-release

$(KERNEL_DEBUG):   cargo-build-sabios-debug
$(KERNEL_RELEASE): cargo-build-sabios-release

