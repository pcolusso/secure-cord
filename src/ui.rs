use std::{borrow::Cow, future::Future, path::PathBuf, time::Duration};

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

pub async fn run(server_list: Vec<Uhh>, connections_file: PathBuf) -> Result<()> {
    let terminal = ratatui::init();
    App::new(server_list, connections_file).run(terminal).await?;
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
    session: Session,
    server: Server,
    form_fields: Vec<String>,
    active_field: usize,
}

impl EditView {
    fn new(selected: usize, session: Session, server: Server) -> Self {
        let form_fields = vec![
            server.identifier.clone(),
            server.name.clone(),
            server.env.clone(),
            server.host_port.to_string(),
            server.dest_port.to_string(),
        ];
        Self {
            selected,
            stdout: Vec::new(),
            scroll: 0,
            session,
            server,
            form_fields,
            active_field: 0,
        }
    }

    async fn update(&mut self) {
        self.stdout = self.session.stdout().await;
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => {
                self.active_field = self.active_field.saturating_sub(1);
                true
            }
            KeyCode::Down => {
                self.active_field = (self.active_field + 1).min(self.form_fields.len() - 1);
                true
            }
            KeyCode::Left => {
                if let Some(field) = self.form_fields.get_mut(self.active_field) {
                    if !field.is_empty() {
                        field.pop();
                    }
                }
                true
            }
            KeyCode::Char(c) => {
                if let Some(field) = self.form_fields.get_mut(self.active_field) {
                    field.push(c);
                }
                true
            }
            KeyCode::Backspace => {
                if let Some(field) = self.form_fields.get_mut(self.active_field) {
                    let _ = field.pop();
                }
                true
            }
            _ => false,
        }
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .split(area);

        let form_block = Block::default()
            .title("Edit Server")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        f.render_widget(form_block, chunks[0]);

        let form_fields = vec![
            ("Instance ID", &self.form_fields[0]),
            ("Name", &self.form_fields[1]),
            ("Environment", &self.form_fields[2]),
            ("Source Port", &self.form_fields[3]),
            ("Destination Port", &self.form_fields[4]),
        ];

        let form_items: Vec<Paragraph> = form_fields.iter().enumerate().map(|(i, (label, value))| {
            let style = if i == self.active_field {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Paragraph::new(format!("{}: {}", label, value))
                .style(style)
                .block(Block::bordered())
        }).collect();

        let form_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(4); form_items.len()])
            .split(chunks[0].inner(Margin::new(1, 1)));

        for (i, item) in form_items.into_iter().enumerate() {
            f.render_widget(item, form_layout[i]);
        }

        // Output block
        let output_block = Block::default()
            .title("SSM Output")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        f.render_widget(output_block, chunks[1]);

        // Limit scroll to reasonable bounds
        self.scroll = self.scroll.min(self.stdout.len().saturating_sub(1));

        let visible_lines = chunks[1].height.saturating_sub(2) as usize;
        let start_idx = self.stdout.len().saturating_sub(self.scroll + visible_lines);
        let end_idx = start_idx + visible_lines;
        let visible_text = self.stdout[start_idx..end_idx.min(self.stdout.len())].join("\n");

        let stdout_para = Paragraph::new(visible_text)
            .block(Block::default().title(format!(
                "{} lines",
                self.stdout.len()
            )));
        f.render_widget(stdout_para, chunks[1].inner(Margin::new(1, 1)));
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
    event_stream: EventStream,
    connections_file: PathBuf,
}

impl App {
    fn new(server_list: Vec<Uhh>, connections_file: PathBuf) -> Self {
        let mut res = App {
            mode: Mode::Main,
            server_list,
            table_state: TableState::default(),
            event_stream: EventStream::default(),
            running: false,
            editing: None,
            connections_file
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
                    edit_view.update().await;
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
                        let (session, server, _) = &self.server_list[sel];
                        let mut edit_view = EditView::new(sel, session.clone(), server.clone());
                        edit_view.update().await;
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
                KeyCode::Char('s') => {
                    let servers: Vec<_> = self.server_list.iter().map(|(_, server, _)| server.clone()).collect();
                    let path = self.connections_file.clone();
                    tokio::spawn(async move {
                        if let Err(e) = crate::servers::save(path, &servers).await {
                            eprintln!("Failed to save servers: {}", e);
                        }
                    });
                }
                _ => {}
            },
            Mode::Edit(edit_view) => {
                if !edit_view.handle_key(key) {
                    match key.code {
                        KeyCode::Enter => {
                            if let Mode::Edit(edit_view) = std::mem::replace(&mut self.mode, Mode::Main) {
                                let (session, server, _) = &mut self.server_list[edit_view.selected];
                                // Update the server with form field values
                                server.identifier = edit_view.form_fields[0].clone();
                                server.name = edit_view.form_fields[1].clone();
                                server.env = edit_view.form_fields[2].clone();
                                server.host_port = edit_view.form_fields[3].parse().unwrap_or(server.host_port);
                                server.dest_port = edit_view.form_fields[4].parse().unwrap_or(server.dest_port);

                                let session = session.clone();
                                let identifer = server.identifier.clone();
                                let env = server.env.clone();
                                let host_port = server.host_port;
                                let dest_port = server.dest_port;
                                tokio::spawn(async move {
                                    session.update(identifer, env, host_port, dest_port).await;
                                });
                            }
                        }
                        KeyCode::Esc => {
                            self.mode = Mode::Main;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
