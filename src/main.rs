mod app;
mod claude;
mod detect;
mod hook;
mod rewrite;
mod setup;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, IsTerminal};
use tokio::sync::mpsc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Setup / uninstall
    if args.iter().any(|a| a == "--setup") {
        return setup::setup();
    }
    if args.iter().any(|a| a == "--uninstall") {
        return setup::uninstall();
    }
    if args.iter().any(|a| a == "--update") {
        return setup::update();
    }

    // Toggle commands
    if args.iter().any(|a| a == "--disable") {
        hook::disable();
        return Ok(());
    }
    if args.iter().any(|a| a == "--enable") {
        hook::enable();
        return Ok(());
    }

    // Test mode: --test "prompt text"
    if let Some(pos) = args.iter().position(|a| a == "--test") {
        let prompt = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let detection = detect::analyze(prompt);
        if detection.is_vague {
            eprintln!("BLOCK (score: {})", detection.score);
        } else {
            eprintln!("ALLOW (score: {})", detection.score);
        }
        return Ok(());
    }

    // Hook mode: --hook flag or stdin is piped
    if args.iter().any(|a| a == "--hook") || !io::stdin().is_terminal() {
        return hook::run();
    }

    // TUI mode
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_tui())
}

async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<claude::Message>();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        while let Ok(msg) = rx.try_recv() {
            app.handle_message(msg);
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.handle_key(key, &tx) {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }
}
