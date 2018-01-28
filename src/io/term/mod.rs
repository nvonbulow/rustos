use core::fmt;

pub mod ansi;

static PRINTER: PrinterDriver = PrinterDriver::VGA_TEXT_BUFFER;

pub struct PrinterDriver(fn(fmt::Arguments));

#[allow(dead_code)]
impl PrinterDriver {
    const VGA_TEXT_BUFFER: Self = PrinterDriver(::io::vga::text_buffer::kprint);
    const SERIAL_COM1: Self = PrinterDriver(::io::serial::kprint);
}

macro_rules! kprint {
    ($($arg:tt)*) => ({
        $crate::io::term::kprint(format_args!($($arg)*))
    });
}

macro_rules! kprintln {
    ($fmt:expr) => (kprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (kprint!(concat!($fmt, "\n"), $($arg)*));
}

pub fn kprint(args: fmt::Arguments) {
    PRINTER.0(args);
}
