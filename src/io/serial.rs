#![allow(dead_code)]

// THIS FILE IS NOT PUBLIC YET
// SIMPLY CODE FROM THE OLD KERNEL

use x86::shared::io::*;
use spin::Mutex;

const COM1_PORT: u16 = 0x3F8;
const COM2_PORT: u16 = 0x2F8;
const COM3_PORT: u16 = 0x3E8;
const COM4_PORT: u16 = 0x2E8;

lazy_static! {
    pub static ref COM1: Mutex<Port> = Mutex::new(Port::init(COM1_PORT));
    pub static ref COM2: Mutex<Port> = Mutex::new(Port::init(COM2_PORT));
    pub static ref COM3: Mutex<Port> = Mutex::new(Port::init(COM3_PORT));
    pub static ref COM4: Mutex<Port> = Mutex::new(Port::init(COM4_PORT));
}

const CLOCK_BASE: u32 = 115200;

#[repr(u8)]
#[allow(dead_code)]
enum Register {
    Data = 0,
    DivisorLatchHigh = 1 | 1 << 7,
    DivisorLatchLow = 0 | 1 << 7,
    InterruptEnable = 1,
    LineControl = 3,
    LineStatus = 5,
    FIFOControl = 2,
    Scratch = 7,
}

pub struct Port {
    io_base: u16
}

#[allow(dead_code)]
impl Port {

    pub fn init(io_base: u16) -> Port {
        let mut port = Port {
            io_base: io_base
        };
        port.set_baud_rate(CLOCK_BASE);
        port.write_register(Register::FIFOControl, 0xC7);
        port
    }

    fn read_register(&self, register: Register) -> u8 {
        unsafe {
            inb(self.io_base + (register as u16 & 0x7F))
        }
    }

    fn write_register(&self, register: Register, value: u8) {
        unsafe {
            outb(self.io_base + (register as u16 & 0x7F), value);
        }
    }

    fn set_dlab(&self) {
        let mut lcr = self.read_register(Register::LineControl);
        lcr |= 0x80;
        self.write_register(Register::LineControl, lcr);
    }

    fn clear_dlab(&self) {
        let mut lcr = self.read_register(Register::LineControl);
        lcr &= 0x7F;
        self.write_register(Register::LineControl, lcr);
    }

    pub fn get_divisor_latch(self) -> u16 {
        self.set_dlab();
        let high = self.read_register(Register::DivisorLatchHigh) as u16;
        let low = self.read_register(Register::DivisorLatchLow) as u16;
        self.clear_dlab();
        high | low
    }

    pub fn set_divisor_latch(&mut self, latch: u16) {
        let high = latch as u8;
        let low = (latch >> 8) as u8;
        self.set_dlab();
        self.write_register(Register::DivisorLatchHigh, high);
        self.write_register(Register::DivisorLatchLow, low);
        self.clear_dlab();
    }

    pub fn get_baud_rate(self) -> u32 {
        CLOCK_BASE / self.get_divisor_latch() as u32
    }

    pub fn set_baud_rate(&mut self, baud: u32) {
        if CLOCK_BASE % baud != 0 {
            panic!("Can only set the baud rate to a divisor of {}", CLOCK_BASE);
        }
        self.set_divisor_latch((CLOCK_BASE / baud) as u16);
    }

    pub fn has_available_byte(&self) -> bool {
        self.read_register(Register::LineStatus) & 1 == 1
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.has_available_byte() {
            return Some(self.read_register(Register::Data))
        }
        None
    }

    pub fn has_write_space(&self) -> bool {
        self.read_register(Register::LineStatus) & 0x20 != 0
    }

    pub fn write_byte(&mut self, byte: u8) -> bool {
        if !self.has_write_space() {
            return false
        }
        self.write_register(Register::Data, byte);
        true
    }

    pub fn write_str(&mut self, s: &str) -> bool {
        for byte in s.bytes() {
            while !self.write_byte(byte) {};
        }
        false
    }

}

// Should go into the VGA mod
fn update_vga_text_cursor(row: usize, column: usize) {
    let position: u16 = (row * 80 + column) as u16;

    unsafe {
        //If cursor breaks, then it's because we have the wrong ports
        outb(0x3D4, 0x0F);
        outb(0x3D5, position as u8);
        outb(0x3D4, 0x0E);
        outb(0x3D5, (position >> 8) as u8);
    }
}
