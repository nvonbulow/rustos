#![feature(abi_x86_interrupt)]
#![feature(alloc)]
#![feature(allocator_api)]
#![feature(global_allocator)]
#![feature(lang_items)]
#![feature(unique)]
#![feature(const_fn)]
#![no_std]

#[allow(unused_imports)]
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
extern crate raw_cpuid;
extern crate rlibc;
extern crate spin;
extern crate volatile;
extern crate x86_64;

#[macro_use]
mod io;
mod interrupts;
mod memory;

#[no_mangle]
pub extern fn rust_main(multiboot_info: usize) {
    io::vga::text_buffer::clear_screen();

    let boot_info = unsafe {
        multiboot2::load(multiboot_info)
    };
    let mut memory_controller = memory::init(boot_info);

    unsafe {
        HEAP_ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE)
    }
    interrupts::init(&mut memory_controller);

    kprintln!("It did not crash!");

    {
        use io::term::ansi::*;
        let green: AnsiSequence = AnsiSequence::SetGraphicsMode([
            Some(TextAttribute::Blue),
            None,
            None,
        ]);
        let red: AnsiSequence = AnsiSequence::SetGraphicsMode([
            Some(TextAttribute::Magenta),
            None,
            None,
        ]);
        kprintln!("{}MAGENTA", red.to_escaped_string());
        kprint!("{}BLUE", green.to_escaped_string());
    }

    let com1 = &mut io::serial::COM1.lock();
    loop {
        match com1.read_byte() {
            Some(b) => {
                com1.write_byte_sync(b);
                kprint!("{}", b as char);
            },
            None => {}
        }
    }
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
