use std::{iter, borrow::Cow};

use ratatui::{prelude::*, widgets::StatefulWidget};

use crate::states;

pub struct TypingWidget {
    style_correct: Style,
    style_error: Style,
    style_untyped: Style,
    style_cursor: Style,
}
impl TypingWidget {
    pub fn new() -> Self {
        Self {
            style_correct: Style::default().green(),
            style_error: Style::default().red(),
            style_untyped: Style::default().dark_gray(),
            style_cursor: Style::default().on_white(),
        }
    }
    fn render_input_dif(
        &self,
        input: &String,
        word: &String,
        buf: &mut Buffer,
        area: Rect,
        x: u16,
        y: u16,
    ) {
        if input == word {
            buf.set_style(
                Rect {
                    x: area.x + x,
                    y: area.y + y,
                    width: word.len() as u16,
                    height: 1,
                },
                self.style_correct,
            )
        } else {
            for ((i, input_char), correct_char) in input
                .char_indices()
                .zip(word.chars().map(Some).chain(iter::repeat(None)))
            {
                buf.set_style(
                    Rect {
                        x: area.x + x + i as u16,
                        y: area.y + y,
                        width: 1,
                        height: 1,
                    },
                    match correct_char.is_some_and(|char| char == input_char) {
                        false => self.style_error,
                        true => self.style_correct,
                    },
                )
            }
        }
    }
    fn combine_input<'a>(input: Option<&'a String>, word: &'a String) -> Cow<'a, str> {
        match input {
            None => word.into(),
            Some(s) => {
                if word.len() > s.len() {
                    (s.clone() + &word[s.len()..]).into()
                } else {
                    s.into()
                }
            }
        }
    }
}
impl StatefulWidget for TypingWidget {
    type State = states::TypingState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (mut x, mut y) = (0, 0);

        let mut new_rows = vec![state.rows[0]];

        for (word, (input_index, input)) in state
            .word_list
            .iter()
            .zip(
                state
                    .written_words
                    .iter()
                    .map(Some)
                    .chain(iter::repeat(None))
                    .enumerate(),
            )
            .skip(state.rows[0])
        {
            let word_to_display = Self::combine_input(input, word);
            if x + word_to_display.len() as u16 > area.width {
                y += 1;
                x = 0;
                new_rows.push(input_index);
            }
            if y >= area.height {
                break;
            }
            if input_index == state.written_words.len() - 1 {
                if y >= 2 {
                    new_rows.remove(0);
                }
                let mut cursor_x = x + input.unwrap().len() as u16;
                let mut cursor_y = y;
                if cursor_x >= area.width {
                    cursor_x = 0;
                    cursor_y += 1;
                }
                buf.set_style(
                    Rect {
                        x: area.x + cursor_x,
                        y: area.y + cursor_y,
                        width: 1,
                        height: 1,
                    },
                    self.style_cursor,
                )
            }
            buf.set_string(x + area.x, y + area.y, &word_to_display, self.style_untyped);
            if let Some(input) = input {
                self.render_input_dif(input, word, buf, area, x, y);
            }
            x += word_to_display.len() as u16 + 1;
        }
        state.rows = new_rows;
    }
}