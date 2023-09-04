use std::time::Duration;

use crossterm::event;
use ratatui::{
    prelude::*,
    widgets::{Axis, Chart, Dataset, GraphType, List, ListItem},
};

use crate::App;

use super::{Backend, KeyStrokeKind, State};

use itertools::{EitherOrBoth, Itertools};

pub struct StatsState {
    raw_wpms: Vec<(f64, f64)>,
    //correct_wpms: Vec<(f64, f64)>,
    errors_wpms: Vec<(f64, f64)>,
    key_strokes: Vec<(Duration, KeyStrokeKind)>,
    accuracy: f64,
    test_duration: Duration,
    final_stats: FinalStats,
}

#[derive(PartialEq, Clone, Debug)]
struct FinalStats {
    wpm: f64,     // amount of characters in fully correct words + spaces normalized
    raw_wpm: f64, // wpm with incorrect words' characters
    correct: u32,
    incorrect: u32,
    extra: u32,
    missed: u32,
}

impl Default for FinalStats {
    fn default() -> Self {
        Self {
            wpm: 0.0,
            raw_wpm: 0.0,
            correct: 0,
            incorrect: 0,
            extra: 0,
            missed: 0,
        }
    }
}

impl FinalStats {
    fn calculate(
        inputted_words: &[String],
        correct_words: &[String],
        test_duration: Duration, //for normalizing wpm
    ) -> Self {
        let mut result = inputted_words
            .iter()
            .zip(correct_words.iter())
            .enumerate()
            .fold(Self::default(), |mut acc, (i, (input, correct))| {
                if input == correct {
                    acc.wpm += input.len() as f64 + 1.0
                } else if i == inputted_words.len() - 1 && input.len() <= correct.len() && input == &correct[0..input.len()] {
                    acc.wpm += input.len() as f64;
                    acc.raw_wpm -= 1.0;
                }
                acc.raw_wpm += input.len() as f64 + 1.0;
                for d in word_difference(if i != inputted_words.len() - 1{&correct} else {&correct[0..input.len().min(correct.len())]}, &input) {
                    match d {
                        CharDiffKind::Correct => acc.correct += 1,
                        CharDiffKind::Incorrect => acc.incorrect += 1,
                        CharDiffKind::Extra => acc.extra += 1,
                        CharDiffKind::Missed => acc.missed += 1,
                    }
                }
                acc
            });
        result.wpm = normalize_wpm(result.wpm, test_duration.as_secs_f64());
        result.raw_wpm = normalize_wpm(result.raw_wpm, test_duration.as_secs_f64());
        result
    }
}
impl StatsState {
    pub fn new(
        key_strokes: Vec<(Duration, KeyStrokeKind)>,
        test_duration: Duration,
        inputted_words: &[String],
        correct_words: &[String],
    ) -> Self {
        let time_step = (test_duration.as_secs_f64() / 20.0).max(0.5);
        let batched_ks = batch_key_strokes(&key_strokes, time_step);

        Self {
            raw_wpms: batched_ks
                .iter()
                .map(|t| (t.0, normalize_wpm(t.1, time_step)))
                .collect_vec(),
            errors_wpms: batched_ks
                .iter()
                .filter_map(|t| (t.2 != 0.0).then_some((t.0, normalize_wpm(t.2, time_step))))
                .collect_vec(),
            accuracy:calculate_accuracy(&key_strokes),
            key_strokes,
            test_duration,
            final_stats: FinalStats::calculate(inputted_words, correct_words, test_duration)
        }
    }

    fn render_stats(&self, f: &mut Frame<'_, CrosstermBackend<std::io::Stdout>>, area: Rect) {
        let stats = [
            ("wpm", format!("{:.0}", self.final_stats.wpm)),
            ("raw", format!("{:.0}", self.final_stats.raw_wpm)),
            ("acc", format!("{:.0}%", self.accuracy * 100.0)),
            ("chars", format!("correct:   {}\nincorrect: {}\nextra:     {}\nmissed:    {}", self.final_stats.correct, self.final_stats.incorrect, self.final_stats.extra, self.final_stats.missed)),
        ];
        let t = stats.map(|(name, value)| {
            ListItem::new({
                let mut it = vec![Line::from(Span::styled(name.to_string(), Style::default().yellow()))];
                for row in value.split('\n') {
                    it.push(Line::from(Span::raw(row.to_string())))
                }
                it.push(Line::from(""));
                it
            })
        });
        let list = List::new(t.to_vec());
        f.render_widget(list, area)
    }

