#![allow(dead_code)]
use std::{
    fs::File,
    io::{self, Stdout, Write},
    pin::Pin,
    sync::Arc,
    time::Instant,
};

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    queue,
    style::{self, style},
    terminal,
};

use crate::{on_backspace, quote::get_quote, reset_session, terminate_session, Quote};

// TODO: keep stuff in single terminal
// TODO: watch out for cursor getting messed up in last column
// TODO: remove colors when pressing backspace

/// The the type of typing test
pub enum SessionType {
    /// One quote
    SingleQuote,
    /// One or more quotes determined by the user
    MultiQuote(u16),
    /// Lasts for a specified number of seconds
    Time(u32),
    /// Goes forever
    Zen,
}

pub async fn typing_session(
    session_type: SessionType,
    out: &mut Stdout,
    log_file: &mut File,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = TypingState::new(session_type, out, get_quote().await?);
    let mut next_quote = None;

    loop {
        let event = read()?;
        let buf = format!("{:?}\n", event);
        log_file.write_all(buf.as_bytes())?;

        let key_event = match event {
            Event::Key(key) => key,
            Event::Resize(c, r) => {
                state.on_resize(c, r)?;
                continue;
            }
            _ => continue,
        };

        match state.on_key_event(key_event)? {
            ControlFlow::Normal => (),
            ControlFlow::RequestsQuote => {
                next_quote = Some(tokio::spawn(get_quote()));
                state.getting_next_quote();
            }
            ControlFlow::WaitingForQuote => {
                if let Some(future_quote) = next_quote.take() {
                    state.add_quote(future_quote.await??);
                }
            }
            ControlFlow::Finished => {
                break;
            }
            ControlFlow::Exit => {
                terminate_session(out)?;
                println!("User exited program");
                std::process::exit(130);
            }
        };
    }

    Ok(())
}

pub struct TypingState<'a> {
    session_type: SessionType,
    quote_num: u16,
    start_time: Instant,
    out: &'a mut Stdout,
    quote_chars: Vec<char>,
    typed_chars: Vec<char>,
    control_flow: ControlFlow,
}

const CHARS_TILL_NEXT_QUOTE: usize = 15;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlFlow {
    Exit,
    Normal,
    Finished,
    RequestsQuote,
    WaitingForQuote,
}

impl<'a> TypingState<'a> {
    fn new(session_type: SessionType, out: &'a mut Stdout, quote: Quote) -> Self {
        Self {
            session_type,
            out,
            control_flow: ControlFlow::Normal,
            quote_chars: quote.content_chars(),
            typed_chars: Vec::new(),
            quote_num: 1,
            start_time: Instant::now(),
        }
    }

    fn update_control_flow(&mut self) -> ControlFlow {
        if self.control_flow == ControlFlow::Normal && self.requests_quote() {
            self.control_flow = ControlFlow::RequestsQuote;
        }

        if self.typed_chars.len() == self.quote_chars.len() {
            self.control_flow = ControlFlow::Finished
        }

        self.control_flow
    }

    fn get_control_flow(&self) -> ControlFlow {
        self.control_flow
    }

    /// Basically iterates over all of the key presses to see what is correct
    fn on_resize(&mut self, cols: u16, _rows: u16) -> io::Result<()> {
        queue!(
            self.out,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        for (index, quote_char) in self.quote_chars.iter().enumerate() {
            if let Some(char) = self.typed_chars.get(index) {
                let color = if char == quote_char {
                    style::Color::Green
                } else {
                    style::Color::Red
                };
                queue!(self.out, style::SetForegroundColor(color))?;
            } else {
                queue!(self.out, style::ResetColor)?;
            };
            queue!(self.out, style::Print(quote_char))?;
        }

        let cursor_col = self.typed_chars.len() as u16 % cols;
        let cursor_row = self.typed_chars.len() as u16 / cols;
        queue!(self.out, cursor::MoveTo(cursor_col, cursor_row))?;
        self.out.flush()?;
        Ok(())
    }

    fn on_key_event(&mut self, key_event: KeyEvent) -> io::Result<ControlFlow> {
        // We do not care about when the key is released
        if key_event.kind == KeyEventKind::Release {
            return Ok(self.control_flow);
        }

        match key_event {
            // Exit on ctr-c and set control flow to exit
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.control_flow = ControlFlow::Exit,
            // Print characters typed
            KeyEvent {
                code: KeyCode::Char(c),
                ..
            } => {
                self.typed_chars.push(c);
                reset_session(&mut self.out, &self.quote_chars, &self.typed_chars)?;
            }
            // Go to new line on enter
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                self.typed_chars.push('\n');
                reset_session(&mut self.out, &self.quote_chars, &self.typed_chars)?;
            }
            // On backspace do some stuff
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                self.typed_chars.pop();
                on_backspace(&mut self.out)?;
            }
            _ => (),
        };

        Ok(self.update_control_flow())
    }

    fn add_quote(&mut self, quote: Quote) {
        self.control_flow = ControlFlow::Normal;
        self.quote_num += 1;
        self.quote_chars.extend(quote.content_chars());
    }

    /// Checks if another quote is needed to continue the session. It assumes
    /// that the main loop will provide the quote at a later time using the
    /// `add_quote()` function
    fn requests_quote(&self) -> bool {
        let does_request = match self.session_type {
            SessionType::SingleQuote => false,
            SessionType::MultiQuote(x) => self.quote_num < x,
            SessionType::Zen => true,
            SessionType::Time(x) => self.start_time.elapsed().as_secs_f32() < x as f32,
        };

        let chars_left = self.quote_chars.len() - self.typed_chars.len();
        does_request && chars_left <= CHARS_TILL_NEXT_QUOTE
    }

    /// Lets the state know that the next quote is being processed
    /// changes the control_flow to `WaitingForQuote`
    fn getting_next_quote(&mut self) {
        self.control_flow = ControlFlow::WaitingForQuote;
    }
}
