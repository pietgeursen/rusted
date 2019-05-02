#[macro_use]
extern crate lazy_static;
extern crate ropey;

use regex::Regex;
use std::io::{ BufRead, Write, BufReader, BufWriter, stdin, stdout, Error};

use ropey::{Rope, RopeBuilder};

#[derive(Debug, PartialEq)]
enum Mode {
    Command(Rope),
    BadCommand(Rope),
    Input(Rope, Option<usize>),
    Exit,
    ConfirmExit(Rope, bool),
}

fn main() -> Result<(), Error> {
    let rd = stdin();
    let mut wr = stdout();
    let mut buf_rd = BufReader::new(rd);

    let mut mode = Mode::Command(Rope::new());
    loop {
        mode = read_lines(mode, &mut buf_rd, &mut wr)?;

        if mode == Mode::Exit {
            break;
        }
    }
    Ok(())
}

fn read_lines(mode: Mode, rd: &mut BufRead, wr: &mut Write) -> Result<Mode, Error> {
    lazy_static! {
        static ref PUNKT: Regex = Regex::new(r"^\.\n").unwrap();
        static ref QUIT: Regex = Regex::new(r"^q\n").unwrap();
        static ref APPEND: Regex = Regex::new(r"^(?P<line_num>\d+)|a\n").unwrap();
    }

    match mode {
        Mode::BadCommand(state) => {
            wr.write(b"?\n")?;
            return Ok(Mode::Command(state));
        }
        Mode::Command(state) => {
            let mut lines = String::new();
            rd.read_line(&mut lines)?;

            if APPEND.is_match(&lines) {
                let line_num = APPEND
                    .captures(&lines)
                    .map(|cap|{
                        cap.name("line_num")
                            .map(|cap|{
                                cap.as_str()
                            })
                            .map(|num_str|{
                                usize::from_str_radix(num_str, 10)
                                    .unwrap_or(0)
                            })
                    })
                    .unwrap_or(None);
                return Ok(Mode::Input(state, line_num));
            }

            if QUIT.is_match(&lines) {
                return Ok(Mode::Exit);
            }
            Ok(Mode::BadCommand(state))
        }
        Mode::Input(mut state, line_num) => {

            // If the line_num they're trying to append to is too large, immediately fail.
            if let Some(num) = line_num {
                if num >= state.len_lines(){
                    return Ok(Mode::BadCommand(state));
                }
            }

            let mut builder = RopeBuilder::new();
            loop{
                let mut line = String::new();
                rd.read_line(&mut line)?;
                if PUNKT.is_match(&line){
                    break;
                }

                builder.append(&line);
            }

            let lines_rope = builder.finish();

            match line_num {
                Some(line_num) =>{
                    let idx = state.line_to_char(line_num);
                    state.insert(idx, &lines_rope.to_string());
                },
                None => {
                    state.append(lines_rope);
                }
            }
            Ok(Mode::Command(state))
        }
        _ => Ok(Mode::Exit),
    }
}

#[cfg(test)]
mod test {
    use crate::*;
    use std::io::BufReader;
    use ropey::Rope;

    #[test]
    fn q_quits_simple() {
        let mode = Mode::Command(Rope::new());
        let chars = "q\n";
        let mut buf = BufReader::new(chars.as_bytes());
        let mut wr = BufWriter::new(Vec::new());
        let result = read_lines(mode, &mut buf, &mut wr).unwrap();
        assert_eq!(result, Mode::Exit);
    }
}
