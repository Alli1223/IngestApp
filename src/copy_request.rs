use std::path::PathBuf;

#[derive(Clone)]
pub struct CopyRequest {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub file_count: usize,
}
