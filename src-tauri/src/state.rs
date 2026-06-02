use serde::{Deserialize, Serialize};

// Passed to the preview window so it knows which file to show.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewInfo {
    pub path: String,   // absolute path to the recorded file
    pub format: String, // "mp4", "webm", or "gif"
}
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Mp4,
    Gif,
    WebM,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Quality {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct AppState {
    pub region: Mutex<Option<CaptureRegion>>,
    pub recording: AtomicBool,
    pub stop_flag: Mutex<Option<Arc<AtomicBool>>>,
    pub selected_display_index: Mutex<Option<usize>>,
    // User-chosen save folder. None = use platform default (~/Movies on macOS).
    pub save_folder: Mutex<Option<String>>,
    pub output_format: Mutex<OutputFormat>,
    // Set just before opening the preview window; preview.js reads it on load.
    pub pending_preview: Mutex<Option<PreviewInfo>>,
    pub fps: Mutex<u32>,
    pub quality: Mutex<Quality>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            region: Mutex::new(None),
            recording: AtomicBool::new(false),
            stop_flag: Mutex::new(None),
            selected_display_index: Mutex::new(None),
            save_folder: Mutex::new(None),
            output_format: Mutex::new(OutputFormat::Mp4),
            pending_preview: Mutex::new(None),
            fps: Mutex::new(30),
            quality: Mutex::new(Quality::Medium),
        }
    }

    // Returns true if currently recording.
    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Relaxed)
    }
}
