use colored::*;
use quote_typer::{
    initialize_session,
    typing::{typing_session, SessionType},
};
use std::{
    fs::File,
    io::{Stdout, Write}, time::Duration,
};

fn true_on_enter(out: &mut Stdout) -> bool {
    let mut input = String::new();
    out.write("\nPress Enter for another".as_bytes()).unwrap();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input == "\n"
}

fn print_analysis(quote_chars: &[char], typed_chars: &[char], elapsed_time: Duration) {
    let num_correct = std::iter::zip(quote_chars.iter(), typed_chars.iter())
        .filter(|(q, t)| q == t)
        .count();

    let elapsed = elapsed_time.as_secs_f32();
    let total_str = quote_chars.len().to_string().color(Color::Blue);
    let num_correct_str = num_correct.to_string().color(Color::Green);
    let mistakes_str = (quote_chars.len() - num_correct)
        .to_string()
        .color(Color::Red);
    println!(
        "Total: {}, Correct: {}, Mistakes: {}",
        total_str, num_correct_str, mistakes_str
    );
    println!("CPM: {}", 60.0 * num_correct as f32 / elapsed);
    println!("WPM: {}", 60.0 * num_correct as f32 / elapsed / 5.0);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log_file = File::create("log.txt")?;
    let mut log_file2 = File::create("log2.txt")?;
    env_logger::builder()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(log::LevelFilter::Debug)
        .init();
    let mut out = std::io::stdout();
    initialize_session(&mut out)?;
    loop {
        typing_session(SessionType::MultiQuote(4), &mut out, &mut log_file2).await?;
        // print_analysis(&quote.content_chars(), &typed_chars, start.elapsed());

        // if !true_on_enter(&mut out) {
        //     break;
        // }
    }
    // terminate_session(&mut out)?;
    // Ok(())
}
