pub mod quote;

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue, style, terminal,
};
use quote::Quote;
use std::io::{Stdout, Write};

/// This is where the actual typing test is done
pub fn typing_session(stdout: &mut Stdout, quote: &Quote) -> std::io::Result<Vec<char>> {
    initialize_session(stdout, &quote.content)?;
    let quote_chars = quote.content_chars();
    let mut typed_chars: Vec<char> = vec![];
    let (mut cols, mut _rows) = terminal::size()?;

    while typed_chars.len() < quote_chars.len() {
        // `read()` blocks until an `Event` is available
        let key = match read()? {
            Event::Key(key) => key,
            Event::Resize(c, r) => {
                cols = c;
                _rows = r;
                reset_session(stdout, &quote_chars, &typed_chars)?;
                continue
            },
            _ => continue,
        };

        match key {
            // Exit on ctr-c
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => break,
            // Print characters typed
            KeyEvent { code: KeyCode::Char(c), .. } => {
                typed_chars.push(c);
                print_char(stdout, &quote_chars, &typed_chars)?;
            }
            // Go to new line on enter
            KeyEvent { code: KeyCode::Enter, .. } => {
                typed_chars.push('\n');
                print_char(stdout, &quote_chars, &typed_chars)?;
            }
            // On backspace do some stuff
            KeyEvent { code: KeyCode::Backspace, .. } => {
                typed_chars.pop();
                let (cursor_col, cursor_row) = cursor::position()?;
                // if the cursor is on the first or 0th column then
                // the cursor needs to be moved up one row and all
                // the way to the right
                if cursor_col == 0 && cursor_row != 0 {
                    execute!(
                        stdout,
                        cursor::MoveToPreviousLine(1),
                        cursor::MoveToColumn(cols - 1),
                        style::Print("")
                    )?;
                } else {
                    execute!(stdout, cursor::MoveLeft(1), style::Print(""))?;
                }
            }
            _ => (),
        }
    }

    terminate_session(stdout)?;
    Ok(typed_chars)
}


fn initialize_session(stdout: &mut Stdout, quote_str: &str) -> Result<(), std::io::Error> {
    execute!(stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print(quote_str.to_owned()),
        cursor::MoveTo(0, 0)
    )?;
    terminal::enable_raw_mode()
}

fn reset_session(stdout: &mut Stdout, quote: &[char], chars_typed: &[char]) -> std::io::Result<()> {
    queue!(stdout, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;
    for (index, quote_char) in quote.iter().enumerate() {
        if let Some(char) = chars_typed.get(index) {
            let color = if char == quote_char {
                style::Color::Green
            } else {
                style::Color::Red
            };
            queue!(stdout, style::SetForegroundColor(color))?;
        } else {
            queue!(stdout, style::ResetColor)?;
        };
        queue!(stdout, style::Print(quote_char))?;
    }

    let (cols, _rows) = terminal::size()?;
    let cursor_col = chars_typed.len() as u16 % cols;
    let cursor_row = chars_typed.len() as u16 / cols;
    queue!(stdout, cursor::MoveTo(cursor_col, cursor_row))?;
    stdout.flush()?;
    Ok(())
}

fn terminate_session(stdout: &mut Stdout) -> std::io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(stdout, style::ResetColor)?;
    println!("\n");
    Ok(())
}

fn print_char(
    stdout: &mut Stdout,
    quote: &[char],
    chars_typed: &[char],
) -> std::io::Result<()> {
    let len = chars_typed.len();
    let c = quote[len - 1];
    let color = if &c == chars_typed.last().unwrap() {
        style::Color::Green
    } else {
        style::Color::Red
    };

    execute!(stdout, style::SetForegroundColor(color), style::Print(c))
}

