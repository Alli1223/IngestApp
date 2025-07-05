use std::path::PathBuf;

#[derive(Clone, Default)]
pub struct ProgressInfo {
    pub message: String,
    pub total_bytes: u64,
    pub copied_bytes: u64,
    pub file_total_bytes: u64,
    pub file_copied_bytes: u64,
    pub current_file: String,
    pub preview_path: Option<PathBuf>,
    pub speed: f64,
}

impl ProgressInfo {
    pub fn total_progress(&self) -> f32 {
        if self.total_bytes == 0 { 0.0 } else { self.copied_bytes as f32 / self.total_bytes as f32 }
    }

    pub fn file_progress(&self) -> f32 {
        if self.file_total_bytes == 0 { 0.0 } else { self.file_copied_bytes as f32 / self.file_total_bytes as f32 }
    }
}
