use futures::prelude::*;
use inu_rs::State as InuState;
use log::debug;
use ropey::Rope;
use std::pin::Pin;
use std::sync::Arc;

mod action;
mod effect;
mod internal_state;

pub use action::{Action, LineNumber};
pub use effect::Effect;
pub use internal_state::{InputLinesToAdd, InternalState};

#[derive(Debug, Clone)]
pub enum Mode {
    Command(InternalState),
    //    Print(State),
    Input(InternalState, InputLinesToAdd), // TODO this could also contain the lines to add, and then they can all be added at once for efficiency.
    Exit,
    ConfirmExit(InternalState),
}

impl InuState for Mode {
    type Action = Action;
    type Effect = Effect;

    fn apply_action(&mut self, action: &Self::Action) {
        debug!("ACTION: {:?}, CURRENT STATE: {:?}", action, self);

        match self {
            Mode::Command(state) => match action {
                Action::StartAppendingInput(line_number) => {
                    state.current_line = line_number.0;
                    *self = Mode::Input(state.clone(), InputLinesToAdd::default());
                }
                Action::Quit => *self = Mode::Exit,
                _ => (),
            },
            Mode::Input(state, lines_to_add) => match action {
                Action::AddInputLine(line) => {
                    lines_to_add.lines.push_str(&line);
                    lines_to_add.num_lines += 1;
                }
                Action::ChangeToCommandMode => {
                    let idx = state.rope.line_to_char(state.current_line);
                    Arc::<Rope>::get_mut(&mut state.rope)
                        .unwrap()
                        .insert(idx, &lines_to_add.lines);
                    state.current_line += lines_to_add.num_lines;
                    *self = Mode::Command(state.clone())
                }
                _ => (),
            },
            Mode::ConfirmExit(_state) => match action {
                Action::Quit => *self = Mode::Exit,
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
        match self {
            Mode::Command(state) => match effect {
                Effect::Print => {
                    let state = state.clone();
                    let strm = async {
                        if let Some(mut writer) = state.output_writer{
                            let mut buff = vec![];
                            state.rope.write_to(&mut buff).unwrap();
                            writer.send(buff).await.unwrap();
                        }
                        None
                    }.into_stream();

                    Box::pin(strm)
                }
            },
            _ => Box::pin(stream::empty()),
        }
    }
}
