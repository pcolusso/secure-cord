use std::{borrow::Cow, future::Future, time::Duration};

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind, KeyCode};
use futures::{FutureExt, StreamExt};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Borders, Cell, ListState, Paragraph, Row, StatefulWidget, Table, TableState, Widget},
    DefaultTerminal, Frame
};

use crate::{servers::Server, ssm::Session, Uhh};

pub async fn run(server_list: Vec<Uhh>) -> Result<()> {
    let terminal = ratatui::init();
    App::new(server_list).run(terminal).await?;
    ratatui::restore();

    Ok(())
}

enum Mode {
    Main,
    Edit(EditView),
}

struct EditView {
    selected: usize,
    stdout: Vec<String>,
    scroll: usize,
    last_update: std::time::Instant,
    session: Session,
}

impl EditView {
    fn new(selected: usize, session: Session) -> Self {
        Self {
            selected,
            stdout: Vec::new(),
            scroll: 0,
            last_update: std::time::Instant::now(),
            session,
        }
    }

    async fn update_stdout(&mut self) {
        if self.last_update.elapsed() > std::time::Duration::from_secs(1) {
            self.stdout = self.session.stdout().await;
            self.last_update = std::time::Instant::now();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            _ => false,
        }
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Editing")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        f.render_widget(block, area);

        // Limit scroll to reasonable bounds
        self.scroll = self.scroll.min(self.stdout.len().saturating_sub(1));

        let visible_lines = area.height.saturating_sub(2) as usize;
        let start_idx = self.stdout.len().saturating_sub(self.scroll + visible_lines);
        let end_idx = start_idx + visible_lines;
        let visible_text = self.stdout[start_idx..end_idx.min(self.stdout.len())].join("\n");

        let stdout_para = Paragraph::new(visible_text)
            .block(Block::default().title(format!(
                "SSM Output ({} lines)",
                self.stdout.len()
            )).borders(Borders::ALL)
            .border_type(BorderType::Rounded));
        f.render_widget(stdout_para, area.inner(Margin::new(1, 1)));
    }
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
            self.handle_events().await?;
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

        match &mut self.mode {
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
            Mode::Edit(edit_view) => {
                edit_view.draw(f, cunks[0]);
                let help = Paragraph::new("esc to cancel, return to save.").style(Style::new().bg(Color::Blue));
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

    async fn handle_events(&mut self) -> Result<()> {
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
                if let Mode::Edit(edit_view) = &mut self.mode {
                    edit_view.update_stdout().await;
                }
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
                        let (session, _, _) = &self.server_list[sel];
                        let mut edit_view = EditView::new(sel, session.clone());
                        edit_view.update_stdout().await;
                        self.mode = Mode::Edit(edit_view);
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
            Mode::Edit(edit_view) => {
                if !edit_view.handle_key(key) {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            self.mode = Mode::Main;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
