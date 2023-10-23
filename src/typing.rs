#![allow(dead_code, unused_imports)]
use std::{
    fs::File,
    io::{self, Stdout, Write},
    time::Instant,
};

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    queue,
    style::{self, style},
    terminal,
};

use crate::{quote::get_quote, terminate_session, write_to_terminal, Quote, Stats, get_number_input, clear_screen_and_print};
use log::debug;

/// The the type of typing test
#[derive(Copy, Clone)]
pub enum SessionType {
    /// One quote
    SingleQuote,
    /// One or more quotes determined by the user
    MultiQuote(u16),
    /// Lasts for a specified number of seconds
    Time(u16),
    /// Goes forever
    Zen,
}

pub enum SessionOptions {
    StatsOn,
    StatsOff,
}


const SESSION_REQUEST_INFO: &str =
"Press character to choose mode:

enter: Previous Settings (Defaults to Single Quote)       
    s: Single Quote
    m: Multi Quote
    t: Timed Mode
    z: Zen Mode -- Press Ctr-f to finish
    q: Quit
";

pub fn get_session_type(out: &mut Stdout, previous: SessionType) -> io::Result<Option<SessionType>> {
    loop {
        clear_screen_and_print(out, SESSION_REQUEST_INFO, true)?;
        let Event::Key(key_event) = read()? else {
            continue;
        };

        let session_type = match key_event.code {
            KeyCode::Enter => previous,
            KeyCode::Char('q') | KeyCode::Char('c') => return Ok(None),
            KeyCode::Char('s') => SessionType::SingleQuote,
            KeyCode::Char('z') => SessionType::Zen,
            KeyCode::Char('m') => {
                let num = get_number_input(out)?;
                SessionType::MultiQuote(num)
            }
            KeyCode::Char('t') => {
                let num = get_number_input(out)?;
                SessionType::Time(num)
            }
            _ => continue,
        };

        return Ok(Some(session_type));
    }
}

pub async fn typing_session(
    session_type: SessionType,
    out: &mut Stdout,
) -> Result<(), Box<dyn std::error::Error>> {
    clear_screen_and_print(out, "", false)?;
    let mut state = TypingState::new(session_type, out, get_quote().await?);
    state.print_to_terminal()?;
    let mut next_quote = None;

    loop {
        let key_event = match read()? {
            Event::Key(key) => key,
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

        state.print_to_terminal()?;
    }

    let results = state.stats.analysis_str("Press enter to continue");
    clear_screen_and_print(out, &results, true)?;
    // wait for user to press enter
    loop {
        let key_event = match read()? {
            Event::Key(key) => key,
            _ => continue,
        };

        if key_event.code == KeyCode::Enter {
            break;
        }
    }
    Ok(())
}

pub struct TypingState<'a> {
    session_type: SessionType,
    quote_num: u32,
    out: &'a mut Stdout,
    quote_chars: Vec<char>,
    typed_chars: Vec<char>,
    control_flow: ControlFlow,
    start_time: Instant,
    stats: Stats,
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
            stats: Stats::new(session_type),
        }
    }

    fn update_control_flow(&mut self) -> ControlFlow {
        if self.control_flow == ControlFlow::Normal && self.requests_quote() {
            self.control_flow = ControlFlow::RequestsQuote;
        }

        if self.is_finished() {
            self.control_flow = ControlFlow::Finished;
        }

        self.control_flow
    }

    fn is_finished(&self) -> bool {
        if let SessionType::Time(x) = self.session_type {
            return self.start_time.elapsed().as_secs() >= x as u64;
        }
        return self.typed_chars.len() == self.quote_chars.len();
    }

    fn get_control_flow(&self) -> ControlFlow {
        self.control_flow
    }

    fn print_to_terminal(&mut self) -> io::Result<()> {
        write_to_terminal(
            &mut self.out,
            &self.quote_chars,
            &self.typed_chars,
            Some(self.stats),
        )
    }

    fn on_key_event(&mut self, key_event: KeyEvent) -> io::Result<ControlFlow> {
        // We do not care about when the key is released
        if key_event.kind == KeyEventKind::Release {
            return Ok(self.control_flow);
        }

        match key_event.code {
            // Exit on ctr-c and set control flow to exit
            KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL
                => self.control_flow = ControlFlow::Exit,
            // Print characters typed
            KeyCode::Char(c) => self.typed_chars.push(c),
            // On backspace do some stuff
            KeyCode::Backspace => { self.typed_chars.pop(); }
            _ => (),
        };

        self.stats.update(
            &self.quote_chars,
            &self.typed_chars,
            self.quote_num,
            self.start_time.elapsed().as_secs_f32(),
        );

        Ok(self.update_control_flow())
    }

    fn add_quote(&mut self, quote: Quote) {
        self.control_flow = ControlFlow::Normal;
        self.quote_num += 1;
        // add new space
        self.quote_chars.push(' ');
        self.quote_chars.extend(quote.content_chars());
    }

    /// Checks if another quote is needed to continue the session. It assumes
    /// that the main loop will provide the quote at a later time using the
    /// `add_quote()` function
    fn requests_quote(&self) -> bool {
        let does_request = match self.session_type {
            SessionType::SingleQuote => false,
            SessionType::MultiQuote(x) => self.quote_num < x as u32,
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
