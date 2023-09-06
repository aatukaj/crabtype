use std::{fs, io, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use ratatui::prelude::*;
use serde::Deserialize;
use strum::IntoEnumIterator;
mod typingwidget;

mod states;
use states::*;

use clap::Args;
use clap::Parser;

use rand::prelude::*;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    mode: Mode,
    #[arg(long, default_value_t={"english_1k".to_string()})]
    words_file: String,
}

#[derive(Args, Debug)]
#[group(multiple = false)]
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
    words: Vec<String>,
}
pub struct App {
    word_list: WordList,
    state: Option<Box<dyn State>>,
}


#[derive(EnumIter, Clone, Copy, PartialEq)]
enum PunctuationKind {
    Period,
    Comma,
    Hyphen,
    Parantheses,
    Exclamation,
    Semicolon,
    Colon,
    DQuotes,
    Quotes,

}
impl Into<char> for PunctuationKind {
    fn into(self) -> char {
        use PunctuationKind::*;
        match self {
            Period => '.',
            Comma => ',',
            Hyphen => '-',
            Parantheses => ')',
            Exclamation => '!',
            Semicolon => ';',
            Colon => ':',
            DQuotes => '"',
            Quotes => '\''
        }
    }
}
use PunctuationKind as PK;
use strum::EnumIter;

fn main() -> Result<()> {
    let args: Cli = Cli::parse();

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;



    let contents = fs::read_to_string(format!("./words/{}.json", args.words_file))?;
    let mut word_list = serde_json::from_str::<WordList>(&contents)?;

    let punctuations: Vec<PK> = PK::iter().collect();
    let weights = [3, 2, 2, 2, 2, 1, 2, 2, 2];
    debug_assert_eq!(punctuations.len(), weights.len());
    word_list.words.shuffle(&mut rand::thread_rng());
    let dist = rand::distributions::WeightedIndex::new(weights).unwrap();

    println!("{:?}", dist.sample(&mut rand::thread_rng()));
    let words = &mut word_list.words;
    let mut i = 1;
    while i < words.len() {
        let pk = punctuations[dist.sample(&mut rand::thread_rng())];
        let c: char = pk.into();
        if pk == PK::Hyphen {
            words.insert(i, String::from("-"))
        } else {
            words[i].push(c);
        }
        
        match pk {
            PK::Exclamation | PK::Period => words[i+1][0..1].make_ascii_uppercase(),
            PK::DQuotes | PK::Quotes => words[i].insert(0, c),
            PK::Parantheses => words[i].insert(0, '('),
            _ => (),
        }
        i+=2;
    }
    


    let mode = match args.mode {
        Mode {
            words: Some(words), ..
        } => TestMode::Words(words),
        Mode {
            duration: Some(duration),
            ..
        } => TestMode::Duration(Duration::from_secs(duration)),
        _ => TestMode::Duration(Duration::from_secs(30)),
    };

    

    let app = App {
        state: Some(Box::new(TypingState::new(word_list.words.clone(), mode))),
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
            KeyCode::Esc => true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            _ => false,
        }
    } else {
        false
    }
}

fn ui(f: &mut Frame<states::Backend>, app: &mut App) {
    if let Some(mut state) = app.state.take() {
        state.render(f, app);
        app.state = Some(state)
    }
}
