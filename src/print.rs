use core::fmt;

static PRINTER: PrinterDriver = PrinterDriver::SERIAL_COM1;

pub struct PrinterDriver(fn(fmt::Arguments));

#[allow(dead_code)]
impl PrinterDriver {
    const VGA_TEXT_BUFFER: Self = PrinterDriver(::vga::text_buffer::kprint);
    const SERIAL_COM1: Self = PrinterDriver(::io::serial::kprint);
}

macro_rules! kprint {
    ($($arg:tt)*) => ({
        $crate::print::kprint(format_args!($($arg)*))
    });
}

macro_rules! kprintln {
    ($fmt:expr) => (kprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (kprint!(concat!($fmt, "\n"), $($arg)*));
}

pub fn kprint(args: fmt::Arguments) {
    PRINTER.0(args);
}
