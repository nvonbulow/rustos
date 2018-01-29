use core::ptr::Unique;
use core::fmt;
use spin::Mutex;
use volatile::Volatile;

use io::term::ansi::{self, AnsiWrite, AnsiSequence};
use io::Port;

#[allow(dead_code)]
#[repr(u8)]
pub enum Color {
    // Darker colors
    Black      = 0,
    Blue       = 1,
    Green      = 2,
    Cyan       = 3,
    Red        = 4,
    Magenta    = 5,
    Brown      = 6,
    LightGray  = 7,

    // Lighter colors
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    Pink       = 13,
    Yellow     = 14,
    White      = 15,
}

#[derive(Debug, Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }

    pub fn foreground(&self) -> Color {
        use core::mem;
        unsafe { mem::transmute(self.0 & 0xF) }
    }

    pub fn background(&self) -> Color {
        use core::mem;
        unsafe { mem::transmute((self.0 >> 4) & 0xF) }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

#[derive(Copy, Clone)]
struct CursorPosition {
    row: usize,
    col: usize,
}

pub struct Writer {
    pos: CursorPosition,
    saved_pos: Option<CursorPosition>,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
    cursor_port: Port<u8>,
}

#[allow(dead_code)]
impl Writer {
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            // backspace
            b'\x08' => {
                if self.pos.col != 0 {
                    self.pos.col -= 1;
                }
            },
            byte => {
                if self.pos.row >= BUFFER_WIDTH {
                    self.new_line();
                }

                let CursorPosition { row, col } = self.pos;

                let color_code = self.color_code;
                self.buffer().chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.pos.col += 1;
            }
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        self.update_cursor();
    }

    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.as_mut() }
    }

    fn shift_up(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let buffer = self.buffer();
                let character = buffer.chars[row][col].read();
                buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
    }

    fn new_line(&mut self) {
        if self.pos.row == BUFFER_HEIGHT - 1 {
            self.shift_up();
        }
        else {
            self.pos.row += 1;
        }
        self.pos.col = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer().chars[row][col].write(blank);
        }
    }

    fn clear_screen(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
    }

    fn update_cursor(&mut self) {
        let pos = (self.pos.row * BUFFER_WIDTH + self.pos.col) as u16;
        let mut data_port = unsafe { self.cursor_port.offset(1) };

        // Cursor location low
        self.cursor_port.write(0x0f);
        data_port.write(pos as u8);
        // Cursor location high
        self.cursor_port.write(0x0e);
        data_port.write((pos >> 8) as u8);
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        self.pos = CursorPosition { row, col };
        self.update_cursor();
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte)
        }
        self.update_cursor();
        Ok(())
    }
}

impl AnsiWrite for Writer {
    fn write_ansi_sequence(&mut self, seq: AnsiSequence) -> fmt::Result {
        use self::ansi::*;
        match seq {
            CursorPosition { row, col } => {
                self.pos = self::CursorPosition {
                    row: row as usize,
                    col: col as usize,
                };
                self.update_cursor();
            },
            CursorUp(amount) => {
                if self.pos.row != 0 {
                    self.pos.row -= 1;
                    self.update_cursor();
                }
            },
            CursorDown(amount) => {
                if self.pos.row != BUFFER_HEIGHT {
                    self.pos.row += 1;
                    self.update_cursor();
                }
            },
            CursorForward(amount) => {
                if self.pos.col != BUFFER_WIDTH {
                    self.pos.col += 1;
                    self.update_cursor();
                }
            },
            CursorBackward(amount) => {
                if self.pos.col != 0 {
                    self.pos.col -= 1;
                    self.update_cursor();
                }
            },
            SaveCursorPosition => {
                self.saved_pos = Some(self.pos);
            },
            RestoreCursorPosition => {
                if let Some(pos) = self.saved_pos {
                    self.pos = pos;
                    self.update_cursor();
                }
            },
            EraseDisplay => { self.clear_screen(); },
            EraseLine => {
                let row = self.pos.row;
                self.clear_row(row);
            },
            SetGraphicsMode(modes) => {
                let mut foreground = self.color_code.foreground();
                let mut background = self.color_code.background();
                for mode in modes.iter() {
                    if mode.is_some() {
                        foreground = match mode.unwrap() {
                            Black   => Color::Black,
                            Red     => Color::Red,
                            Green   => Color::Green,
                            Yellow  => Color::Yellow,
                            Blue    => Color::Blue,
                            Magenta => Color::Magenta,
                            Cyan    => Color::Cyan,
                            White   => Color::White,

                            _ => foreground,
                        };

                        background = match mode.unwrap() {
                            BlackBackground   => Color::Black,
                            RedBackground     => Color::Red,
                            GreenBackground   => Color::Green,
                            YellowBackground  => Color::Yellow,
                            BlueBackground    => Color::Blue,
                            MagentaBackground => Color::Magenta,
                            CyanBackground    => Color::Cyan,
                            WhiteBackground   => Color::White,

                            _ => background,
                        }
                    }
                }
                self.color_code = ColorCode::new(foreground, background);
            },
            _ => {}
        };
        Ok(())
    }
}

pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
    pos: CursorPosition { row: 0, col: 0},
    saved_pos: None,
    color_code: ColorCode::new(Color::White, Color::Black),
    buffer: unsafe { Unique::new_unchecked(0xb8000 as *mut _) },
    cursor_port: unsafe { Port::new(0x3d4) },
});

pub fn clear_screen() {
    WRITER.lock().clear_screen()
}
