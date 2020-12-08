use futures::prelude::*;
use inu_rs::State as InuState;
use log::debug;
use ropey::Rope;
use std::pin::Pin;

mod action;
mod effect;

pub use action::Action;
pub use effect::Effect;

#[derive(Debug, Clone)]
pub enum Mode {
    Normal,
    Command(String),
    Input,
    Exit,
    ConfirmExit,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CursorPostion {
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub mode: Mode,
    pub cursor_position: CursorPostion,
    pub rope: Rope,
}

impl State {
    pub fn get_entire_rope(&self) -> String {
        self.rope.chunks().collect()
    }
}

impl InuState for State {
    type Action = Action;
    type Effect = Effect;

    fn apply_action(&mut self, action: &Self::Action) {
        debug!("ACTION: {:?}, CURRENT STATE: {:?}", action, self);

        match &self.mode {
            Mode::Normal => match action {
                Action::StartAppendingInput => {
                    self.mode = Mode::Input;
                }
                Action::ChangeToCommandMode => {
                    self.mode = Mode::Command(String::new());
                }
                _ => (),
            },
            Mode::Input => match action {
                Action::AddChar(chr) => {
                    let idx = self.rope.line_to_char(self.cursor_position.row)
                        + self.cursor_position.column;
                    self.rope.insert(idx, &chr);
                    self.cursor_position.column += 1;
                }
                Action::Enter => {
                    let idx = self.rope.line_to_char(self.cursor_position.row)
                        + self.cursor_position.column;
                    self.rope.insert(idx, "\n");
                    self.cursor_position.column = 0;
                    self.cursor_position.row += 1;
                }
                Action::ChangeToNormalMode => {
                    //state.current_line += lines_to_add.num_lines;
                    self.mode = Mode::Normal
                }
                _ => (),
            },
            Mode::Command(chars) => match action {
                Action::Quit => self.mode = Mode::Exit,
                Action::AddChar(chr) => {
                    let mut new_chars = chars.to_string();
                    new_chars.push_str(&chr);
                    self.mode = Mode::Command(new_chars)
                }
                Action::ChangeToNormalMode => self.mode = Mode::Normal,
                _ => (),
            },
            Mode::ConfirmExit => match action {
                Action::Quit => self.mode = Mode::Exit,
                _ => (),
            },
            _ => (),
        };

        debug!("NEW STATE: {:?}", self);
    }

    fn apply_effect(
        &self,
        effect: &Self::Effect,
    ) -> Pin<Box<dyn Stream<Item = Option<Self::Action>>>> {
        debug!("EFFECT: {:?}, CURRENT STATE: {:?}", effect, self);
        Box::pin(stream::empty())
    }
}
