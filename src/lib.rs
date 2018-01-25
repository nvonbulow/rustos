#![feature(alloc)]
#![feature(allocator_api)]
#![feature(global_allocator)]
#![feature(lang_items)]
#![feature(unique)]
#![feature(const_fn)]
#![no_std]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate bitflags;
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
mod memory;

#[no_mangle]
pub extern fn rust_main(multiboot_info: usize) {
    vga_buffer::clear_screen();
    kprintln!("Hello!");

    let boot_info = unsafe {
        multiboot2::load(multiboot_info)
    };
    memory::init(boot_info);
    unsafe {
        HEAP_ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE)
    }
    kprintln!("Check 1");
    use alloc::boxed::Box;
    let heap_test = Box::new(42);
    kprintln!("Check 2");
    let mut test_vec = vec![1,2,3,4,5,6,7,8,9,0];
    kprintln!("Check 3");
    test_vec[5] = 2;
    for i in test_vec {
        kprint!("{} ", i);
    }
    kprintln!("Check 4");
    for i in 1..1000000 {
        format!("String-O");
    }
    kprintln!("We did NOT crash!!");
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
