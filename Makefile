SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules

# commands
CARGO := cargo

# configs
TARGET := x86_64-sabios

# artifacts
DEBUG_KERNEL := target/x86-64/debug/sabios

# phony targets
all: kernel
.PHONY: all

kernel: $(DEBUG_KERNEL)
.PHONY: kernel

# dummy targets
FORCE:
.PHONY: FORCE

# file targets
$(DEBUG_KERNEL): FORCE
	$(CARGO) build \
	    --target $(TARGET).json \
	    -Z build-std=core \
	    -Z build-std-features=compiler-builtins-mem

