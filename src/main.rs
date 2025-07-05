mod app;
mod config;
mod progress;
mod copy_request;

use crate::app::IngestApp;
use crate::config::Config;
use crate::progress::ProgressInfo;
use crate::copy_request::CopyRequest;
use eframe;
use fs_extra::dir::{copy_with_progress, CopyOptions, TransitProcessResult};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::{Disks, System};
use std::thread;
use std::time::Instant;
use std::fs;
use std::path::Path;

pub fn copy_media(
    src: &PathBuf,
    dest: &PathBuf,
    progress: Arc<Mutex<ProgressInfo>>,
    logs: Arc<Mutex<Vec<String>>>,
) -> std::io::Result<()> {
    let mut options = CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;
    let start = Instant::now();
    let mut last_file = String::new();
    copy_with_progress(src, dest, &options, |info| {
        {
            let mut p = progress.lock().unwrap();
            p.total_bytes = info.total_bytes;
            p.copied_bytes = info.copied_bytes;
            p.file_total_bytes = info.file_total_bytes;
            p.file_copied_bytes = info.file_bytes_copied;
            p.current_file = info.file_name.clone();
            p.message = format!("Copying {}", info.file_name);
            p.speed = info.copied_bytes as f64 / start.elapsed().as_secs_f64();
            let path = src.join(&info.file_name);
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| matches!(e.to_lowercase().as_str(), "png" | "jpg" | "jpeg"))
                .unwrap_or(false)
            {
                p.preview_path = Some(path);
            } else {
                p.preview_path = None;
            }
        }
        if info.file_name != last_file {
            logs.lock().unwrap().push(format!("Copying {}", info.file_name));
            last_file = info.file_name.clone();
        }
        TransitProcessResult::ContinueOrAbort
    })
    .map(|_| ())
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    // Record destination path on the source drive for future runs
    let ingest_file = src.join("ingest.txt");
    let _ = fs::write(ingest_file, dest.to_string_lossy().as_ref());
    Ok(())
}

fn count_files(path: &Path) -> usize {
    fn helper(p: &Path, count: &mut usize) {
        if let Ok(entries) = fs::read_dir(p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    helper(&path, count);
                } else {
                    *count += 1;
                }
            }
        }
    }
    let mut count = 0usize;
    helper(path, &mut count);
    count
}

fn main() -> eframe::Result<()> {
    let config = Config::load();
    let progress = Arc::new(Mutex::new(ProgressInfo::default()));
    progress.lock().unwrap().message = "Waiting for drive...".to_string();
    let progress_clone = progress.clone();
    let logs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let logs_thread = logs.clone();
    let dest = config.destination.clone();
    let pending_copy: Arc<Mutex<Vec<CopyRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_copy_watch = pending_copy.clone();

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
                let ingest_path = mount.join("ingest.txt");
                let dest_from_file = fs::read_to_string(&ingest_path).ok().map(|s| PathBuf::from(s.trim()));
                let dest_path = dest_from_file.or_else(|| dest.clone());
                if let Some(dest_path) = dest_path {
                    let file_count = count_files(mount);
                    {
                        let mut p = progress_clone.lock().unwrap();
                        p.message = format!("Drive {} detected", mount.display());
                    }
                    logs_thread
                        .lock()
                        .unwrap()
                        .push(format!("Drive {} detected", mount.display()));
                    pending_copy_watch.lock().unwrap().push(CopyRequest {
                        src: mount.clone(),
                        dest: dest_path,
                        file_count,
                    });
                }
            }

            known = current;
            thread::sleep(Duration::from_secs(5));
        }
    });

    let app = IngestApp::new(config, progress, logs, pending_copy);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Ingest App",
        native_options,
        Box::new(|_| Box::new(app)),
    )
}
