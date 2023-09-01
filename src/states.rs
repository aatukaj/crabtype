use std::io::Stdout;

use crossterm::event;
use ratatui::{Frame, prelude::CrosstermBackend};

use crate::App;

pub trait State {
    fn handle_event(self: Box<Self>, event: event::KeyEvent, app: &mut App) -> Box<dyn State>;
    fn update(self: Box<Self>, app: &mut App) -> Box<dyn State>;
    fn render(&mut self, f: &mut Frame<Backend>, app: &mut App);
}

pub type Backend = CrosstermBackend<Stdout>;
mod typing;
pub use typing::*;
mod stats;
pub use stats::*;
