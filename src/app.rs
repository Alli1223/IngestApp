use crate::config::Config;
use crate::progress::ProgressInfo;
use eframe::egui::{self, CentralPanel, Context, TextEdit, TopBottomPanel};
use eframe::App;
use image::io::Reader as ImageReader;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct IngestApp {
    config: Config,
    destination_input: String,
    progress: Arc<Mutex<ProgressInfo>>, // shared progress information
    logs: Arc<Mutex<Vec<String>>>,
    preview_texture: Option<egui::TextureHandle>,
    last_preview_path: Option<PathBuf>,
}

impl IngestApp {
    pub fn new(
        config: Config,
        progress: Arc<Mutex<ProgressInfo>>,
        logs: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        let destination_input = config
            .destination
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            config,
            destination_input,
            progress,
            logs,
            preview_texture: None,
            last_preview_path: None,
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
            let progress = self.progress.lock().unwrap().clone();
            ui.label(progress.message.clone());
            ui.add(egui::ProgressBar::new(progress.total_progress()).show_percentage());
            ui.add(egui::ProgressBar::new(progress.file_progress()).show_percentage());
            ui.label(format!("Speed: {:.2} MB/s", progress.speed / 1_048_576.0));

            if let Some(path) = progress.preview_path {
                if self.last_preview_path.as_ref() != Some(&path) {
                    if let Ok(reader) = ImageReader::open(&path) {
                        if let Ok(img) = reader.decode() {
                            let size = [img.width() as usize, img.height() as usize];
                            let color = egui::ColorImage::from_rgba_unmultiplied(
                                size,
                                img.to_rgba8().as_flat_samples().as_slice(),
                            );
                            self.preview_texture = Some(ctx.load_texture("preview", color, Default::default()));
                            self.last_preview_path = Some(path.clone());
                        }
                    }
                }
                if let Some(tex) = &self.preview_texture {
                    let max = 200.0;
                    let size = tex.size_vec2();
                    let scale = (max / size.x).min(max / size.y).min(1.0);
                    ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size * scale));
                }
            }

            ui.separator();
            let logs = self.logs.lock().unwrap();
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for log in logs.iter() {
                    ui.label(log);
                }
            });
        });
    }
}

