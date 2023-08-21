use std::{os::raw, time::Duration};

use crossterm::event;
use ratatui::{
    prelude::*,
    widgets::{Axis, Chart, Dataset, GraphType, List, ListItem, Paragraph, Wrap},
};

use crate::App;

use super::{Backend, KeyStrokeKind, State};

pub struct StatsState {
    raw_wpms: Vec<(f64, f64)>,
    correct_wpms: Vec<(f64, f64)>,
    errors_wpms: Vec<(f64, f64)>,
    key_strokes: Vec<(Duration, KeyStrokeKind)>,
    wpm: f64,
    raw_wpm: f64,
    accuracy: f64,
}
impl StatsState {
    pub fn new(key_strokes: Vec<(Duration, KeyStrokeKind)>, test_duration: Duration) -> Self {
        let time_step = 0.5;
        let mut raw_stats = (0..=(test_duration.as_secs_f64() / time_step) as u32)
            .map(|i| (i as f64 * time_step, 0.0))
            .collect::<Vec<_>>();
        let mut correct_stats = raw_stats.clone();
        let mut error_stats = raw_stats.clone();
        let l = raw_stats.len();
        for (duration, key_stroke) in key_strokes.iter() {
            let index = ((duration.as_secs_f64() / time_step).ceil() as usize).min(l - 1);
            raw_stats[index].1 += 1.0;
            match key_stroke {
                KeyStrokeKind::Incorrect(_) => error_stats[index].1 += 1.0,
                _ => correct_stats[index].1 += 1.0,
            }
        }
        calc_wpm(&mut raw_stats, time_step);
        calc_wpm(&mut correct_stats, time_step);
        calc_wpm(&mut error_stats, time_step);
        let mut errs = 0.0;
        let mut corrects = 0.0;
        let mut spaces = 0.0;
        for (_, key_stroke) in key_strokes.iter() {
            match key_stroke {
                KeyStrokeKind::Incorrect(_) => errs += 1.0,
                KeyStrokeKind::Correct(_) => corrects += 1.0,
                KeyStrokeKind::Space => spaces += 1.0,
                _ => ()
            }
        }
        Self {
            raw_wpms: avarage_out_wpms(&raw_stats),
            correct_wpms: avarage_out_wpms(&correct_stats),
            errors_wpms: error_stats
                .into_iter()
                .filter(|(_, count)| count > &0.0)
                .collect(), // do not want to avarage the errors
            key_strokes,
            wpm: (corrects + spaces) * (60.0 / test_duration.as_secs_f64()) / 5.0,
            raw_wpm: (corrects + spaces + errs) * (60.0 / test_duration.as_secs_f64()) / 5.0,
            accuracy: corrects / (corrects + errs),
        }
    }
}

fn calc_wpm(raw_stats: &mut Vec<(f64, f64)>, time_step: f64) {
    for (_, wpm) in raw_stats.iter_mut() {
        *wpm = *wpm * 60.0 / time_step / 5.0;
    }
}
fn wpm_slice_avg(slice: &[(f64, f64)]) -> (f64, f64) {
    (
        slice[0].0,
        slice.iter().map(|w| w.1).sum::<f64>() / slice.len() as f64,
    )
}

fn avarage_out_wpms(wpms: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let window_size = 4;
    let mut stats = Vec::new();
    for window in wpms.windows(window_size) {
        stats.push(wpm_slice_avg(window))
    }
    for i in (0..window_size).rev() {
        stats.push(wpm_slice_avg(&wpms[(wpms.len() - 1 - i)..]))
    }
    stats
}
impl State for StatsState {
    fn handle_event(self: Box<Self>, _event: event::KeyEvent, _app: &mut App) -> Box<dyn State> {
        self
    }
    fn update(self: Box<Self>, _app: &mut App) -> Box<dyn State> {
        self
    }
    fn render(&mut self, f: &mut Frame<Backend>, _app: &mut App) {
        let max_wpm = (self
            .raw_wpms
            .iter()
            .max_by(|l, r| l.1.total_cmp(&r.1))
            .map(|&(_, wpm)| wpm as usize)
            .unwrap_or(0)
            / 40
            + 1)
            * 40;
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Max(10), Constraint::Min(0)])
            .split(f.size());
        let last_time = self.raw_wpms.last().unwrap_or(&(0.0, 0.0)).0.floor();

        let chart = Chart::new(vec![
            Dataset::default()
                .graph_type(GraphType::Line)
                .data(&self.raw_wpms[0..])
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::DarkGray)),
            Dataset::default()
                .graph_type(GraphType::Line)
                .data(&self.correct_wpms[0..])
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Yellow)),
            Dataset::default()
                .graph_type(GraphType::Scatter)
                .data(&self.errors_wpms[0..])
                .marker(symbols::Marker::Bar)
                .style(Style::default().fg(Color::Red)),
        ])
        .y_axis(
            Axis::default()
                .bounds([0f64, max_wpm as f64])
                .title("wpm")
                .labels(
                    (0..=max_wpm)
                        .step_by(40)
                        .map(|i| Span::from(format!("{}", i)))
                        .collect(),
                )
                .labels_alignment(Alignment::Left),
        )
        .x_axis(
            Axis::default()
                .bounds([1f64, last_time])
                .title("time (s)")
                .labels(
                    vec![1.0, last_time / 2.0, last_time]
                        .iter()
                        .map(|i| Span::from(format!("{i}")))
                        .collect(),
                ),
        );
        f.render_widget(chart, layout[1]);
        let stats = [
            ("wpm", format!("{:.0}", self.wpm)),
            ("raw", format!("{:.0}", self.raw_wpm)),
            ("acc", format!("{:.0}%", self.accuracy * 100.0)),
        ];
        let t = stats.map(|(name, value)| {
            ListItem::new(vec![
                Line::from(Span::styled(name, Style::default().yellow())),
                Line::from(Span::raw(value)),
                Line::from(""),
            ])
        });
        let list = List::new(t.to_vec());
        f.render_widget(list, layout[0])
    }
}