    fn render_chart(&mut self, f: &mut Frame<'_, CrosstermBackend<std::io::Stdout>>, area: Rect) {
        let max_wpm = (self
            .raw_wpms
            .iter()
            .max_by(|l, r| l.1.total_cmp(&r.1))
            .map(|&(_, wpm)| wpm as usize)
            .unwrap_or(0)
            / 40
            + 1)
            * 40;
        
        let last_time = self.test_duration.as_secs_f64();

        let chart = Chart::new(vec![
            Dataset::default()
                .graph_type(GraphType::Line)
                .data(&self.raw_wpms[0..])
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::DarkGray)),
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
                    [1.0, (last_time / 2.0).round(), last_time.round()]
                        .iter()
                        .map(|i| Span::from(format!("{i}")))
                        .collect(),
                ),
        );
        f.render_widget(chart, area);
    }
}


impl State for StatsState {
    fn handle_event(self: Box<Self>, _event: event::KeyEvent, _app: &App) -> Box<dyn State> {
        self
    }
    fn update(self: Box<Self>, _app: &App) -> Box<dyn State> {
        self
    }
    fn render(&mut self, f: &mut Frame<Backend>, _app: &App) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Max(17), Constraint::Min(0)])
            .split(f.size());
        self.render_chart(f, layout[1]);
        self.render_stats(f, layout[0]);
    }
}

fn normalize_wpm(char_amount: f64, time: f64) -> f64 {
    char_amount / 5.0 * (60.0 / time)
}

#[allow(unused)]
#[derive(PartialEq, Clone, Debug)]
pub enum CharDiffKind {
    Correct,
    Incorrect,
    Extra,
    Missed,
}

pub fn word_difference<'a>(
    correct_word: &'a str,
    input: &'a str,
) -> impl Iterator<Item = CharDiffKind> + 'a {
    correct_word
        .chars()
        .zip_longest(input.chars())
        .map(|e| match e {
            EitherOrBoth::Left(_) => CharDiffKind::Missed,
            EitherOrBoth::Right(_) => CharDiffKind::Extra,
            EitherOrBoth::Both(c, i) => {
                if c == i {
                    CharDiffKind::Correct
                } else {
                    CharDiffKind::Incorrect
                }
            }
        })
}

fn calculate_accuracy(
    key_strokes: &[(Duration, KeyStrokeKind)]
) -> f64 {
    let mut correct = 0.0;
    let mut incorrect = 0.0;
    for (_, ks) in key_strokes.iter() {
        match ks {
            KeyStrokeKind::Correct(_) => correct+=1.0,
            KeyStrokeKind::Incorrect(_) => incorrect+=1.0,
            KeyStrokeKind::Space(i) if i != &0 => incorrect+=1.0,
            _ => ()
        }
    }
    correct/(correct+incorrect)
}

