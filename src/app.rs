use crate::config::Config;
use eframe::egui::{CentralPanel, Context, TextEdit, TopBottomPanel};
use eframe::App;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct IngestApp {
    config: Config,
    destination_input: String,
    status: Arc<Mutex<String>>, // status messages from background thread
}

impl IngestApp {
    pub fn new(config: Config, status: Arc<Mutex<String>>) -> Self {
        let destination_input = config
            .destination
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            config,
            destination_input,
            status,
        }
    }
}

impl App for IngestApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Destination:");
                ui.add(TextEdit::singleline(&mut self.destination_input).desired_width(300.0));
                if ui.button("Save").clicked() {
                    if !self.destination_input.is_empty() {
                        self.config.destination = Some(PathBuf::from(&self.destination_input));
                        self.config.save();
                    }
                }
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            let status = self.status.lock().unwrap().clone();
            ui.label(status);
        });
    }
}

