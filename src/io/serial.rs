use core::fmt;
use spin::Mutex;

use super::Port;

#[allow(dead_code)]
#[repr(u16)]
enum SerialIoPort {
    COM1 = 0x3F8,
    COM2 = 0x2F8,
    COM3 = 0x3E8,
    COM4 = 0x2E8,
}

#[allow(dead_code)]
#[repr(u16)]
// Offsets for the base io port
enum Register {
    Data = 0,
    // The 1 << 7 is just to differentiate between data and divisor latch
    DivisorLatchHigh = 1 | 1 << 7,
    DivisorLatchLow = 0 | 1 << 7,
    InterruptEnable = 1,
    LineControl = 3,
    LineStatus = 5,
    FIFOControl = 2,
    Scratch = 7,
}

const SERIAL_CLOCK_BASE: u32 = 115200;

#[allow(dead_code)]
lazy_static! {
    #[allow(dead_code)]
    pub static ref COM1: Mutex<SerialPort> = Mutex::new(SerialPort::init(SerialIoPort::COM1));
    #[allow(dead_code)]
    pub static ref COM2: Mutex<SerialPort> = Mutex::new(SerialPort::init(SerialIoPort::COM2));
    #[allow(dead_code)]
    pub static ref COM3: Mutex<SerialPort> = Mutex::new(SerialPort::init(SerialIoPort::COM3));
    #[allow(dead_code)]
    pub static ref COM4: Mutex<SerialPort> = Mutex::new(SerialPort::init(SerialIoPort::COM4));
}

#[allow(dead_code)]
#[allow(unused_variables)]
// dummy function to get rid of stupid errors
fn init() {
    let com1 = &COM1.lock();
    let com2 = &COM2.lock();
    let com3 = &COM3.lock();
    let com4 = &COM4.lock();
}

pub struct SerialPort {
    port: Port<u8>
}

#[allow(dead_code)]
impl SerialPort {
    fn init(io_base: SerialIoPort) -> SerialPort {
        let mut port = SerialPort {
            port: unsafe { Port::new(io_base as u16) },
        };
        port.set_baud_rate(SERIAL_CLOCK_BASE);
        port.write_register(Register::FIFOControl, 0xC7);
        port
    }

    fn register_port(&mut self, register: Register) -> Port<u8> {
        unsafe { self.port.offset(register as u16 & 0x7F) }
    }

    fn read_register(&mut self, register: Register) -> u8 {
        self.register_port(register).read()
    }

    fn write_register(&mut self, register: Register, val: u8) {
        self.register_port(register).write(val)
    }

    fn set_dlab(&mut self, enabled: bool) {
        let mut lcr = self.read_register(Register::LineControl);
        if enabled {
            lcr |= 0x80;
        }
        else {
            lcr &= 0x7F;
        }
        self.write_register(Register::LineControl, lcr);
    }

    pub fn get_baud_rate_divisor(&mut self) -> u16 {
        self.set_dlab(true);
        let high = self.read_register(Register::DivisorLatchHigh) as u16;
        let low = self.read_register(Register::DivisorLatchLow) as u16;
        self.set_dlab(false);
        (high << 8) + low
    }

    pub fn set_baud_rate_divisor(&mut self, divisor: u16) {
        let high = divisor as u8;
        let low = (divisor >> 8) as u8;
        self.set_dlab(true);
        self.write_register(Register::DivisorLatchHigh, high);
        self.write_register(Register::DivisorLatchLow, low);
        self.set_dlab(false);
    }

    pub fn get_baud_rate(&mut self) -> u32 {
        SERIAL_CLOCK_BASE / self.get_baud_rate_divisor() as u32
    }

    pub fn set_baud_rate(&mut self, baud: u32) {
        if SERIAL_CLOCK_BASE % baud != 0 {
            panic!("Can only set the baud rate to a divisor of {}", SERIAL_CLOCK_BASE);
        }
        self.set_baud_rate_divisor((SERIAL_CLOCK_BASE / baud) as u16);
    }

    pub fn has_available_byte(&mut self) -> bool {
        self.read_register(Register::LineStatus) & 1 == 1
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.has_available_byte() {
            Some(self.read_register(Register::Data))
        }
        else {
            None
        }
    }

    pub fn has_write_space(&mut self) -> bool {
        self.read_register(Register::LineStatus) & 0x20 != 0
    }

    pub fn write_byte_sync(&mut self, val: u8) {
        loop {
            if self.write_byte(val) {
                break;
            }
        }
    }

    pub fn write_byte(&mut self, val: u8) -> bool {
        if !self.has_write_space() {
            false
        }
        else {
            self.write_register(Register::Data, val);
            true
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.write_byte_sync(b);
        }
    }
}

struct SerialWriter<'a>(&'a mut SerialPort);

#[allow(dead_code)]
impl<'a> SerialWriter<'a> {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                self.0.write_byte_sync(byte);
            }
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.0.write_byte(byte);
        }
    }

    pub fn new_line(&mut self) {
        self.0.write_str("\r\n");
    }
}

impl<'a> fmt::Write for SerialWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

pub fn kprint(args: fmt::Arguments) {
    use core::fmt::Write;
    use core::borrow::BorrowMut;

    let com1 = &mut COM1.lock();
    let mut writer = SerialWriter(com1.borrow_mut());

    writer.write_fmt(args).unwrap()
}
