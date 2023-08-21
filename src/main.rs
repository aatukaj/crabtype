use std::{io, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use ratatui::prelude::*;
use serde::Deserialize;

mod typingwidget;

mod states;
use states::*;

use clap::Parser;
use clap::Args;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    mode: Mode,
    #[arg(long, default_value_t={"english_1k.json".to_string()})]
    words_file: String,
}

#[derive(Args, Debug)]
#[group(multiple=false)]
struct Mode {
    #[arg(long, short)]
    words: Option<usize>,
    #[arg(long, short)]
    duration: Option<u64>,
}
#[derive(Deserialize, Debug)]
#[allow(unused)]
struct WordList {
    name: String,

    #[serde(rename = "noLazyMode")]
    no_lazy_mode: bool,
    #[serde(rename = "orderedByFrequency")]
    ordered_by_frequency: bool,
    words: Vec<String>,
}
pub struct App {

    word_list: WordList,
    state: Option<Box<dyn State>>,
}

fn main() -> Result<()> {
    let args: Cli = Cli::parse();

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut word_list = serde_json::from_str::<WordList>(include_str!("../words/english_1k.json"))?;
    let mode = match args.mode {
        Mode {words: Some(words), ..} => TestMode::Words(words),
        Mode {duration: Some(duration), ..} => TestMode::Duration(Duration::from_secs(duration)),
        _ => unreachable!(),
    };

    word_list.words.shuffle(&mut rand::thread_rng());
    let app = App {
        state: Some(Box::new(TypingState::new(
            word_list.words.clone(),
            mode,
        ))),
        word_list,
    };

    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<states::Backend>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Some(Event::Key(key)) = event::poll(Duration::from_millis(16))
            .and_then(|polled| polled.then(event::read).transpose())?
        {
            if handle_event(key, &mut app) {
                break;
            }
            app.state = Some(app.state.take().unwrap().handle_event(key, &mut app))
        }
        app.state = Some(app.state.take().unwrap().update(&mut app))
    }

    Ok(())
}

fn handle_event(key: event::KeyEvent, _app: &mut App) -> bool {
    if key.kind == KeyEventKind::Press {
        match key.code {
            KeyCode::Esc => return true,
            _ => (),
        }
    }
    false
}

fn ui(f: &mut Frame<states::Backend>, app: &mut App) {
    if let Some(mut state) = app.state.take() {
        state.render(f, app);
        app.state = Some(state)
    }
}
