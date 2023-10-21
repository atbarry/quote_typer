use std::time::{Instant, Duration};

use quote_typer::{quote::get_quote, typing_session};
use crossterm::{execute, terminal};
use colored::*;

fn true_on_enter() -> bool {
    std::thread::sleep(Duration::from_millis(500));
    let mut input = String::new();
    println!("\nPress Enter for another");
    std::io::stdin().read_line(&mut input)
        .expect("Failed to read line");

    input == "\n"
}

fn print_analysis(quote_chars: &[char], typed_chars: &[char], elapsed_time: Duration) {
    let num_correct = std::iter::zip(
        quote_chars.iter(),
        typed_chars.iter()
        ).filter(|(q, t)| q == t)
        .count();

    let elapsed =  elapsed_time.as_secs_f32();
    let total_str = quote_chars.len().to_string().color(Color::Blue);
    let num_correct_str = num_correct.to_string().color(Color::Green);
    let mistakes_str = (quote_chars.len() - num_correct).to_string().color(Color::Red);
    println!("Total: {}, Correct: {}, Mistakes: {}", total_str, num_correct_str, mistakes_str);
    println!("CPM: {}", 60.0 * num_correct as f32 / elapsed );
    println!("WPM: {}", 60.0 * num_correct as f32 / elapsed / 5.0 );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut quote = get_quote().await?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    loop {
        let start = Instant::now();
        let next_quote = get_quote();
        let typed_chars = typing_session(&mut stdout, &quote)?;
        print_analysis(&quote.content_chars(), &typed_chars, start.elapsed());

        if !true_on_enter() {
            break;
        }

        quote = next_quote.await?;
    }
    execute!(stdout, terminal::LeaveAlternateScreen)?;
    Ok(())
}
