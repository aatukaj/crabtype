use std::time::{Duration, Instant};

use crossterm::event::{self, KeyCode, KeyEventKind};

use crate::{typingwidget::TypingWidget, App};

use super::{Backend, State, StatsState};
use ratatui::{prelude::*, widgets::Gauge};

pub struct TypingState {
    pub written_words: Vec<String>,
    start_time: Option<Instant>,
    pub rows: Vec<usize>,
    pub word_list: Vec<String>,
    key_strokes: Vec<(Duration, KeyStrokeKind)>, //(time of keystroke, kind)
    mode: TestMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStrokeKind {
    Correct(char),
    Incorrect(char),
    Space,
    Remove,
}
pub enum TestMode {
    Duration(Duration),
    Words(usize),
}
impl TypingState {
    pub fn new(word_list: Vec<String>, mode: TestMode) -> Self {
        Self {
            written_words: vec![String::new()],
            start_time: None,
            rows: vec![0],
            word_list,
            key_strokes: Vec::new(),
            mode,
        }
    }
}
impl State for TypingState {
    fn handle_event(mut self: Box<Self>, event: event::KeyEvent, _app: &mut App) -> Box<dyn State> {
        if event.kind == KeyEventKind::Press {
            let time = match self.start_time {
                Some(time) => time,
                None => {
                    let time = Instant::now();
                    self.start_time = Some(time);
                    time
                }
            };
            match event.code {
                KeyCode::Char(c @ ('a'..='z' | 'A'..='Z')) => {
                    if let Some(s) = self.written_words.last_mut() {
                        s.push(c);
                        let len = s.len();
                        self.key_strokes.push((
                            time.elapsed(),
                            match self.word_list[self.written_words.len() - 1]
                                .chars()
                                .nth(len - 1)
                                .is_some_and(|val| val == c)
                            {
                                true => KeyStrokeKind::Correct(c),
                                false => KeyStrokeKind::Incorrect(c),
                            },
                        ))
                    }
                }
                KeyCode::Char(' ') => {
                    self.written_words.push(String::new());
                    self.key_strokes
                        .push((time.elapsed(), KeyStrokeKind::Space))
                }
                KeyCode::Backspace => {
                    let last = self.written_words.last().unwrap();
                    if last.is_empty() {
                        self.written_words.pop();
                    } else {
                        self.written_words.last_mut().unwrap().pop();
                    }
                    self.key_strokes
                        .push((time.elapsed(), KeyStrokeKind::Remove));
                }
                _ => (),
            };
        }

        self
    }
    fn update(self: Box<Self>, _app: &mut App) -> Box<dyn State> {
        if let TestMode::Duration(d) = self.mode {
            if self.start_time.is_some_and(|t| t.elapsed() > d) {
                return Box::new(StatsState::new(self.key_strokes, d));
            }
        }

        self
    }
    fn render(&mut self, f: &mut ratatui::Frame<Backend>, _app: &mut App) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Max(1), Constraint::Min(0)])
            .vertical_margin(1)
            .split(f.size());

        let (ratio, label) = match self.mode {
            TestMode::Duration(d) => (
                self
                    .start_time
                    .map_or(Duration::ZERO, |t| t.elapsed())
                    .as_secs_f64()
                    / d.as_secs_f64()
  ,
                self.start_time
                    .map_or("Start Typing to begin.".to_string(), |duration| {
                        format!(
                            "{:.1}/{:.1}s",
                            duration.elapsed().as_secs_f32(),
                            d.as_secs_f32()
                        )
                    }),
            ),
            TestMode::Words(words) => {
                ((self.written_words.len() - 1) as f64 / words as f64, format!("{}/{}words", (self.written_words.len() - 1), words))
            }
        };
        let ratio = ratio.clamp(0.0, 1.0); // ratio thats not in 0..1.0 causes a panic
        let timer = Gauge::default()
            .ratio(ratio)
            .gauge_style(Style::default().yellow())
            .use_unicode(true)
            .label(label);

        let text_box_layout = Layout::new()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(30),
                Constraint::Max(3),
                Constraint::Percentage(40),
            ])
            .horizontal_margin(10)
            .split(layout[1]);
        f.render_widget(timer, layout[0]);
        f.render_stateful_widget(TypingWidget::new(), text_box_layout[1], self);
    }
}
