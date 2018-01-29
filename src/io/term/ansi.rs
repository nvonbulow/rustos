// http://ascii-table.com/ansi-escape-sequences.php
use core::fmt;

use alloc::String;
pub use self::TextAttribute::*;
pub use self::AnsiSequence::*;

pub const ESCAPE: char = '\x1b';

pub trait AnsiWrite: fmt::Write {
    fn write_ansi_str(&mut self, s: &str) -> fmt::Result {
        let mut chars = s.chars();
        while let Some(char) = chars.next() {
            match char {
                ESCAPE => {
                    let mut esc_chars = chars.clone().as_str();
                    let seq = AnsiSequence::parse(esc_chars);
                    if seq.is_some() {
                        self.write_ansi_sequence(seq.unwrap()).unwrap();
                        while let Some(c) = chars.next() {
                            if c.is_alphabetic() {
                                break;
                            }
                        }
                    }
                    else {
                        panic!("Invalid escape sequence!");
                    }
                },
                c => {
                    self.write_char(c).unwrap();
                }
            };
        }
        Ok(())
    }

    fn write_ansi_sequence(&mut self, seq: AnsiSequence) -> fmt::Result;
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum TextAttribute {
    Off        = 0,
    Bold       = 1,
    Underscore = 2,
    Blink      = 5,
    Reverse    = 7,
    Concealed = 8,

    Black   = 30,
    Red     = 31,
    Green   = 32,
    Yellow  = 33,
    Blue    = 34,
    Magenta = 35,
    Cyan    = 36,
    White   = 37,

    BlackBackground   = 40,
    RedBackground     = 41,
    GreenBackground   = 42,
    YellowBackground  = 43,
    BlueBackground    = 44,
    MagentaBackground = 45,
    CyanBackground    = 46,
    WhiteBackground   = 47,
}

impl TextAttribute {
    pub fn from_u8(n: u8) -> Option<Self> {
        use core::mem;
        if n <= 8 || (n >= 30 && n <= 37) || (n >= 40 && n <= 47) {
            Some(unsafe { mem::transmute(n) })
        }
        else {
            None
        }
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum ScreenMode {
    TextMonochrome40x25 = 0,
    TextColor40x25      = 1,
    TextMonochrome80x25 = 2,
    TextColor80x25      = 3,
    // Graphics
    EnableLineWrapping  = 7,
    // Graphics
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum AnsiSequence {
    CursorPosition { row: u8, col: u8 },
    CursorUp(u8),
    CursorDown(u8),
    CursorForward(u8),
    CursorBackward(u8),
    SaveCursorPosition,
    RestoreCursorPosition,
    EraseDisplay,
    EraseLine,
    SetGraphicsMode([Option<TextAttribute>; 3]),
    // SetMode(ScreenMode),
    // ResetMode,
    // SetKeyboardStrings(&'a [StringMappings]),
    UnknownSequence,
}

impl AnsiSequence {
    // Try to find a way to just use &str
    pub fn to_string(&self) -> String {
        use alloc::string::ToString;
        let code = match *self {
            CursorPosition { row, col } => {
                format!("{};{}H", row, col)
            },
            CursorUp(amount) => {
                format!("{}A", amount)
            },
            CursorDown(amount) => {
                format!("{}B", amount)
            },
            CursorForward(amount) => {
                format!("{}C", amount)
            },
            CursorBackward(amount) => {
                format!("{}D", amount)
            },
            SaveCursorPosition => { "s".to_string() },
            RestoreCursorPosition => { "u".to_string() },
            EraseDisplay => { "2J".to_string() },
            EraseLine => { "K".to_string() },
            SetGraphicsMode(ref modes) => {
                let mut str = "".to_string();
                for (i, mode) in modes.into_iter().enumerate() {
                    str += match *mode {
                        Some(m) => format!("{}", m as u8),
                        None => "".to_string(),
                    }.as_str();
                    if i < 2 && modes[i + 1].is_some() {
                        str += ";";
                    }
                }
                str + "m"
            },
            _ => { return "bad ansi str".to_string(); }
        };
        format!("[{}", code)
        // format!("{}[{}", ESCAPE, code)
    }

    pub fn to_escaped_string(&self) -> String {
        format!("{}{}", ESCAPE, self.to_string())
    }

    pub fn parse(s: &str) -> Option<Self> {
        let mut chars = s.chars();
        // Make sure first two chars are `ESCAPE and '['`
        match chars.next() {
            Some(ESCAPE) => {
                match chars.next() {
                    Some('[') => {},
                    _ => {
                        return None;
                    }
                }
            },
            Some('[') => {},
            _ => {
                return None;
            }
        };
        const MAX_ARGS: usize = 3;
        let mut args: ([Option<u8>; MAX_ARGS], usize) = ([None; MAX_ARGS], 0);
        for (i, c) in chars.clone().enumerate() {
            // Is this the end of the sequence?
            if c.is_alphabetic() {
                return Some(match c {
                    'H' | 'f' => { CursorPosition {
                        row: args.0[0].unwrap_or(0 as u8),
                        col: args.0[1].unwrap_or(0 as u8),
                    }},
                    'A' => { CursorUp(args.0[0].unwrap_or(1 as u8)) },
                    'B' => { CursorDown(args.0[0].unwrap_or(1 as u8)) },
                    'C' => { CursorForward(args.0[0].unwrap_or(1 as u8)) },
                    'D' => { CursorBackward(args.0[0].unwrap_or(1 as u8)) },
                    's' => { SaveCursorPosition },
                    'u' => { RestoreCursorPosition },
                    'J' => {
                        if let Some(arg) = args.0[0] {
                            if arg == 2 {
                                EraseDisplay
                            }
                            else {
                                UnknownSequence
                            }
                        }
                        else {
                            UnknownSequence
                        }
                    },
                    'K' => { EraseLine },
                    'm' => {
                        let mut mapped_vals: [Option<TextAttribute>; MAX_ARGS] = [None; MAX_ARGS];
                        for (i, val) in args.0.into_iter().enumerate() {
                            mapped_vals[i] = val.and_then(|v| TextAttribute::from_u8(v))
                        }
                        SetGraphicsMode(mapped_vals)
                    },
                    _   => { UnknownSequence }
                });
            }
            // Is this part of an argument?
            else if c.is_numeric() {
                if args.1 >= MAX_ARGS {
                    return Some(UnknownSequence);
                }
                // Already taken care of
                if args.0[args.1].is_some() {
                    continue;
                }
                let z = '0' as u8;
                let c1 = c as u8;
                let c2 = chars.clone().nth(i + 1).unwrap_or('\x00') as u8;
                args.0[args.1] = if (c2 as char).is_numeric() {
                    Some((c1 - z) * 10 + (c2 - z))
                }
                else {
                    Some(c1 - z)
                }
            }
            // Is this an argument separator?
            else if c == ';' {
                args.1 += 1;
            }
            // Otherwise it's invalid
            else {
                return None;
            }
        }
        None
    }
}
