arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
    build/arch/$(arch)/%.o, $(assembly_source_files))
rust_source_files := $(shell find src/ -type f -name "*.rs")

target ?= $(arch)-rustos
buildtype ?= debug
rust_os = target/$(target)/$(buildtype)/librustos.a

.PHONY: all clean run debug iso kernel

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
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

$(kernel): $(rust_os) $(assembly_object_files) $(linker_script)
	ld -n --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

$(rust_os): $(rust_source_files)
	@RUST_TARGET_PATH="$(shell pwd)" xargo build --target $(target)

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@