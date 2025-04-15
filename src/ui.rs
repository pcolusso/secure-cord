use std::{borrow::Cow, future::Future, time::Duration};

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
                        Cell::from(Cow::Borrowed(s.1.name.as_str())),
                        Cell::from(Cow::Borrowed(s.1.identifier.as_str())),
                        Cell::from(Cow::Borrowed(s.1.env.as_str())),
                        Cell::from(if s.2 { "Running" } else { "Stopped" })
                    ])
                });

                let table = Table::new(rows, vec![30, 30, 20, 10])
                    .block(block)
                    .header(Row::new(vec![
                        Cell::from("Nickname"),
                        Cell::from("Identifier"),
                        Cell::from("Environment"),
                        Cell::from("Status")
                    ]).style(Style::new().bold().bg(Color::LightRed)))
                    .highlight_symbol(" 👉 ")
                    .row_highlight_style(Style::new().light_green());

                f.render_stateful_widget(table, cunks[0], &mut self.table_state);

                let help = Paragraph::new("up/down to move, e to edit, d to delete, s to save, space to start/stop").style(Style::new().bg(Color::Blue));
                f.render_widget(help, cunks[1]);
            }
            Mode::Edit { selected } => {
                let block = Block::default()
                    .title("Editing")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);
                f.render_widget(block, cunks[0]);

                let help = Paragraph::new("up/down to move, esc to cancel, return to save.").style(Style::new().bg(Color::Blue));
                f.render_widget(help, cunks[1]);
            }
        }
    }

    async fn poll_sessions(&mut self) {
        let futs: Vec<_> = self.server_list.iter().enumerate().map(|(i, (session, server, status))| {
            let f = session.healthy();
            async move { (i, f.await) }
        }).collect();
        let res: Vec<(usize, bool)> = futures::future::join_all(futs).await;
        for (i, status) in res {
            self.server_list[i].2 = status;
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
                self.poll_sessions().await;
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
                KeyCode::Char('e') => {
                    if let Some(sel) = self.table_state.selected() {
                        self.mode = Mode::Edit { selected: sel }
                    }
                },
                KeyCode::Char(' ') => {
                    if let Some(selected) = self.table_state.selected() {
                        let handle = self.server_list[selected].0.clone();
                        let running = self.server_list[selected].2;
                        tokio::spawn(async move {
                            if running {
                                handle.stop().await;
                            } else {
                                handle.start().await;
                            }
                        });
                    }
                }
                _ => {}
            },
            Mode::Edit { selected } => match key.code {
                KeyCode::Enter => {
                    self.mode = Mode::Main;
                },
                KeyCode::Esc  => self.mode = Mode::Main,
                _ => {}
            },
        }
    }
}
