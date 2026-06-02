use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::{
    capture::capture_loop,
    state::{AppState, CaptureRegion, OutputFormat, PreviewInfo, Quality},
};

// ── Monitor enumeration ───────────────────────────────────────────────────────

// Info about one monitor, serialized and sent to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    // Position of this monitor in the global screen coordinate space.
    // Primary monitor is at (0, 0); secondary monitors are at offsets.
    pub x: i32,
    pub y: i32,
}

// Returns all connected monitors with their positions and sizes.
// The frontend uses this to populate the monitor picker dropdown.
#[tauri::command]
pub fn get_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .into_iter()
        .enumerate()
        .map(|(i, m)| MonitorInfo {
            index: i,
            name: m.name().cloned().unwrap_or_else(|| format!("Display {}", i + 1)),
            width: m.size().width,
            height: m.size().height,
            x: m.position().x,
            y: m.position().y,
        })
        .collect())
}

// Sets the capture region to the full area of the monitor at the given index.
// Replaces the previous select_full_screen for single-monitor setups.
#[tauri::command]
pub fn select_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
    index: usize,
) -> Result<(), String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let m = monitors
        .get(index)
        .ok_or_else(|| format!("No monitor at index {index}"))?;

    // Store physical pixel dimensions — scap outputs frames in physical pixels.
    let region = CaptureRegion {
        x: 0,
        y: 0,
        width: m.size().width,
        height: m.size().height,
    };

    *state.region.lock().unwrap() = Some(region.clone()); // TODO: handle error
    *state.selected_display_index.lock().unwrap() = Some(index); // TODO: handle error

    if let Some(win) = app.get_webview_window("main") {
        win.emit("region-selected", region).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Region selection ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn start_region_select(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.hide().map_err(|e| e.to_string())?;
    }

    // Open the overlay on whichever monitor the user selected, not always the primary.
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let idx = state.selected_display_index.lock().unwrap().unwrap_or(0); // TODO: handle error
    let monitor = monitors
        .get(idx)
        .or_else(|| monitors.first())
        .ok_or("No monitors found".to_string())?;

    let phys = monitor.size();
    let scale = monitor.scale_factor();
    let pos = monitor.position();

    // Physical → logical pixel conversion for both size and position.
    let logical_w = phys.width as f64 / scale;
    let logical_h = phys.height as f64 / scale;
    let logical_x = pos.x as f64 / scale;
    let logical_y = pos.y as f64 / scale;

    WebviewWindowBuilder::new(&app, "overlay", WebviewUrl::App("overlay.html".into()))
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .inner_size(logical_w, logical_h)
        .position(logical_x, logical_y)
        .skip_taskbar(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn set_region(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let mut region = state.region.lock().unwrap(); // TODO: handle error
    *region = Some(CaptureRegion { x, y, width, height });

    if let Some(overlay) = app.get_webview_window("overlay") {
        overlay.close().map_err(|e| e.to_string())?;
    }
    if let Some(main) = app.get_webview_window("main") {
        main.emit("region-selected", region.clone()).map_err(|e| e.to_string())?;
        main.show().map_err(|e| e.to_string())?;
        main.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_region_select(app: AppHandle) -> Result<(), String> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        overlay.close().map_err(|e| e.to_string())?;
    }
    if let Some(main) = app.get_webview_window("main") {
        main.show().map_err(|e| e.to_string())?;
        main.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn select_full_screen(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No primary monitor found".to_string())?;
    let phys = monitor.size();

    let full = CaptureRegion { x: 0, y: 0, width: phys.width, height: phys.height };
    let mut region = state.region.lock().unwrap(); // TODO: handle error
    *region = Some(full.clone());

    if let Some(main) = app.get_webview_window("main") {
        main.emit("region-selected", full).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_region(state: State<'_, AppState>) -> Result<Option<CaptureRegion>, String> {
    let region = state.region.lock().unwrap(); // TODO: handle error
    Ok(region.clone())
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_fps(state: State<'_, AppState>, fps: u32) -> Result<(), String> {
    *state.fps.lock().unwrap() = fps; // TODO: handle error
    Ok(())
}

#[tauri::command]
pub fn set_quality(state: State<'_, AppState>, quality: String) -> Result<(), String> {
    let q = match quality.as_str() {
        "high" => Quality::High,
        "low"  => Quality::Low,
        _      => Quality::Medium,
    };
    *state.quality.lock().unwrap() = q; // TODO: handle error
    Ok(())
}

#[derive(serde::Serialize)]
pub struct Settings {
    pub fps: u32,
    pub quality: String,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    let fps = *state.fps.lock().unwrap(); // TODO: handle error
    let quality = match *state.quality.lock().unwrap() { // TODO: handle error
        Quality::High   => "high",
        Quality::Medium => "medium",
        Quality::Low    => "low",
    };
    Settings { fps, quality: quality.to_string() }
}

// ── Preview ───────────────────────────────────────────────────────────────────

// Opens a preview window for the just-recorded file.
// Stores path+format in state; preview.js reads it on load via get_preview_info().
#[tauri::command]
pub fn open_preview_window(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    format: String,
) -> Result<(), String> {
    *state.pending_preview.lock().unwrap() = Some(PreviewInfo { // TODO: handle error
        path: path.clone(),
        format,
    });

    // Close any existing preview before opening a new one.
    if let Some(win) = app.get_webview_window("preview") {
        let _ = win.close();
    }

    WebviewWindowBuilder::new(&app, "preview", WebviewUrl::App("preview.html".into()))
        .title("Preview Recording")
        .inner_size(760.0, 540.0)
        .center()
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// Called by preview.js on load to get the file it should display.
#[tauri::command]
pub fn get_preview_info(state: State<'_, AppState>) -> Option<PreviewInfo> {
    state.pending_preview.lock().unwrap().clone() // TODO: handle error
}

// Deletes the recorded file and closes the preview window.
#[tauri::command]
pub fn discard_recording(app: AppHandle, path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    if let Some(win) = app.get_webview_window("preview") {
        win.close().map_err(|e| e.to_string())?;
    }
    // Tell the main window to clear the "Saved:" label.
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit("recording-discarded", ());
    }
    Ok(())
}

// Closes the preview window (file is already saved — nothing else needed).
#[tauri::command]
pub fn close_preview(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("preview") {
        win.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Output format ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_output_format(state: State<'_, AppState>, format: String) -> Result<(), String> {
    let fmt = match format.as_str() {
        "gif"  => OutputFormat::Gif,
        "webm" => OutputFormat::WebM,
        _      => OutputFormat::Mp4,
    };
    *state.output_format.lock().unwrap() = fmt; // TODO: handle error
    Ok(())
}

#[tauri::command]
pub fn get_output_format(state: State<'_, AppState>) -> String {
    match *state.output_format.lock().unwrap() { // TODO: handle error
        OutputFormat::Gif  => "gif".to_string(),
        OutputFormat::WebM => "webm".to_string(),
        OutputFormat::Mp4  => "mp4".to_string(),
    }
}

// ── Save folder ───────────────────────────────────────────────────────────────

// Stores the user-chosen save folder path (path comes from the JS dialog API).
#[tauri::command]
pub fn set_save_folder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    *state.save_folder.lock().unwrap() = Some(path); // TODO: handle error
    Ok(())
}

// Returns the current save folder, or the platform default if none chosen.
#[tauri::command]
pub fn get_save_folder(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    if let Some(folder) = state.save_folder.lock().unwrap().clone() { // TODO: handle error
        return Ok(folder);
    }
    app.path()
        .video_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

// ── Recording ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    do_start(&app, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    do_stop(&app, &state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recording_status(state: State<'_, AppState>) -> bool {
    state.is_recording()
}

// pub so the global-shortcut handler in lib.rs can call them directly.

// Starts the capture thread. Safe to call if already recording (no-op).
pub fn do_start(app: &AppHandle, state: &AppState) -> anyhow::Result<()> {
    if state.is_recording() {
        return Ok(());
    }

    let region       = state.region.lock().unwrap().clone();            // TODO: handle error
    let display_index = *state.selected_display_index.lock().unwrap(); // TODO: handle error
    let format       = state.output_format.lock().unwrap().clone();    // TODO: handle error
    let fps          = *state.fps.lock().unwrap();                     // TODO: handle error
    let quality      = state.quality.lock().unwrap().clone();          // TODO: handle error

    // Use the user-chosen folder if set, otherwise fall back to ~/Movies.
    let video_dir = match state.save_folder.lock().unwrap().clone() { // TODO: handle error
        Some(folder) => std::path::PathBuf::from(folder),
        None => app.path().video_dir()?,
    };
    std::fs::create_dir_all(&video_dir)?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ext = match format {
        OutputFormat::Gif  => "gif",
        OutputFormat::WebM => "webm",
        OutputFormat::Mp4  => "mp4",
    };
    let output_path = video_dir.join(format!("pkap-{ts}.{ext}"));

    let stop = Arc::new(AtomicBool::new(false));
    *state.stop_flag.lock().unwrap() = Some(Arc::clone(&stop)); // TODO: handle error
    state.recording.store(true, Ordering::Relaxed);

    let app_clone = app.clone();
    let path_clone = output_path.clone();

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide(); // TODO: handle error
    }

    std::thread::spawn(move || {
        capture_loop(app_clone, stop, region, path_clone, display_index, format, fps, quality);
    });

    app.emit("recording-status", true)?;
    app.emit("recording-output-path", output_path.to_string_lossy().as_ref())?;
    Ok(())
}

// Signals the capture thread to stop. Safe to call if not recording (no-op).
pub fn do_stop(app: &AppHandle, state: &AppState) -> anyhow::Result<()> {
    if !state.is_recording() {
        return Ok(());
    }

    // take() removes the flag from state, leaving None. The Arc keeps the flag alive
    // until the capture thread drops its own clone — at which point the thread has exited.
    if let Some(flag) = state.stop_flag.lock().unwrap().take() { // TODO: handle error
        flag.store(true, Ordering::Relaxed);
    }

    state.recording.store(false, Ordering::Relaxed);
    app.emit("recording-status", false)?;

    // Bring the main window back so the user can see the "Saved:" message.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show(); // TODO: handle error
        let _ = win.set_focus(); // TODO: handle error
    }
    Ok(())
}
