use quote_typer::{
    initialize_session,
    typing::{typing_session, SessionType, get_session_type}, terminate_session,
};
use std:: fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log_file = File::create("log.txt")?;
    env_logger::builder()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(log::LevelFilter::Debug)
        .init();
    let mut out = std::io::stdout();
    
    let mut session_type = SessionType::SingleQuote;
    loop {
        initialize_session(&mut out)?;
        match get_session_type(&mut out, session_type)? {
            None => break,
            Some(st) => session_type = st,
        };
        typing_session(session_type, &mut out).await?;
    }

    terminate_session(&mut out)?;
    Ok(())
}