//kinda breaks when the duration is 0 but that rarely (never) happens so its ok :)
fn batch_key_strokes(
    key_strokes: &[(Duration, KeyStrokeKind)],
    time_step: f64,
) -> Vec<(f64, f64, f64)> {
    let mut results = Vec::new();
    for (key, group) in key_strokes
        .iter()
        .group_by(|(dur, _)| (dur.as_secs_f64() / time_step).ceil() as usize)
        .into_iter()
    {
        let (chars, errors) = group.fold((0.0, 0.0), |mut acc, (_, ks)| {
            match ks {
                KeyStrokeKind::Incorrect(_) => {
                    acc.1 += 1.0;
                    acc.0 += 1.0
                }
                _ => acc.0 += 1.0,
            }
            acc
        });
        results.push((key as f64 * time_step, chars, errors))
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn word_dif_extra() {
        use CharDiffKind::*;
        assert!(word_difference("aabbc", "ahhbcaa")
            .eq([Correct, Incorrect, Incorrect, Correct, Correct, Extra, Extra]))
    }
    #[test]
    fn word_dif_missed() {
        use CharDiffKind::*;
        assert!(word_difference("bbbdas", "bbb")
            .eq([Correct, Correct, Correct, Missed, Missed, Missed]))
    }
    #[test]
    fn batch_ks() {
        use KeyStrokeKind::*;
        let stats = [
            (0.1, Correct('a')),
            (0.5, Correct('a')),
            (0.8, Space(0)),
            (1.1, Incorrect('b')),
            (1.3, Correct('d')),
        ]
        .map(|(d, ks)| (Duration::from_secs_f64(d), ks));
        assert_eq!(
            batch_key_strokes(&stats, 1.0),
            vec![(1.0, 3.0, 0.0), (2.0, 2.0, 1.0)]
        )
    }
    #[test]
    fn batch_ks_time_step() {
        use KeyStrokeKind::*;
        let stats = [
            (0.1, Correct('a')),
            (0.5, Correct('a')),
            (0.8, Space(0)),
            (1.1, Incorrect('b')),
            (1.3, Correct('d')),
        ]
        .map(|(d, ks)| (Duration::from_secs_f64(d), ks));
        assert_eq!(
            batch_key_strokes(&stats, 0.4),
            vec![
                (0.4, 1.0, 0.0),
                (0.8, 2.0, 0.0),
                (1.2000000000000002, 1.0, 1.0),
                (1.6, 1.0, 0.0)
            ]
        )
    }
    #[test]
    fn final_stats_empty() {
        let stats = FinalStats::calculate(&[], &[], Duration::from_secs(60));
        assert_eq!(stats, FinalStats::default())
    }
    #[test]
    fn final_stats_all_correct() {
        let input = ["dac", "b"].map(String::from);
        let correct = ["dac", "bb"].map(String::from);
        let stats = FinalStats::calculate(
            &input,
            &correct,
            Duration::from_secs(
                12, /* 12 to make char amount match wpm due to how normalize_wpm() works : x/5 * (60/12) = x  */
            ),
        );
        assert_eq!(stats, FinalStats {
            wpm: 5.0, // 4 chars + 1 space
            raw_wpm: 5.0,
            correct: 4, 
            extra: 0,
            incorrect: 0,
            missed: 0,
        })
    }
    #[test]
    fn final_stats_errors() {
        let input = ["bbc", "bda", "cdq", "a"].map(String::from);
        let correct = ["dac", "bb", "cd", "aaa"].map(String::from);
        let stats = FinalStats::calculate(
            &input,
            &correct,
            Duration::from_secs(
                12, /* 12 to make char amount match wpm due to how normalize_wpm() works : x/5 * (60/12) = x  */
            ),
        );
        assert_eq!(stats, FinalStats {
            wpm: 1.0, 
            raw_wpm: 13.0,
            correct: 5, 
            extra: 2,
            incorrect:3,
            missed: 0,
        })
    }
    #[test]
    fn final_stats_missed() {
        let input = ["bb", "b", "ha", "b"].map(String::from);
        let correct = ["bbaa", "baaa", "haaa", "b"].map(String::from);
        let stats = FinalStats::calculate(
            &input,
            &correct,
            Duration::from_secs(
                12, 
            ),
        );
        assert_eq!(stats, FinalStats {
            wpm: 2.0, 
            raw_wpm: 10.0,
            correct: 6, 
            extra: 0,
            incorrect:0,
            missed: 7,
        })
    }
    #[test]
    fn final_stats_duration() {
        let input = ["aaaa", "aaaa", "aaaa", "aaaa"].map(String::from);
        let correct = ["aaaa", "aaaa", "aaaa", "aaaa"].map(String::from);
        let stats = FinalStats::calculate(
            &input,
            &correct,
            Duration::from_secs(
                60, 
            ),
        );
        assert_eq!(stats, FinalStats {
            wpm: 20.0 / 5.0, // 4 chars + 1 space
            raw_wpm: 20.0 / 5.0,
            correct: 16, 
            extra: 0,
            incorrect:0,
            missed: 0,
        })
    }
}
