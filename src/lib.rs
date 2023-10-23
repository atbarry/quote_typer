pub mod quote;
pub mod typing;

use colored::*;
use crossterm::{cursor, execute, queue, style, terminal};
use log::debug;
use quote::Quote;
use std::{
    fmt::Display,
    io::{self, Stdout, Write},
    iter::zip,
};
use typing::SessionType;

#[derive(Copy, Clone)]
struct Stats {
    num_chars_typed: u32,
    num_correct: u32,
    current_quote: u32,
    num_quotes: Option<u32>,
    elapsed_time: f32,
}

impl Stats {
    fn new(session_type: SessionType) -> Self {
        let num_quotes = match session_type {
            SessionType::MultiQuote(x) => Some(x as u32),
            _ => None,
        };

        Self {
            num_chars_typed: 0,
            num_correct: 0,
            current_quote: 1,
            num_quotes,
            elapsed_time: 0.0,
        }
    }

    fn update(
        &mut self,
        quote_chars: &[char],
        typed_chars: &[char],
        current_quote: u32,
        elapsed_time: f32,
    ) {
        self.num_correct = std::iter::zip(quote_chars.iter(), typed_chars.iter())
            .filter(|(q, t)| q == t)
            .count() as u32;
        self.num_chars_typed = typed_chars.len() as u32;
        self.current_quote = current_quote;
        self.elapsed_time = elapsed_time;
    }

    fn analysis_str(&self, extra: &str) -> String {
        let total_str = self.num_chars_typed.to_string().color(Color::Blue);
        let num_correct_str = self.num_correct.to_string().color(Color::Green);
        let mistakes_str = (self.num_chars_typed - self.num_correct)
            .to_string()
            .color(Color::Red);
        let cpm = 60.0 * self.num_correct as f32 / self.elapsed_time;
        let wpm = cpm / 5.0;
        format!(
r#"Total: {}, Correct: {}, Mistakes: {}
Elapsed Time: {}
CPM: {}
WPM: {}
{}
"#,
            total_str,
            num_correct_str,
            mistakes_str,
            self.elapsed_time,
            cpm.to_string().color(Color::Blue),
            wpm.to_string().color(Color::Blue),
            extra
        )
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cpm = (60.0 * (self.num_correct as f32) / self.elapsed_time) as u32;
        let progress = if let Some(num_quotes) = self.num_quotes {
            format!("{}/{}", self.current_quote, num_quotes)
        } else {
            self.current_quote.to_string()
        };

        write!(
            f,
            "Time: {}, Correct: {}, CPM: {}, Progress: {}",
            self.elapsed_time as u32, self.num_correct, cpm, progress
        )
    }
}

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

        // overrides
        let outchar = match (*typed_char, *quote_char) {
            (t, ' ') => t,
            (_t, q) => q,
        };

        Self {
            character: outchar,
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
        queue!(out, cursor::MoveTo(cursor_col, cursor_row))
    }

    fn print_stats(&self, out: &mut Stdout, stats: Stats) -> io::Result<()> {
        queue!(
            out,
            cursor::MoveTo(0, self.num_rows - 1),
            style::Print(stats.to_string())
        )
    }

    fn write_before(&self, out: &mut Stdout, chars: &[ColoredChar]) -> io::Result<()> {
        let mut cursor = self.clone();

        for c in chars {
            if cursor.cursor_back_one().is_err() {
                break;
            };

            queue!(
                out,
                cursor::MoveTo(cursor.col, cursor.row),
                style::SetForegroundColor(c.color),
                style::Print(c.character)
            )?;
        }
        queue!(out, cursor::MoveTo(self.col, self.row))
    }

    fn write_after(&self, out: &mut Stdout, chars: &[char]) -> io::Result<()> {
        let mut cursor = self.clone();
        queue!(out, style::SetForegroundColor(style::Color::Reset))?;

        for c in chars {
            if cursor.cursor_forward_one().is_err() {
                break;
            };

            queue!(out, style::Print(c))?;
        }
        let clear_type = terminal::ClearType::UntilNewLine;
        queue!(
            out,
            cursor::MoveToNextLine(1),
            terminal::Clear(clear_type),
            cursor::MoveTo(self.col, self.row)
        )
    }

    fn cursor_back_one(&mut self) -> Result<(), ()> {
        if self.col == 0 && self.row == 0 {
            // cannot move back one if at 0, 0
            return Err(());
        } else if self.col == 0 {
            self.row -= 1;
            self.col = self.num_cols - 1;
        } else {
            self.col -= 1;
        }
        Ok(())
    }

    fn cursor_forward_one(&mut self) -> Result<(), ()> {
        if self.col == self.num_cols - 1 && self.row == self.num_rows - 1 {
            // cannot move back one if at 0, 0
            return Err(());
        } else if self.col == self.num_cols - 1 {
            self.row += 1;
            self.col = 0;
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

fn write_to_terminal(
    out: &mut Stdout,
    quote_chars: &[char],
    typed_chars: &[char],
    stats: Option<Stats>,
) -> std::io::Result<()> {
    let mut cursor = Cursor::new()?;
    if let Some(stats) = stats {
        cursor.print_stats(out, stats)?;
    }
    cursor.align_center(out, typed_chars.len() as u32)?;
    let before_cursor: Vec<ColoredChar> = zip(typed_chars.iter(), quote_chars.iter())
        .map(|(t, q)| ColoredChar::new(t, q))
        .rev()
        .collect();

    let after_cursor = &quote_chars[typed_chars.len()..];
    cursor.write_before(out, &before_cursor)?;
    cursor.write_after(out, after_cursor)?;
    out.flush()
}

fn get_number_input(out: &mut Stdout) -> io::Result<u16> {
    terminal::disable_raw_mode()?;
    clear_screen_and_print(out, "", false)?;
    let num;
    loop {
        let mut input = String::new();
        println!("Please enter a number:");
        io::stdin().read_line(&mut input)?;
        let number: Result<u16, _> = input.trim().parse();

        match number {
            Ok(n) => {
                num = n;
                break;
            }
            Err(_) => {
                println!("Invalid input. Please enter an integer from 1..2^16");
                continue; // Continue the loop to prompt for input again
            }
        }
    }
    terminal::enable_raw_mode()?;
    return Ok(num);
}

fn clear_screen_and_print(out: &mut Stdout, stuff: &str, is_raw: bool) -> io::Result<()> {
    if is_raw {
        terminal::disable_raw_mode()?;
    }
    execute!(
        out,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print(stuff)
    )?;
    if is_raw {
        terminal::enable_raw_mode()?;
    }
    Ok(())
}

