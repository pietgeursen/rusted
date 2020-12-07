use crate::Mode;
use crate::State;
use crosstermion::input::{key_input_stream, Key};
use crosstermion::termion::raw::IntoRawMode;
use crosstermion::tui::backend::Backend;
use crosstermion::tui::backend::TermionBackend;
use crosstermion::tui::layout::Rect;
use crosstermion::tui::layout::{Constraint, Direction, Layout};
use crosstermion::tui::style::{Color, Modifier, Style};
use crosstermion::tui::text::{Span, Spans};
use crosstermion::tui::widgets::Paragraph;
use crosstermion::tui::widgets::{Block, Borders, Wrap};
use crosstermion::tui::Frame;
use crosstermion::tui::Terminal;
use futures::Stream;
use std::io;
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;

pub fn create_ui() -> Result<(Sender<State>, impl Stream<Item = Key>), io::Error> {
    let (sender, receiver) = channel();

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let reader_stream = key_input_stream();

    spawn(move || -> Result<(), io::Error> {
        receiver.iter().for_each(|mode| {
            terminal
                .draw(|mut f| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(0)
                        .constraints([Constraint::Min(30), Constraint::Length(2)].as_ref())
                        .split(f.size());

                    draw_text_input(&mut f, chunks[0], &mode);
                    draw_bottom_bar(&mut f, chunks[1], &mode);
                })
                .unwrap();
        });
        Ok(())
    });

    Ok((sender, reader_stream))
}

fn draw_text_input<B>(f: &mut Frame<B>, area: Rect, state: &State)
where
    B: Backend,
{
    let text: String = state.get_entire_rope();
    let text_spans = vec![Spans::from(text)];

    let block = Block::default().borders(Borders::ALL);

    let paragraph = Paragraph::new(text_spans)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area)
}

fn draw_bottom_bar<B>(f: &mut Frame<B>, area: Rect, state: &State)
where
    B: Backend,
{
    let status = match state.mode {
        Mode::Command(_) => "COMMAND",
        Mode::Input => "INSERT",
        Mode::Normal => "NORMAL",
        _ => "",
    };

    let cmd = match &state.mode {
        Mode::Command(sofar) => format!(":{}", sofar),
        _ => "".into(),
    };

    let status_text = vec![Spans::from(status), Spans::from(cmd)];

    let block = Block::default().borders(Borders::NONE);

    let paragraph = Paragraph::new(status_text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area)
}
