#[macro_use]
extern crate lazy_static;

use crosstermion::input::Key;
use futures::prelude::*;
use inu_rs::Handle;
use inu_rs::Inu;
use log::trace;
use regex::Regex;
use smol::spawn;
use std::io::Error;

mod mode;
mod ui;
use mode::*;
use ui::create_ui;

fn main() -> Result<(), Error> {
    pretty_env_logger::init_timed();

    smol::block_on(async {
        // Set up our inu state manager.
        let initial_state = State::default();
        let mut inu = Inu::new(initial_state);

        // Set up a future that will resolve when the state is Mode::Exit so that we can exit the
        // program.
        let quitter = inu
            .subscribe()
            .await
            .filter(|state| {
                let is_exit = if let Mode::Exit = state.mode {
                    true
                } else {
                    false
                };

                future::ready(is_exit)
            })
            .into_future();

        // Set up the ui. Runs in another thread.
        let (ui_sender, ui_input_receiver) = create_ui().unwrap();
        let handle = inu.get_handle();
        let sender = ui_sender.clone();

        // Render the initial state.
        // We need to detach here. The handle can't get the state until inu is running.
        spawn(async move {
            let mode = handle.get_state().await;
            sender.send(mode).unwrap();
        })
        .detach();

        // Subscribe to changes in state and send them to the ui for rendering
        let ui = inu.subscribe().await.for_each(|mode| async {
            ui_sender.send(mode).unwrap();
        });

        // Set up a stream that maps from key events to actions
        let handle = inu.get_handle();
        let input_dispatcher = ui_input_receiver
            .map(|line| (line, handle.clone()))
            .filter_map(map_line_to_action)
            .for_each(|(action, effect, mut handle)| async move {
                handle.dispatch(action, effect).await.unwrap();
            });

        // select! will resolve when one of these futures resolves, which will only ever be the
        // quitter, the others never end but we still need to include them in the select so that
        // the futures do get polled and driven by the event loop.
        futures::select! {
            _ = inu.run().fuse() => (),
            _ = input_dispatcher.fuse() => (),
            _ = ui.fuse() => (),
            _ = quitter.fuse() => ()
        };

        Ok(())
    })
}

async fn map_line_to_action(
    (key, handle): (Key, Handle<State>),
) -> Option<(Option<Action>, Option<Effect>, Handle<State>)> {
    lazy_static! {
        static ref PUNKT: Regex = Regex::new(r"^\.\n$").unwrap();
        static ref QUIT: Regex = Regex::new(r"^q!$").unwrap();
        static ref PRINT: Regex = Regex::new(r"^(?P<line_num>\d+)?p\n$").unwrap();
        static ref APPEND: Regex = Regex::new(r"^(?P<line_num>\d+)?a\n$").unwrap();
    }

    trace!("INPUT LINE: {:?}", key);
    let state = handle.get_state().await;

    let dispatch_action = |action: Action| -> Option<(Option<Action>, Option<Effect>, Handle<State>)> {
        Some((Some(action), None, handle.clone()))
    };

    match state.mode {
        Mode::Normal => {
            match key {
                Key::Char('a') => Some((
                    Some(Action::StartAppendingInput),
                    None,
                    handle,
                )),
                Key::Char('i') => Some((
                    Some(Action::StartInsertingInput),
                    None,
                    handle,
                )),
                Key::Char(':') => Some((Some(Action::ChangeToCommandMode), None, handle)),
                _ => None,
            }
            //            if QUIT.is_match(&line) {
            //                Some((Some(Action::Quit), None, handle))
            //            } else if APPEND.is_match(&line) {
            //                Some((
            //                    Some(Action::StartAppendingInput(LineNumber(state.current_line))),
            //                    None,
            //                    handle,
            //                ))
            //            } else if PRINT.is_match(&line) {
            //                Some((None, Some(Effect::Print), handle))
            //            } else {
            //                None
            //            }
        }
        Mode::Command(sofar) => {
            match key {
                Key::Esc => Some((Some(Action::ChangeToNormalMode), None, handle)),
                Key::Char('\n') => {
                    //TODO
                    if QUIT.is_match(&sofar) {
                        return dispatch_action(Action::Quit);
                    }
                    dispatch_action(Action::ChangeToNormalMode)
                    //Some((Some(Action::ChangeToNormalMode), None, handle))
                }
                Key::Char(c) => dispatch_action(Action::AddChar(c.into())),
                _ => None,
            }
        }
        Mode::Input => {
            match key {
                Key::Char('\n') => dispatch_action(Action::Enter),
                Key::Char(c) => dispatch_action(Action::AddChar(c.into())),
                Key::Esc => dispatch_action(Action::ChangeToNormalMode),
                _ => None,
            }
            //            if PUNKT.is_match(&line) {
            //                Some((Some(Action::ChangeToNormalMode), None, handle))
            //            } else {
            //                Some((Some(Action::AddInputLine(line)), None, handle))
            //            }
        }
        _ => None,
    }
}

