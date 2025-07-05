use crate::config::Config;
use crate::progress::ProgressInfo;
use eframe::egui::{self, CentralPanel, Context, TextEdit, TopBottomPanel};
use eframe::App;
use image::io::Reader as ImageReader;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::copy_request::CopyRequest;
use crate::copy_media;

pub struct IngestApp {
    config: Config,
    destination_input: String,
    progress: Arc<Mutex<ProgressInfo>>, // shared progress information
    logs: Arc<Mutex<Vec<String>>>,
    pending_copy: Arc<Mutex<Option<CopyRequest>>>,
    preview_texture: Option<egui::TextureHandle>,
    last_preview_path: Option<PathBuf>,
}

impl IngestApp {
    pub fn new(
        config: Config,
        progress: Arc<Mutex<ProgressInfo>>,
        logs: Arc<Mutex<Vec<String>>>,
        pending_copy: Arc<Mutex<Option<CopyRequest>>>,
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
            pending_copy,
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

        if let Some(request) = self.pending_copy.lock().unwrap().clone() {
            let mut start_copy = false;
            egui::Window::new("Confirm Copy")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!("Copy from: {}", request.src.display()));
                    ui.label(format!("Copy to: {}", request.dest.display()));
                    ui.label(format!("Files to copy: {}", request.file_count));
                    ui.horizontal(|ui| {
                        if ui.button("Start").clicked() {
                            start_copy = true;
                        }
                        if ui.button("Cancel").clicked() {
                            self.pending_copy.lock().unwrap().take();
                        }
                    });
                });

            if start_copy {
                self.pending_copy.lock().unwrap().take();
                self.logs
                    .lock()
                    .unwrap()
                    .push(format!("Starting copy from {}", request.src.display()));
                {
                    let mut p = self.progress.lock().unwrap();
                    p.message = format!("Copying from {}...", request.src.display());
                }
                let progress = self.progress.clone();
                let logs = self.logs.clone();
                thread::spawn(move || {
                    if let Err(e) = copy_media(&request.src, &request.dest, progress.clone(), logs.clone()) {
                        progress.lock().unwrap().message = format!("Error copying: {}", e);
                    } else {
                        progress.lock().unwrap().message = String::from("Copy completed");
                    }
                });
            }
        }
    }
}

