use std::sync::mpsc;
use eframe::egui;
use anyhow::Result;
use tokio::runtime::Runtime;

// NB: Footgun with this, the recv channel will never close, because there's always a reference to
// the sender!
struct Chan<T> {
    rx: mpsc::Receiver<T>,
    tx: mpsc::Sender<T>,
}

impl<T> From<(mpsc::Sender<T>, mpsc::Receiver<T>)> for Chan<T> {
    fn from((tx, rx): (mpsc::Sender<T>, mpsc::Receiver<T>)) -> Self {
        Self { rx, tx }
    }
}


// Messages to be processed by the UI. Arbiter will send a snapshot of their state via PollState
// message
enum AppMessage {
}


pub async fn run() -> Result<()> {
    let rt = Runtime::new()?;
    let arb_chan: Chan<ArbiterMessage> = mpsc::channel().into();
    let app_chan: Chan<AppMessage> = mpsc::channel().into();
    let arb_outbox = app_chan.tx.clone();
    let app_outbox = arb_chan.tx.clone();

    let mut arbiter = Arbiter { inbox: arb_chan, outbox: arb_outbox } ;
    let app = App { inbox: app_chan, outbox: app_outbox, value: 0.0, label: "".into() };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    tokio::spawn(async move {
        loop {
            arbiter.update().await;
        }
    });

    eframe::run_native("secure-cords", native_options, Box::new(|cc| Ok(Box::new(app)))).expect("idk you should probably spawn a runtime and ensure UI is on the main thread, bub.");
    
    Ok(())
}

// Messages to be processed by the Arbiter.
enum ArbiterMessage {

}

struct Arbiter {
    inbox: Chan<ArbiterMessage>,
    outbox: mpsc::Sender<AppMessage>,
}

impl Arbiter {
    async fn update(&mut self) {
        
    }
}


struct App {
    inbox: Chan<AppMessage>,
    outbox: mpsc::Sender<ArbiterMessage>,
    value: f32,
    label: String
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, f: &mut eframe::Frame) {
        // NB: Remember, try_recv on the sync channel avoids blocking!
        for msg in self.inbox.rx.try_iter() {
            match msg {

            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

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
