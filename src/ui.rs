use anyhow::Result;
use eframe::egui;
use std::sync::{mpsc, Arc, Mutex};

use crate::Server;

pub fn run(servers: Vec<Server>) -> Result<()> {
    let app = App::new(servers);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "secure-cords",
        native_options,
        Box::new(|cc| Ok(Box::new(app))),
    )
    .expect("idk you should probably spawn a runtime and ensure UI is on the main thread, bub.");

    Ok(())
}

struct State {
    servers: Vec<Server>,
}

struct App {
    state: Arc<Mutex<State>>,
}

impl App {
    fn new(servers: Vec<Server>) -> Self {
        let state = Arc::new(Mutex::new(State { servers }));
        Self { state }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, f: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
