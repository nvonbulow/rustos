RELEASE ?= 0
ifeq ($(RELEASE), 1)
    buildtype := release
else
    buildtype := debug
endif

arch ?= x86_64
target ?= $(arch)-rustos

build_dir := build/$(target)/$(buildtype)
kernel := $(build_dir)/kernel-$(arch).bin
kernel_debug := $(kernel).debug
iso := $(build_dir)/os-$(arch).iso

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
    $(build_dir)/arch/$(arch)/%.o, $(assembly_source_files))
rust_source_files := $(shell find src/ -type f -name "*.rs")

rust_os = target/$(target)/$(buildtype)/librustos.a

xargo_flags =
ifeq ($(RELEASE), 1)
    xargo_flags += --release
endif

ld_flags = -n --gc-sections

.PHONY: all clean run debug iso kernel release

all: $(kernel)

clean:
	@rm -rf build
	@xargo clean

run: $(iso)
	@qemu-system-x86_64 -no-reboot -cdrom $(iso) -s

debug: $(iso)
	@qemu-system-x86_64 -d int -no-reboot -cdrom $(iso) -s -S

gdb: $(kernel)
	@gdb $(kernel) -ex "target remote :1234"

iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p $(build_dir)/isofiles/boot/grub
	@cp $(kernel) $(build_dir)/isofiles/boot/kernel.bin
	@cp $(grub_cfg) $(build_dir)/isofiles/boot/grub
	@grub-mkrescue -o $(iso) $(build_dir)/isofiles 2> /dev/null
	@rm -r $(build_dir)/isofiles
	@echo $(buildtype)

$(kernel): $(rust_os) $(assembly_object_files) $(linker_script)
	ld $(ld_flags) -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)
	@objcopy --only-keep-debug $(kernel) $(kernel_debug)
	@strip --strip-debug --strip-unneeded $(kernel)
	@objcopy --add-gnu-debuglink="$(kernel_debug)" $(kernel)

$(rust_os): $(rust_source_files)
	@RUST_TARGET_PATH="$(shell pwd)" xargo build --target $(target) $(xargo_flags)

# compile assembly files
$(build_dir)/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@