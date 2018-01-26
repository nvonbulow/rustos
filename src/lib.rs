#![feature(abi_x86_interrupt)]
#![feature(alloc)]
#![feature(allocator_api)]
#![feature(global_allocator)]
#![feature(lang_items)]
#![feature(unique)]
#![feature(const_fn)]
#![no_std]

#[macro_use]
extern crate alloc;
extern crate bitfield;
extern crate bit_field;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate multiboot2;
#[macro_use]
extern crate once;
extern crate rlibc;
extern crate spin;
extern crate volatile;
extern crate x86;
extern crate x86_64;

#[macro_use]
mod vga_buffer;
mod interrupts;
mod memory;

#[no_mangle]
pub extern fn rust_main(multiboot_info: usize) {
    vga_buffer::clear_screen();
    kprintln!("Hello!");

    let boot_info = unsafe {
        multiboot2::load(multiboot_info)
    };
    let mut memory_controller = memory::init(boot_info);

    unsafe {
        HEAP_ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE)
    }
    interrupts::init(&mut memory_controller);

    // unsafe { *(0xdeadbeaf as *mut u64) = 42; };

    fn stack_overflow() {
        stack_overflow();
    }

    stack_overflow();

    kprintln!("It did not crash!");
    loop {}
}

pub const HEAP_START: usize = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: usize = 100 * 1024;

use memory::heap_allocator::linked_list_allocator::LockedHeap;
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();


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
