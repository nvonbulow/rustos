#![feature(lang_items)]
#![feature(unique)]
#![feature(const_fn)]
#![no_std]

#[macro_use]
extern crate bitflags;
extern crate multiboot2;
extern crate rlibc;
extern crate spin;
extern crate volatile;

#[macro_use]
mod vga_buffer;

#[no_mangle]
pub extern fn rust_main(multiboot_info: usize) {
    let boot_info = unsafe { multiboot2::load(multiboot_info) };
    let memory_map_tag = boot_info.memory_map_tag().expect("Memory Map Tag Required");
    let elf_sections_tag = boot_info.elf_sections_tag().expect("Elf Sections Tag Required!");

    vga_buffer::clear_screen();
    kprint!("hello");
    kprintln!("memory areas");
    for area in memory_map_tag.memory_areas() {
        kprintln!("  start: 0x{:x}, length: 0x{:x}", area.base_addr, area.length);
    }
    kprintln!("Kernel sections:");
    for section in elf_sections_tag.sections() {
        kprintln!("  addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}", section.addr, section.size, section.flags);
    }
    let kernel_start = elf_sections_tag.sections().map(|s| s.addr).min().unwrap();
    let kernel_end = elf_sections_tag.sections().map(|s| s.addr + s.size).max().unwrap();
    let multiboot_start = multiboot_info;
    let multiboot_end = multiboot_start + (boot_info.total_size as usize);

    kprintln!("Kernel goes from 0x:{:x} to 0x{:x}", kernel_start, kernel_end);
    kprintln!("Multiboot goes from 0x{:x} to 0x{:x}", multiboot_start, multiboot_end);
    loop {}
}



#[lang = "eh_personality"]
#[no_mangle]
pub extern fn eh_personality() {}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    kprintln!("\n\nPANIC in {} at line {}:", file, line);
    kprintln!("  {}", fmt);
    loop {}
}
