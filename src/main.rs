#[macro_use]
extern crate lazy_static;

use regex::Regex;
use std::io::{stdin, stdout, BufRead, BufReader, Error, Write};
use redux_bundler_rs::{State as ReduxState, Reactor, Bundle, Redux};
use std::sync::Arc;

use ropey::{Rope, RopeBuilder};

#[derive(Debug, PartialEq)]
struct State {
    rope: Rope,
    current_line: usize,
}

impl State {
    pub fn new() -> State {
        State {
            rope: Rope::new(),
            current_line: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Mode {
    Command(State),
    BadCommand(State),
    Print(State),
    Input(State, usize),
    Exit,
    ConfirmExit(State),
}

struct LineNumber(usize);

enum Action{
    AddInputLine(String),
    StartInsertingInput(LineNumber),
    StartAppendingInput(LineNumber),
    Print,
    ChangeToCommandMode,
    Quit,
    SetLineNumber(usize),
}

impl ReduxState for Mode{
    type Action = Action;

    fn apply(&mut self, action: &Self::Action) {
        match action {
            Action::StartAppending
            Action::Quit => {
                *self = Mode::Exit;
            },
            _ => ()
        }
    }
}

fn main() -> Result<(), Error> {
    let rd = stdin();
    let wr = stdout();
    let mut buf_rd = BufReader::new(rd);

    let mode = Mode::Command(State::new());
    let print_reactor = Box::new(PrintReactor{writer: wr});

    let mut bundle = Bundle{
        state: Arc::new(mode),
        reactors: vec![print_reactor],
        subscribers: vec![],
    };

    loop {
        let mut command = String::new();
        buf_rd.read_line(&mut command)?;

        let action = create_action(&bundle.state, &command);

        match action {
            Some(action) => {
                bundle.dispatch(&action)
            },
            None => {
                //wr.write(b"?\n")?;
            }
        }
        //mode = read_lines(mode, &mut buf_rd, &mut wr)?;

        if *bundle.state == Mode::Exit {
            break;
        }
    }
    Ok(())
}

fn create_action(mode: &Mode, command_string: &str) -> Option<Action>{

    lazy_static! {
        static ref PUNKT: Regex = Regex::new(r"^\.\n").unwrap();
        static ref QUIT: Regex = Regex::new(r"^q\n").unwrap();
        static ref PRINT: Regex = Regex::new(r"^(?P<line_num>\d+)?p\n").unwrap();
        static ref APPEND: Regex = Regex::new(r"^(?P<line_num>\d+)?a\n").unwrap();
    }

    match mode{
        Mode::Command(state) => {
            if QUIT.is_match(command_string) {
                Some(Action::Quit)
            }else if PRINT.is_match(command_string){
                Some(Action::Print)
            }else if APPEND.is_match(command_string){
                Some(Action::StartAppendingInput(LineNumber(0)))
            }else{
                None
            }
        },
        _ => None 
    }
} 

fn read_lines(mode: Mode, rd: &mut dyn BufRead, wr: &mut dyn Write) -> Result<Mode, Error> {
    lazy_static! {
        static ref PUNKT: Regex = Regex::new(r"^\.\n").unwrap();
        static ref QUIT: Regex = Regex::new(r"^q\n").unwrap();
    }

    match mode {
        Mode::BadCommand(state) => {
            wr.write(b"?\n")?;
            return Ok(Mode::Command(state));
        }
        Mode::Command(state) => {
            let mut command = String::new();
            rd.read_line(&mut command)?;

            read_and_handle_commands(state, command, wr)
        },
        Mode::Input(mut state, line_num) => {
            let mut builder = RopeBuilder::new();
            loop {
                let mut line = String::new();
                rd.read_line(&mut line)?;
                if PUNKT.is_match(&line) {
                    break;
                }

                builder.append(&line);
            }

            let lines_rope = builder.finish();

            let idx = state.rope.line_to_char(line_num);
            state.rope.insert(idx, &lines_rope.to_string());

            state.current_line = line_num + lines_rope.len_lines() - 1;
            Ok(Mode::Command(state))
        }
        Mode::ConfirmExit(state) => {
            wr.write(b"?\n")?;
            let mut command = String::new();
            rd.read_line(&mut command)?;

            if QUIT.is_match(&command) {
                return Ok(Mode::Exit);
            }
            read_and_handle_commands(state, command, wr)
        }
        _ => Ok(Mode::Exit),
    }
}

fn read_and_handle_commands(
    state: State,
    command: String,
    wr: &mut dyn Write,
) -> Result<Mode, Error> {

    lazy_static! {
        static ref QUIT: Regex = Regex::new(r"^q\n").unwrap();
        static ref APPEND: Regex = Regex::new(r"^(?P<line_num>\d+)?a\n").unwrap();
        static ref PRINT: Regex = Regex::new(r"^(?P<line_num>\d+)?p\n").unwrap();
    }

    if APPEND.is_match(&command) {
        let line_num = APPEND
            .captures(&command)
            .and_then(|cap| {
                cap.name("line_num")
                    .map(|cap| cap.as_str())
                    .map(|num_str| usize::from_str_radix(num_str, 10).unwrap_or(0))
            })
            .unwrap_or(state.current_line);

        // If the line_num they're trying to append to is too large, immediately fail.
        if line_num >= state.rope.len_lines() {
            return Ok(Mode::BadCommand(state));
        }

        return Ok(Mode::Input(state, line_num));
    }

    if PRINT.is_match(&command) {
        let line_num = PRINT
            .captures(&command)
            .and_then(|cap| {
                cap.name("line_num")
                    .map(|cap| cap.as_str())
                    .map(|num_str| usize::from_str_radix(num_str, 10).unwrap_or(0))
            })
            .unwrap_or(state.current_line);

        // If the line_num they're trying to print to is too large, immediately fail.
        if line_num >= state.rope.len_lines() {
            return Ok(Mode::BadCommand(state));
        }

        let chunks = state.rope.line(line_num - 1).chunks();
        for chunk in chunks {
            wr.write_all(chunk.as_bytes())?;
        }

        return Ok(Mode::Command(state));
    }

    if QUIT.is_match(&command) {
        return Ok(Mode::ConfirmExit(state));
    }
    Ok(Mode::BadCommand(state))
}

struct PrintReactor<W: Write>{
    writer: W
}

impl<W: Write> Reactor<Mode> for PrintReactor<W>{
    fn apply(&mut self, state: &Mode) -> Option<Action>{
        match state {
            Mode::Print(state) => {
                state.rope.write_to(&mut self.writer).expect("unable to write to writer");
                Some(Action::ChangeToCommandMode)
            }
            _ => None
        }
    }
}


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
