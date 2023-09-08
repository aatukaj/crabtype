use std::{fs, io, time::Duration, path::Path, borrow::Cow};

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}, style::ContentStyle,
};
use rand::{distributions::uniform::SampleRange, seq::SliceRandom};
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
    #[arg(long)]
    words_file: Option<String>,
    #[arg(short, long)]
    punctuate: bool,
    #[arg(long, short)]
    seed: Option<u64>,
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
            Quotes => '\'',
        }
    }
}
use strum::EnumIter;
use PunctuationKind as PK;

fn punctuate<R: Rng, S: SampleRange<usize> + Clone>(
    words: Vec<String>,
    jump_range: S,
    rand: &mut R,
) -> Vec<String> {
    let mut capitalize_next = true;
    let mut new_words = Vec::new();

    let punctuations: Vec<PK> = PK::iter().collect();
    let weights = [3, 2, 2, 2, 2, 1, 2, 2, 2];
    debug_assert_eq!(punctuations.len(), weights.len());

    let dist = rand::distributions::WeightedIndex::new(weights).unwrap();

    let mut next_index = rand.gen_range(jump_range.clone());

    for (i, mut word) in words.into_iter().enumerate() {
        if capitalize_next {
            capitalize_next = false;
            word[0..1].make_ascii_uppercase();
        }
        if i == next_index {
            next_index += rand.gen_range(jump_range.clone());
            let pk = punctuations[dist.sample(rand)];
            let c: char = pk.into();
            match pk {
                PK::Exclamation | PK::Period => {
                    capitalize_next = true;
                    word.push(c)
                }
                PK::DQuotes | PK::Quotes => word = format!("{c}{word}{c}"),
                PK::Parantheses => word = format!("({word})"),
                PK::Hyphen => new_words.push(c.into()),
                _ => word.push(c),
            }
        }
        new_words.push(word);
    }
    new_words
}

fn main() -> Result<()> {
    

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let args: Cli = Cli::parse();

    let contents: Cow<'_, str> = match args.words_file {
        Some(path) =>  {let c = fs::read_to_string(Path::new(&path))?; c.into()},
        None => include_str!("../words/english_1k.json").into(),
    };

    let mut word_list = serde_json::from_str::<WordList>(&contents)?;

    let seed = args.seed.unwrap_or(thread_rng().gen());
    let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
    word_list.words.shuffle(&mut rng);
    if args.punctuate {
        word_list.words = punctuate(word_list.words, 2..=4, &mut rng);
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
    println!("seed:");
    println!("{}", seed);
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
