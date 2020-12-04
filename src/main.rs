#[macro_use]
extern crate lazy_static;

use futures::io::BufReader;
use futures::prelude::*;
use log::debug;

//use futures_lite::io::{AsyncBufReadExt, BufReader};
//use futures_lite::stream::StreamExt;
use inu_rs::State as InuState;
use inu_rs::*;
use regex::Regex;
use smol::Unblock;
use std::io::{stdin, stdout, Error};
use std::pin::Pin;
use std::sync::Arc;

use ropey::Rope;

#[derive(Debug, Clone)]
struct State {
    rope: Arc<Rope>,
    current_line: usize,
}

impl State {
    pub fn new() -> State {
        State {
            rope: Arc::new(Rope::new()),
            current_line: 0,
        }
    }
}

#[derive(Debug, Clone)]
enum Mode {
    Command(State),
    //    Print(State),
    Input(State), // TODO this could also contain the lines to add, and then they can all be added at once for efficiency.
    Exit,
    ConfirmExit(State),
}

#[derive(Clone, Debug)]
struct LineNumber(usize);

#[derive(Clone, Debug)]
enum Action {
    AddInputLine(String),
    StartInsertingInput(LineNumber),
    StartAppendingInput(LineNumber),
    ChangeToCommandMode,
    Quit,
    SetLineNumber(usize),
}

#[derive(Clone, Debug)]
enum Effect {}

impl InuState for Mode {
    type Action = Action;
    type Effect = Effect;

