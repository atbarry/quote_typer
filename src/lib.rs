pub mod quote;
pub mod typing;

use crossterm::{cursor, execute, queue, style, terminal};
use log::{log, debug};
use quote::Quote;
use std::{
    io::{self, Stdout, Write},
    iter::zip,
};

struct ColoredChar {
    character: char,
    color: style::Color,
}

impl ColoredChar {
    fn new(typed_char: &char, quote_char: &char) -> Self {
        let color = if typed_char == quote_char {
            style::Color::Green
        } else {
            style::Color::Red
        };

        Self {
            character: *quote_char,
            color,
        }
    }
}

#[derive(Clone, Debug)]
struct Cursor {
    col: u16,
    row: u16,
    num_cols: u16,
    num_rows: u16,
}

impl Cursor {
    fn new() -> io::Result<Self> {
        let (col, row) = cursor::position()?;
        let (num_cols, num_rows) = terminal::size()?;
        Ok(Self {
            col,
            row,
            num_cols,
            num_rows,
        })
    }

    fn align_center(&mut self, out: &mut Stdout, num_chars: u32) -> io::Result<()> {
        debug!("{:?}, {:?}", num_chars, self.num_cols);
        let cursor_col = num_chars as u16 % self.num_cols;
        let mut cursor_row = num_chars as u16 / self.num_cols;

        if cursor_row > self.num_rows / 2 {
            cursor_row = self.num_rows / 2;
        }

        self.col = cursor_col;
        self.row = cursor_row;
        execute!(out, cursor::MoveTo(cursor_col, cursor_row))
    }

    fn write_before(&self, out: &mut Stdout, chars: &[ColoredChar]) -> io::Result<()> {
        let mut cursor = self.clone();

        for c in chars {
            if cursor.cursor_back_one(&c.character).is_err() {
                break;
            };

            queue!(
                out,
                cursor::MoveTo(cursor.col, cursor.row),
                style::SetForegroundColor(c.color),
                style::Print(c.character)
            )?;
        }
        queue!(out, cursor::MoveTo(self.col, self.row))?;
        out.flush()
    }

    fn write_after(&self, out: &mut Stdout, chars: &[char]) -> io::Result<()> {
        let mut cursor = self.clone();
        queue!(out, style::SetForegroundColor(style::Color::Reset))?;

        for c in chars {
            if cursor.cursor_forward_one(c).is_err() {
                break;
            };

            queue!(out, style::Print(c))?;
        }
        queue!(out, cursor::MoveTo(self.col, self.row))?;
        out.flush()
    }

    fn cursor_back_one(&mut self, c: &char) -> Result<(), ()> {
        if c == &'\n' {
            if self.row == 0 {
                return Err(());
            } else {
                self.row -= 1;
                return Ok(());
            }
        }
        if self.col == 0 && self.row == 0 {
            // cannot move back one if at 0, 0
            return Err(());
        } else if self.col == 0 {
            self.row -= 1;
        } else {
            self.col -= 1;
        }
        Ok(())
    }

    fn cursor_forward_one(&mut self, c: &char) -> Result<(), ()> {
        if c == &'\n' {
            if self.row == self.num_rows - 1 {
                return Err(());
            } else {
                self.row += 1;
                return Ok(());
            }
        }
        if self.col == self.num_cols - 1 && self.row == self.num_rows - 1 {
            // cannot move back one if at 0, 0
            return Err(());
        } else if self.col == self.num_cols - 1 {
            self.row += 1;
        } else {
            self.col += 1;
        }
        Ok(())
    }
}

pub fn initialize_session(out: &mut Stdout) -> Result<(), std::io::Error> {
    execute!(
        out,
        terminal::EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
    )?;
    terminal::enable_raw_mode()
}

pub fn terminate_session(out: &mut Stdout) -> std::io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(out, style::ResetColor, terminal::LeaveAlternateScreen)?;
    println!("\n");
    Ok(())
}

fn reset_session(
    out: &mut Stdout,
    quote_chars: &[char],
    typed_chars: &[char],
) -> std::io::Result<()> {
    let mut cursor = Cursor::new()?;
    cursor.align_center(out, typed_chars.len() as u32)?;
    let before_cursor: Vec<ColoredChar> = zip(typed_chars.iter(), quote_chars.iter())
        .map(|(t, q)| ColoredChar::new(t, q))
        .rev()
        .collect();

    let after_cursor = &quote_chars[typed_chars.len()..];
    cursor.write_before(out, &before_cursor)?;
    cursor.write_after(out, after_cursor)?;
    Ok(())
}

// fn print_char(stdout: &mut Stdout, quote: &[char], chars_typed: &[char]) -> std::io::Result<()> {
//     let len = chars_typed.len();
//     let c = quote[len - 1];
//     let color = if &c == chars_typed.last().unwrap() {
//         style::Color::Green
//     } else {
//         style::Color::Red
//     };
//
//     execute!(stdout, style::SetForegroundColor(color), style::Print(c))
// }

fn on_backspace(out: &mut Stdout) -> io::Result<()> {
    let (cursor_col, cursor_row) = cursor::position()?;
    let (terminal_cols, termial_rows) = terminal::size()?;
    // if the cursor is on the first or 0th column then
    // the cursor needs to be moved up one row and all
    // the way to the right.
    if cursor_col == 0 && cursor_row != 0 {
        execute!(
            out,
            cursor::MoveToPreviousLine(1),
            cursor::MoveToColumn(terminal_cols - 1),
            style::Print("")
        )?;
    } else {
        execute!(out, cursor::MoveLeft(1), style::Print(""))?;
    }

    Ok(())
}
