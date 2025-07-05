mod app;
mod config;

use crate::app::IngestApp;
use crate::config::Config;
use eframe;
use fs_extra::dir::{copy as copy_dir, CopyOptions};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::{Disks, System};
use std::thread;

fn copy_media(src: &PathBuf, dest: &PathBuf) -> std::io::Result<()> {
    let mut options = CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;
    copy_dir(src, dest, &options)
        .map(|_| ())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

fn main() -> eframe::Result<()> {
    let config = Config::load();
    let status = Arc::new(Mutex::new(String::from("Waiting for drive...")));
    let status_clone = status.clone();
    let dest = config.destination.clone();

    // Spawn background thread to watch for new drives
    thread::spawn(move || {
        let mut _system = System::new();
        let mut disks = Disks::new_with_refreshed_list();
        let mut known: HashSet<PathBuf> = HashSet::new();
        loop {
            disks.refresh(true);
            let mut current = HashSet::new();
            for disk in disks.list() {
                if let Some(mount) = disk.mount_point().to_str() {
                    current.insert(PathBuf::from(mount));
                }
            }

            // check for new mount points
            for mount in current.difference(&known) {
                if let Some(dest_path) = &dest {
                    let mut status_lock = status_clone.lock().unwrap();
                    *status_lock = format!("Copying from {}...", mount.display());
                    let src = mount.clone();
                    if let Err(e) = copy_media(&src, dest_path) {
                        *status_lock = format!("Error copying: {}", e);
                    } else {
                        *status_lock = String::from("Copy completed");
                    }
                }
            }

            known = current;
            thread::sleep(Duration::from_secs(5));
        }
    });

    let app = IngestApp::new(config, status);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Ingest App",
        native_options,
        Box::new(|_| Box::new(app)),
    )
}
