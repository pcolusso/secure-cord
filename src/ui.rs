use anyhow::Result;
use eframe::egui;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use crate::servers::{Server, ServerList};

trait SortableCompare {
    fn compare<T: Ord>(&self, a: &T, b: &T) -> Ordering;
}

impl SortableCompare for SortDirection {
    fn compare<T: Ord>(&self, a: &T, b: &T) -> Ordering {
        match self {
            SortDirection::Ascending => a.cmp(b),
            SortDirection::Descending => b.cmp(a),
        }
    }
}

pub fn run(servers: ServerList) -> Result<()> {
    let app = App::new(servers);

    // Embed custom font
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "berkeley_mono".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../BerkeleyMonoVariable-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "berkeley_mono".to_owned());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "secure-cords",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .expect("idk you should probably spawn a runtime and ensure UI is on the main thread, bub.");

    Ok(())
}

#[derive(PartialEq)]
enum SortDirection {
    Ascending,
    Descending,
}

struct SortState {
    column: usize,
    direction: SortDirection,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            column: 0,
            direction: SortDirection::Ascending,
        }
    }
}

// State that can be modified by other threads
struct State {
    servers: ServerList,
}

// UI specific state
struct App {
    state: Arc<Mutex<State>>,
    sort: SortState,
    selected_row: Option<usize>,
    edit_window_open: Option<usize>,
}

impl App {
    fn new(servers: ServerList) -> Self {
        let state = Arc::new(Mutex::new(State { servers }));
        Self {
            state,
            sort: SortState::default(),
            selected_row: None,
            edit_window_open: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, f: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("servers_grid")
                    .striped(true)
                    .min_col_width(100.0)
                    .show(ui, |ui| {
                        // Header row with clickable headers
                        let mut state = self.state.lock().unwrap();

                        let headers = [
                            "Name",
                            "Identifier",
                            "Environment",
                            "Host Port",
                            "Dest Port",
                            "Status",
                        ];
                        for (col_idx, header) in headers.iter().enumerate() {
                            let response = ui.selectable_label(
                                self.sort.column == col_idx,
                                if self.sort.column == col_idx {
                                    match self.sort.direction {
                                        SortDirection::Ascending => format!("{} 👆", header),
                                        SortDirection::Descending => format!("{} 👇", header),
                                    }
                                } else {
                                    header.to_string()
                                },
                            );

                            if response.clicked() {
                                if self.sort.column == col_idx {
                                    self.sort.direction = match self.sort.direction {
                                        SortDirection::Ascending => SortDirection::Descending,
                                        SortDirection::Descending => SortDirection::Ascending,
                                    };
                                } else {
                                    self.sort.column = col_idx;
                                    self.sort.direction = SortDirection::Ascending;
                                }

                                match col_idx {
                                    0 => state.servers.sort_by(|a, b| {
                                        self.sort.direction.compare(&a.name, &b.name)
                                    }),
                                    1 => state.servers.sort_by(|a, b| {
                                        self.sort.direction.compare(&a.identifier, &b.identifier)
                                    }),
                                    2 => state.servers.sort_by(|a, b| {
                                        self.sort.direction.compare(&a.env, &b.env)
                                    }),
                                    3 => state.servers.sort_by(|a, b| {
                                        self.sort.direction.compare(&a.host_port, &b.host_port)
                                    }),
                                    4 => state.servers.sort_by(|a, b| {
                                        self.sort.direction.compare(&a.dest_port, &b.dest_port)
                                    }),
                                    _ => {}
                                }
                            }
                        }
                        ui.end_row();

                        // Data rows
                        for (row_idx, server) in state.servers.iter().enumerate() {
                            let is_selected = self.selected_row == Some(row_idx);
                            let name_response = ui.selectable_label(is_selected, &server.name);
                            if name_response.clicked() {
                                self.selected_row = Some(row_idx);
                                if name_response.double_clicked() {
                                    self.edit_window_open = Some(row_idx);
                                }
                            }
                            ui.label(&server.identifier);
                            ui.label(&server.env);
                            ui.label(server.host_port.to_string());
                            ui.label(server.dest_port.to_string());
                            // TODO
                            ui.label("Not connected");
                            ui.end_row();
                        }
                    });
            });

            // Bottom panel with action buttons
            egui::TopBottomPanel::bottom("toolbar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("➕ Add").clicked() {
                        let mut state = self.state.lock().unwrap();
                        state.servers.push(Server {
                            identifier: "new-instance".to_string(),
                            env: "dev".to_string(),
                            host_port: 8080,
                            name: "New Server".to_string(),
                            dest_port: 8080,
                        });
                    }

                    let mut state = self.state.lock().unwrap();
                    let is_row_selected = self.selected_row.is_some();

                    if ui
                        .add_enabled(is_row_selected, egui::Button::new("✏️ Modify"))
                        .clicked()
                    {
                        if let Some(idx) = self.selected_row {
                            // TODO: Open edit dialog for state.servers[idx]
                        }
                    }

                    if ui
                        .add_enabled(is_row_selected, egui::Button::new("🗑️ Delete"))
                        .clicked()
                    {
                        if let Some(idx) = self.selected_row {
                            state.servers.remove(idx);
                            self.selected_row = None;
                        }
                    }

                    if ui.button("💾 Save").clicked() {
                        let state = self.state.lock().unwrap();
                        if let Err(e) = state.servers.save() {
                            eprintln!("Failed to save: {}", e);
                        }
                    }
                });
            });
        });

        // Render edit window if needed
        if let Some(row_idx) = self.edit_window_open {
            let state = self.state.lock().unwrap();
            if let Some(server) = state.servers.get(row_idx) {
                egui::Window::new("Edit Server")
                    .open(&mut self.edit_window_open.is_some())
                    .show(ctx, |ui| {
                        ui.label(format!("Editing: {}", server.name));
                        // TODO: Add actual edit controls here
                        if ui.button("Close").clicked() {
                            self.edit_window_open = None;
                        }
                    });
            } else {
                // Row was deleted while window was open
                self.edit_window_open = None;
            }
        }
    }
}