//fn read_and_handle_commands(
//    state: State,
//    command: String,
//    wr: &mut dyn Write,
//) -> Result<Mode, Error> {
//    lazy_static! {
//        static ref QUIT: Regex = Regex::new(r"^q\n").unwrap();
//        static ref APPEND: Regex = Regex::new(r"^(?P<line_num>\d+)?a\n").unwrap();
//        static ref PRINT: Regex = Regex::new(r"^(?P<line_num>\d+)?p\n").unwrap();
//    }
//
//    if APPEND.is_match(&command) {
//        let line_num = APPEND
//            .captures(&command)
//            .and_then(|cap| {
//                cap.name("line_num")
//                    .map(|cap| cap.as_str())
//                    .map(|num_str| usize::from_str_radix(num_str, 10).unwrap_or(0))
//            })
//            .unwrap_or(state.current_line);
//
//        // If the line_num they're trying to append to is too large, immediately fail.
//        if line_num >= state.rope.len_lines() {
//            return Ok(Mode::BadNormal(state));
//        }
//
//        return Ok(Mode::Input(state));
//    }
//
//    if PRINT.is_match(&command) {
//        let line_num = PRINT
//            .captures(&command)
//            .and_then(|cap| {
//                cap.name("line_num")
//                    .map(|cap| cap.as_str())
//                    .map(|num_str| usize::from_str_radix(num_str, 10).unwrap_or(0))
//            })
//            .unwrap_or(state.current_line);
//
//        // If the line_num they're trying to print to is too large, immediately fail.
//        if line_num >= state.rope.len_lines() {
//            return Ok(Mode::BadNormal(state));
//        }
//
//        let chunks = state.rope.line(line_num - 1).chunks();
//        for chunk in chunks {
//            wr.write_all(chunk.as_bytes())?;
//        }
//
//        return Ok(Mode::Normal(state));
//    }
//
//    if QUIT.is_match(&command) {
//        //return Ok(Mode::ConfirmExit(state));
//        return Ok(Mode::Exit);
//    }
//    Ok(Mode::BadNormal(state))
//}

//struct PrintReactor<W: Write>{
//    writer: W
//}
//
//impl<W: Write> Reactor<Mode> for PrintReactor<W>{
//    fn apply(&mut self, state: &Mode) -> Option<Action>{
//        match state {
//            Mode::Print(state) => {
//                state.rope.write_to(&mut self.writer).expect("unable to write to writer");
//                Some(Action::ChangeToNormalMode)
//            }
//            _ => None
//        }
//    }
//}

#[cfg(test)]
mod test {
    use crate::*;
    use ropey::Rope;
    use std::io::BufReader;

    #[test]
    fn q_quits_simple() {
        let mode = Mode::Normal(Rope::new());
        let chars = "q\n";
        let mut buf = BufReader::new(chars.as_bytes());
        let mut wr = BufWriter::new(Vec::new());
        let result = read_lines(mode, &mut buf, &mut wr).unwrap();
        assert_eq!(result, Mode::Exit);
    }

    fn print_at_current_cursor() {}

    fn print_with_line_num_args() {}

    fn unknown_command_writes_question_mark() {}

    fn appends_to_end_with_no_line_num_args() {}

    fn appends_to_correct_line_with_line_num_args() {}

    fn does_not_append_when_only_line_args_no_a() {}
}
