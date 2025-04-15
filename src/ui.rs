use std::{borrow::Cow, time::Duration};

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind, KeyCode};
use futures::{FutureExt, StreamExt};
use ratatui::{
    buffer::Buffer, layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style, Stylize}, widgets::{Block, BorderType, Borders, Cell, ListState, Paragraph, Row, StatefulWidget, Table, TableState, Widget}, DefaultTerminal, Frame
};

use crate::{servers::Server, ssm::Session, Uhh};

pub async fn run(server_list: Vec<Uhh>) -> Result<()> {
    let terminal = ratatui::init();
    App::new(server_list).run(terminal).await?;
    ratatui::restore();

    Ok(())
}

#[derive(Debug)]
enum Mode {
    Main,
    Edit { selected: usize },
}

#[derive(Debug)]
struct MainTable<'a> {
    servers: &'a Vec<Server>,
}


pub struct App {
    mode: Mode,
    server_list: Vec<Uhh>,
    table_state: TableState,
    running: bool,
    editing: Option<Server>,
    event_stream: EventStream
}

impl App {
    fn new(server_list: Vec<Uhh>) -> Self {
        let mut res = App {
            mode: Mode::Main,
            server_list,
            table_state: TableState::default(),
            event_stream: EventStream::default(),
            running: false,
            editing: None
        };
        res.table_state.select_first();

        res
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|f| self.draw(f))?;
            self.handle_crossterm_events().await?;
        }
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame) {
        // TODO: Make a funnk Cunk pun for this typo.
        // I'm limited by the creativity of my time.
        let cunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)].as_ref())
            .split(f.area());

        match self.mode {
            Mode::Main => {
                let block = Block::default()
                    .title("Servers")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);

                let rows = self.server_list.iter().map( |s| {
                    Row::new(vec![
                        Cell::from(Cow::Borrowed(s.1.nickname.as_str())),
                        Cell::from(Cow::Borrowed(s.1.identifier.as_str())),
                        Cell::from(Cow::Borrowed(s.1.env.as_str())),
                        Cell::from(if s.2 { "Running" } else { "Stopped" })
                    ])
                });

                let table = Table::new(rows, vec![30, 30, 20])
                    .block(block)
                    .header(Row::new(vec![
                        Cell::from("Nickname"),
                        Cell::from("Identifier"),
                        Cell::from("Environment"),
                    ]))
                    .highlight_symbol(" 👉 ")
                    .row_highlight_style(Style::new().cyan());

                f.render_stateful_widget(table, cunks[0], &mut self.table_state);

                let help = Paragraph::new("up/down to move, e to edit, d to delete, s to save, space to start/stop");
                f.render_widget(help, cunks[1]);
            }
            Mode::Edit { selected } => {
                let block = Block::default()
                    .title("Editing")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);
                f.render_widget(block, cunks[0]);
            }
        }
    }

    async fn handle_crossterm_events(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        tokio::select! {
            event = self.event_stream.next().fuse() => {
                match event {
                    Some(Ok(evt)) => {
                        match evt {
                            Event::Key(key)
                                if key.kind == KeyEventKind::Press
                                    => self.on_key_event(key).await,
                            Event::Mouse(_) => {}
                            Event::Resize(_, _) => {}
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            _ = interval.tick() => {

            }
        }
        Ok(())
    }

    async fn on_key_event(&mut self, key: KeyEvent) {
        match &mut self.mode {
            Mode::Main => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.running = false,
                KeyCode::Up | KeyCode::Char('k') => self.table_state.select_previous(),
                KeyCode::Down | KeyCode::Char('j') => self.table_state.select_next(),
                KeyCode::Char(' ') => {
                    if let Some(selected) = self.table_state.selected() {
                        self.server_list[selected].0.start().await;
                    }
                }
                _ => {}
            },
            Mode::Edit { selected } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Main,
                _ => {}
            },
        }
    }
}