    fn apply_action(&mut self, action: &Self::Action) {
        debug!("ACTION: {:?}, CURRENT STATE: {:?}", action, self);

        match self {
            Mode::Command(state) => match action {
                Action::StartAppendingInput(line_number) => {
                    state.current_line = line_number.0;
                    *self = Mode::Input(state.clone());
                }
                Action::Quit => *self = Mode::Exit,
                _ => (),
            },
            Mode::Input(state) => match action {
                Action::AddInputLine(line) => {
                    let idx = state.rope.line_to_char(state.current_line);
                    Arc::<Rope>::get_mut(&mut state.rope)
                        .unwrap()
                        .insert(idx, line);
                    state.current_line += 1;
                }
                Action::ChangeToCommandMode => *self = Mode::Command(state.clone()),
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
    fn apply_effect(&self, _effect: &Self::Effect) -> Pin<Box<dyn Stream<Item = Self::Action>>> {
        Box::pin(stream::empty())
    }
    //    fn apply(&mut self, action: &Self::Action) {
    //        *self = match self{
    //            Mode::Command(state) => {
    //                match action {
    //                    Action::StartAppendingInput(line_number) => {
    //                        Mode::Input(*state, line_number.0)
    //                    },
    //                    Action::Quit => {
    //                        Mode::Exit
    //                    },
    //                    _ => Mode::BadCommand(*state)
    //                }
    //            }
    //        }
    //    }
}

fn main() -> Result<(), Error> {
    pretty_env_logger::init_timed();

    smol::block_on(async {
        let initial_state = Mode::Command(State::new());
        let mut inu = Inu::new(initial_state);

        let handle = inu.get_handle();
        let quitter = inu
            .subscribe()
            .await
            .filter(|state| {
                let is_exit = if let Mode::Exit = state { true } else { false };

                future::ready(is_exit)
            })
            .into_future()
            .then(|_| handle.stop());

        let handle = inu.get_handle();

        let input_dispatcher = smol::spawn(async move {
            lazy_static! {
                static ref PUNKT: Regex = Regex::new(r"^\.\n$").unwrap();
                static ref QUIT: Regex = Regex::new(r"^q\n$").unwrap();
                static ref PRINT: Regex = Regex::new(r"^(?P<line_num>\d+)?p\n$").unwrap();
                static ref APPEND: Regex = Regex::new(r"^(?P<line_num>\d+)?a\n$").unwrap();
            }

            let rd = Unblock::new(stdin());
            let buf_rd = BufReader::new(rd);

            buf_rd
                .lines()
                .map(|line| line.unwrap() + "\n") // Put the newlines back on.
                .map(move |line| (line, handle.clone()))
                .filter_map(|(line, handle)| async move {
                    debug!("INPUT LINE: {:?}", line);
                    let state = handle.get_state().await.await.unwrap();

                    match state {
                        Mode::Command(state) => {
                            if QUIT.is_match(&line) {
                                Some((Action::Quit, handle))
                            } else if APPEND.is_match(&line) {
                                Some((
                                    Action::StartAppendingInput(LineNumber(state.current_line)),
                                    handle,
                                ))
                            } else {
                                None
                            }
                        }
                        Mode::Input(_) => {
                            if PUNKT.is_match(&line) {
                                Some((Action::ChangeToCommandMode, handle))
                            } else {
                                Some((Action::AddInputLine(line), handle))
                            }
                        }
                        _ => None,
                    }
                })
                .for_each(|(action, mut handle)| async move {
                    handle.dispatch(Some(action), None).await.unwrap();
                })
                .await
        });

        futures::select! {
            _ = inu.run().fuse() => (),
            _ = input_dispatcher.fuse() => (),
            _ = quitter.fuse() => ()

        };

        Ok(())
    })

    //    let rd = stdin();
    //    let wr = stdout();
    //    let mut buf_rd = BufReader::new(rd);
    //
    //
    //    let mut inu = Inu::new(mode);
    //    let inu_handle = inu.get_handle();
    //
    //    loop {
    //        let mut command = String::new();
    //        buf_rd.read_line(&mut command)?;
    //
    //        let action = create_action(&bundle.state, &command);
    //
    //        match action {
    //            Some(action) => {
    //                bundle.dispatch(&action)
    //            },
    //            None => {
    //                //wr.write(b"?\n")?;
    //            }
    //        }
    //        //mode = read_lines(mode, &mut buf_rd, &mut wr)?;
    //
    //        if *bundle.state == Mode::Exit {
    //            break;
    //        }
    //    }
    //    Ok(())
}

//fn create_action(mode: &Mode, command_string: &str) -> Option<Action> {
//    lazy_static! {
//        static ref PUNKT: Regex = Regex::new(r"^\.\n").unwrap();
//        static ref QUIT: Regex = Regex::new(r"^q\n").unwrap();
//        static ref PRINT: Regex = Regex::new(r"^(?P<line_num>\d+)?p\n").unwrap();
//        static ref APPEND: Regex = Regex::new(r"^(?P<line_num>\d+)?a\n").unwrap();
//    }
//
//    match mode {
//        Mode::Command(state) => {
//            if QUIT.is_match(command_string) {
//                Some(Action::Quit)
//            } else if PRINT.is_match(command_string) {
//                None //Some(Action::Print)
//            } else if APPEND.is_match(command_string) {
//                Some(Action::StartAppendingInput(LineNumber(0)))
//            } else {
//                None
//            }
//        }
//        _ => None,
//    }
//}

//fn read_lines(mode: Mode, rd: &mut dyn BufRead, wr: &mut dyn Write) -> Result<Mode, Error> {
//    lazy_static! {
//        static ref PUNKT: Regex = Regex::new(r"^\.\n").unwrap();
//        static ref QUIT: Regex = Regex::new(r"^q\n").unwrap();
//    }
//
//    match mode {
//        Mode::BadCommand(state) => {
//            wr.write(b"?\n")?;
//            return Ok(Mode::Command(state));
//        }
//        Mode::Command(state) => {
//            let mut command = String::new();
//            rd.read_line(&mut command)?;
//
//            read_and_handle_commands(state, command, wr)
//        }
//        Mode::Input(mut state) => {
//            let mut builder = RopeBuilder::new();
//            loop {
//                let mut line = String::new();
//                rd.read_line(&mut line)?;
//                if PUNKT.is_match(&line) {
//                    break;
//                }
//
//                builder.append(&line);
//            }
//
//            let lines_rope = builder.finish();
//
//            let idx = state.rope.line_to_char(line_num);
//            //state.rope.insert(idx, &lines_rope.to_string());
//
//            state.current_line = line_num + lines_rope.len_lines() - 1;
//            Ok(Mode::Command(state))
//        }
//        Mode::ConfirmExit(state) => {
//            wr.write(b"?\n")?;
//            let mut command = String::new();
//            rd.read_line(&mut command)?;
//
//            if QUIT.is_match(&command) {
//                return Ok(Mode::Exit);
//            }
//            read_and_handle_commands(state, command, wr)
//        }
//        _ => Ok(Mode::Exit),
//    }
//}

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
//            return Ok(Mode::BadCommand(state));
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
//            return Ok(Mode::BadCommand(state));
//        }
//
//        let chunks = state.rope.line(line_num - 1).chunks();
//        for chunk in chunks {
//            wr.write_all(chunk.as_bytes())?;
//        }
//
//        return Ok(Mode::Command(state));
//    }
//
//    if QUIT.is_match(&command) {
//        //return Ok(Mode::ConfirmExit(state));
//        return Ok(Mode::Exit);
//    }
//    Ok(Mode::BadCommand(state))
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
//                Some(Action::ChangeToCommandMode)
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
        let mode = Mode::Command(Rope::new());
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
