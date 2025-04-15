use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind, KeyCode};
use futures::{FutureExt, StreamExt};
use ratatui::{
    buffer::Buffer, layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Stylize}, widgets::{Block, BorderType, Borders, Cell, ListState, Paragraph, Row, StatefulWidget, Table, TableState, Widget}, DefaultTerminal, Frame
};

use crate::servers::Server;

pub async fn run(server_list: Vec<Server>) -> Result<()> {
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


#[derive(Debug)]
pub struct App {
    mode: Mode,
    server_list: Vec<Server>,
    table_state: TableState,
    running: bool,
    editing: Option<Server>,
    event_stream: EventStream
}

impl App {
    fn new(server_list: Vec<Server>) -> Self {
        App {
            mode: Mode::Main,
            server_list,
            table_state: TableState::default(),
            event_stream: EventStream::default(),
            running: false,
            editing: None
        }
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
                f.render_widget(block, cunks[0]);

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
        // 'Fuse' means as in like blowing a fuse.
        // It means after the Future returns None, do not poll it again.
        // Streams should be fused, if they are used in a select! macro.
        tokio::select! {
            event = self.event_stream.next().fuse() => {
                match event {
                    Some(Ok(evt)) => {
                        match evt {
                            Event::Key(key)
                                if key.kind == KeyEventKind::Press
                                    => self.on_key_event(key),
                            Event::Mouse(_) => {}
                            Event::Resize(_, _) => {}
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match &mut self.mode {
            Mode::Main => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.running = false,
                _ => {}
            },
            Mode::Edit { selected } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Main,
                _ => {}
            },
        }
    }
}
