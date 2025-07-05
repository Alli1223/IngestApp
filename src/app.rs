use crate::config::Config;
use crate::progress::ProgressInfo;
use eframe::egui::{self, CentralPanel, Context, TextEdit, TopBottomPanel};
use eframe::App;
use image::io::Reader as ImageReader;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::copy_request::CopyRequest;
use crate::copy_media;
use std::process::Command;

pub struct IngestApp {
    config: Config,
    destination_input: String,
    progress: Arc<Mutex<ProgressInfo>>, // shared progress information
    logs: Arc<Mutex<Vec<String>>>,
    pending_copy: Arc<Mutex<Vec<CopyRequest>>>,
    known: Arc<Mutex<HashSet<PathBuf>>>,
    selected_index: usize,
    preview_texture: Option<egui::TextureHandle>,
    last_preview_path: Option<PathBuf>,
}

impl IngestApp {
    pub fn new(
        config: Config,
        progress: Arc<Mutex<ProgressInfo>>,
        logs: Arc<Mutex<Vec<String>>>,
        pending_copy: Arc<Mutex<Vec<CopyRequest>>>,
        known: Arc<Mutex<HashSet<PathBuf>>>,
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
            known,
            selected_index: 0,
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
                ui.add_space(10.0);
                if ui.button("Refresh").clicked() {
                    crate::scan_for_drives(
                        &self.config.destination,
                        &self.progress,
                        &self.logs,
                        &self.pending_copy,
                        &self.known,
                    );
                }
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            let progress = self.progress.lock().unwrap().clone();
            ui.label(progress.message.clone());
            ui.add_space(5.0);
            ui.add(egui::ProgressBar::new(progress.total_progress()).show_percentage());
            ui.add_space(5.0);
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

            if Self::sd_reader_present() {
                let mounts = Self::mounted_media_dirs();
                if mounts.is_empty() {
                    for dev in Self::list_unmounted_devices() {
                        if ui.button(format!("Mount {}", dev)).clicked() {
                            Self::mount_device(&dev, &self.logs);
                        }
                    }
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

        let mut start_copy: Option<CopyRequest> = None;
        {
            let mut pending = self.pending_copy.lock().unwrap();
            if !pending.is_empty() {
                if self.selected_index >= pending.len() {
                    self.selected_index = 0;
                }
                egui::Window::new("Select Drive")
                    .collapsible(false)
                    .show(ctx, |ui| {
                        for (i, req) in pending.iter().enumerate() {
                            ui.radio_value(
                                &mut self.selected_index,
                                i,
                                format!("{} -> {}", req.src.display(), req.dest.display()),
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Go").clicked() {
                                start_copy = Some(pending.remove(self.selected_index));
                                if self.selected_index >= pending.len() {
                                    self.selected_index = 0;
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                pending.remove(self.selected_index);
                                if self.selected_index >= pending.len() {
                                    self.selected_index = 0;
                                }
                            }
                        });
                    });
            }
        }

        if let Some(request) = start_copy {
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

impl IngestApp {
    fn sd_reader_present() -> bool {
        if let Ok(out) = Command::new("lsusb").output() {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            text.contains("05e3:0743") || text.contains("cardreader")
        } else {
            false
        }
    }

    fn list_unmounted_devices() -> Vec<String> {
        if let Ok(out) = Command::new("lsblk")
            .args(&["-rnpo", "NAME,MOUNTPOINT"])
            .output()
        {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| {
                    let mut p = l.split_whitespace();
                    let name = p.next()?;
                    if p.next().unwrap_or("").is_empty() {
                        Some(name.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn mounted_media_dirs() -> Vec<PathBuf> {
        if let Ok(user) = std::env::var("USER") {
            let base = PathBuf::from(format!("/media/{}", user));
            if let Ok(entries) = std::fs::read_dir(base) {
                return entries.flatten().map(|e| e.path()).collect();
            }
        }
        Vec::new()
    }

    fn mount_device(dev: &str, logs: &Arc<Mutex<Vec<String>>>) {
        if let Ok(out) = Command::new("udisksctl")
            .args(&["mount", "-b", dev])
            .output()
        {
            logs.lock()
                .unwrap()
                .push(format!("Mounted {}: {}", dev, String::from_utf8_lossy(&out.stdout)));
        } else {
            logs.lock().unwrap().push(format!("Failed to mount {}", dev));
        }
    }
}

