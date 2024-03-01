use std::time::{Duration, Instant};

use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};

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
    ///amount of extra letters in the word before, when n < 0, skipped letters
    Space(i32),
}

pub enum TestMode {
    Duration(Duration),
    Words(usize),
}
impl TypingState {
    pub fn new(mut word_list: Vec<String>, mode: TestMode) -> Self {
        Self {
            written_words: vec![String::new()],
            start_time: None,
            rows: vec![0],
            word_list: if let TestMode::Words(words) = mode {
                word_list.resize(words, String::new());
                word_list
            } else {
                word_list
            },
            key_strokes: Vec::new(),
            mode,
        }
    }

    fn remove_empty(&mut self) {
        if self.written_words.len() > 1 {
            if self.written_words[self.written_words.len() - 2]
                != self.word_list[self.written_words.len() - 2]
            {
                self.written_words.pop();
            }
        }
    }

    fn remove_char(&mut self) {
        let Some(last) = self.written_words.last_mut() else {
            return;
        };
        if last.is_empty() {
            self.remove_empty()
        } else {
            last.pop();
        }
    }
    fn remove_word(&mut self) {
        let Some(last) = self.written_words.last_mut() else {
            return;
        };
        if last.is_empty() {
            self.remove_empty();
            self.written_words.last_mut().unwrap().clear();
        } else {
            last.clear();
        }
    }

    fn add_char(&mut self, c: char, time: Instant) {
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

    fn add_space(&mut self, time: Instant) {
        let i = self.written_words.len() - 1;
        self.key_strokes.push((
            time.elapsed(),
            KeyStrokeKind::Space(
                self.written_words[i].len() as i32 - self.word_list[i].len() as i32,
            ),
        ));
        self.written_words.push(String::new());
    }
}
impl State for TypingState {
    fn handle_event(mut self: Box<Self>, event: event::KeyEvent, _app: &App) -> Box<dyn State> {
        if event.kind == KeyEventKind::Press {
            // start counting the time on the first event
            let time = *self.start_time.get_or_insert_with(|| Instant::now());
            match event.code {
                KeyCode::Char('w') | KeyCode::Backspace if event.modifiers.contains(KeyModifiers::CONTROL)  => {self.remove_word()},
                KeyCode::Char(c @ ('!'..='~' /* https://www.asciitable.com/ */)) => self.add_char(c, time),
                KeyCode::Char(' ') => self.add_space(time),
                KeyCode::Backspace => self.remove_char(),
                _ => (),
            };
        }
        self
    }
    fn update(self: Box<Self>, _app: &App) -> Box<dyn State> {
        if let Some(start_time) = self.start_time {
            match self.mode {
                TestMode::Duration(dur) => {
                    if start_time.elapsed() > dur {
                        return Box::new(StatsState::new(
                            self.key_strokes,
                            dur,
                            &self.word_list,
                            &self.written_words,
                            self.mode
                        ));
                    }
                }
                TestMode::Words(words) => {
                    if self.written_words.len() > words {
                        return Box::new(StatsState::new(
                            self.key_strokes,
                            start_time.elapsed(),
                            &self.word_list,
                            &self.written_words,
                            self.mode
                        ));
                    }
                }
            }
        }
        self
    }
    fn render(&mut self, f: &mut ratatui::Frame<Backend>, _app: &App) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Max(1), Constraint::Min(0)])
            .vertical_margin(1)
            .split(f.size());

        let (ratio, label) = match self.mode {
            TestMode::Duration(d) => (
                self.start_time
                    .map_or(Duration::ZERO, |t| t.elapsed())
                    .as_secs_f64()
                    / d.as_secs_f64(),
                self.start_time
                    .map_or("Start Typing to begin.".to_string(), |duration| {
                        format!(
                            "{:.1}/{:.1}s",
                            duration.elapsed().as_secs_f32(),
                            d.as_secs_f32()
                        )
                    }),
            ),
            TestMode::Words(words) => (
                (self.written_words.len() - 1) as f64 / words as f64,
                if self.start_time.is_none() {
                    "Start Typing to begin.".to_string()
                } else {
                    format!("{}/{}", (self.written_words.len() - 1), words)
                },
            ),
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
