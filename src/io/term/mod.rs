use core::fmt;
use spin::{Mutex, MutexGuard};

pub mod ansi;

static PRINTER: &Mutex<PrinterDriver<::io::vga::text_buffer::Writer>> =
    &VGA_TEXT_BUFFER;

static VGA_TEXT_BUFFER: Mutex<PrinterDriver<::io::vga::text_buffer::Writer>> =
    Mutex::new(PrinterDriver(&::io::vga::text_buffer::WRITER));

pub struct PrinterDriver<'a, T: ansi::AnsiWrite + 'a>(&'a Mutex<T>);

#[allow(dead_code)]
impl<'a, T> PrinterDriver<'a, T> where T: ansi::AnsiWrite {
    // const SERIAL_COM1: Self = PrinterDriver(::io::serial::kprint);
}

impl<'a, T> fmt::Write for PrinterDriver<'a, T> where T: ansi::AnsiWrite {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Use AnsiWriter here
        self.0.lock().write_ansi_str(s)
    }
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
    use core::fmt::Write;
    PRINTER.lock().write_fmt(args).unwrap();
}
