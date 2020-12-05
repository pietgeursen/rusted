#[macro_use]
extern crate lazy_static;

use futures::channel::mpsc::unbounded;
use futures::io::BufReader;
use futures::lock::Mutex;
use futures::prelude::*;
use log::trace;
use std::io::stdout;
use std::sync::Arc;

use inu_rs::*;
use regex::Regex;
use smol::Unblock;
use std::io::{stdin, Error};

mod mode;
use mode::*;

fn main() -> Result<(), Error> {
    pretty_env_logger::init_timed();

    smol::block_on(async {
        // Set up our inu state manager.
        let (output_sender, output_receiver) = unbounded();
        let initial_state = Mode::Command(InternalState::new(Some(output_sender)));
        let mut inu = Inu::new(initial_state);

        // Set up a future that will resolve when the state is Mode::Exit so that we can exit the
        // program.
        let quitter = inu
            .subscribe()
            .await
            .filter(|state| {
                let is_exit = if let Mode::Exit = state { true } else { false };

                future::ready(is_exit)
            })
            .into_future();

        // Handle the stream of output and write it to std out.
        let wr = Unblock::new(stdout());
        let buf_wr = Arc::new(Mutex::new(wr));
        let output_writer = output_receiver
            .map(|lines| (lines, buf_wr.clone()))
            .for_each(|(lines, buf_wr)| async move {
                let mut buf_wr = buf_wr.lock().await;
                buf_wr.write_all(&lines).await.unwrap();
            });

        // Set up a stream that maps from lines of input on stdin into actions + effects.
        let handle = inu.get_handle();
        let rd = Unblock::new(stdin());
        let buf_rd = BufReader::new(rd);
        let input_dispatcher = buf_rd
            .lines()
            .map(|line| line.unwrap() + "\n") // Put the newlines back on.
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
            _ = output_writer.fuse() => (),
            _ = quitter.fuse() => ()
        };

        Ok(())
    })
}

async fn map_line_to_action(
    (line, handle): (String, Handle<Mode>),
) -> Option<(Option<Action>, Option<Effect>, Handle<Mode>)> {
    lazy_static! {
        static ref PUNKT: Regex = Regex::new(r"^\.\n$").unwrap();
        static ref QUIT: Regex = Regex::new(r"^q\n$").unwrap();
        static ref PRINT: Regex = Regex::new(r"^(?P<line_num>\d+)?p\n$").unwrap();
        static ref APPEND: Regex = Regex::new(r"^(?P<line_num>\d+)?a\n$").unwrap();
    }

    trace!("INPUT LINE: {:?}", line);
    let state = handle.get_state().await;

    match state {
        Mode::Command(state) => {
            if QUIT.is_match(&line) {
                Some((Some(Action::Quit), None, handle))
            } else if APPEND.is_match(&line) {
                Some((
                    Some(Action::StartAppendingInput(LineNumber(state.current_line))),
                    None,
                    handle,
                ))
            } else if PRINT.is_match(&line) {
                Some((None, Some(Effect::Print), handle))
            } else {
                None
            }
        }
        Mode::Input(_, _) => {
            if PUNKT.is_match(&line) {
                Some((Some(Action::ChangeToCommandMode), None, handle))
            } else {
                Some((Some(Action::AddInputLine(line)), None, handle))
            }
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
