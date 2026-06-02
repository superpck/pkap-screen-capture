// Screen capture loop — runs on its own thread.
// Pipes raw BGRA frames from scap into an ffmpeg subprocess that encodes MP4.
// ffmpeg must be installed: brew install ffmpeg
use scap::{
    capturer::{Capturer, Options},
    frame::{Frame, FrameType, VideoFrame},
    get_all_targets, Target,
};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::{CaptureRegion, OutputFormat, Quality};

pub fn capture_loop(
    app: AppHandle,
    stop: Arc<AtomicBool>,
    region: Option<CaptureRegion>,
    output_path: PathBuf,
    display_index: Option<usize>,
    format: OutputFormat,
    fps: u32,
    quality: Quality,
) {
    if !scap::has_permission() {
        scap::request_permission();
        let _ = app.emit(
            "recording-error",
            "Screen Recording permission required. Grant it in System Settings, then try again.",
        );
        return;
    }

    // Build the scap target. Filter get_all_targets() down to Display-only,
    // then pick by the stored index (same ordering as Tauri's available_monitors()).
    let display_targets: Vec<Target> = get_all_targets()
        .into_iter()
        .filter(|t| matches!(t, Target::Display(_)))
        .collect();

    let target = display_index
        .and_then(|i| display_targets.into_iter().nth(i));

    let mut capturer = match Capturer::build(Options {
        fps,
        show_cursor: true,
        show_highlight: false,
        target,
        output_type: FrameType::BGRAFrame,
        ..Default::default()
    }) {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit("recording-error", format!("Failed to start capture: {e}"));
            return;
        }
    };

    capturer.start_capture();

    // Grab the first frame to learn the actual pixel dimensions before starting ffmpeg.
    let first_frame = match capturer.get_next_frame() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[pkap] no first frame: {e}");
            capturer.stop_capture();
            return;
        }
    };

    // In scap 0.1.0-beta.1 frames are Frame::Video(VideoFrame::BGRA(...)).
    // width/height are i32 in this version, so we cast to u32 for ffmpeg.
    let (width, height, first_data) = match first_frame {
        Frame::Video(VideoFrame::BGRA(f)) => (f.width as u32, f.height as u32, f.data),
        _ => {
            let _ = app.emit("recording-error", "Unexpected frame format from capture device.");
            capturer.stop_capture();
            return;
        }
    };

    let output_str = output_path.to_string_lossy().to_string();
    let size_arg = format!("{width}x{height}");

    // Build the crop filter string (shared between MP4 and GIF paths).
    let crop_filter: Option<String> = region.as_ref().and_then(|r| {
        let cx = (r.x.max(0) as u32).min(width.saturating_sub(1));
        let cy = (r.y.max(0) as u32).min(height.saturating_sub(1));
        let cw = (r.width.min(width - cx) / 2) * 2;  // must be even for H.264/GIF
        let ch = (r.height.min(height - cy) / 2) * 2;
        if cw == 0 || ch == 0 { return None; }
        if cx > 0 || cy > 0 || cw < width || ch < height {
            Some(format!("crop={}:{}:{}:{}", cw, ch, cx, cy))
        } else {
            None
        }
    });

    // CRF values: lower = better quality, larger file.
    // MP4 (libx264): 18 high, 28 medium, 40 low
    // WebM (libvpx-vp9): 20 high, 33 medium, 48 low
    let (crf_mp4, crf_webm) = match quality {
        Quality::High   => ("18", "20"),
        Quality::Medium => ("28", "33"),
        Quality::Low    => ("40", "48"),
    };

    // GIF: cap fps at 20 regardless of capture fps, scale by quality tier.
    let gif_fps   = fps.min(20);
    let gif_scale = match quality {
        Quality::High   => "min(1280,iw)",
        Quality::Medium => "min(1024,iw)",
        Quality::Low    => "min(640,iw)",
    };

    let fps_str = fps.to_string();

    let mut args: Vec<String> = vec![
        "-f".into(), "rawvideo".into(),
        "-pixel_format".into(), "bgra".into(),
        "-video_size".into(), size_arg,
        "-framerate".into(), fps_str,
        "-i".into(), "pipe:0".into(),
    ];

    match format {
        OutputFormat::Mp4 => {
            if let Some(ref crop) = crop_filter {
                args.push("-vf".into());
                args.push(crop.clone());
            }
            args.extend([
                "-c:v".into(), "libx264".into(),
                "-crf".into(), crf_mp4.into(),
                "-pix_fmt".into(), "yuv420p".into(),
                "-movflags".into(), "+faststart".into(),
                "-y".into(),
                output_str.clone(),
            ]);
        }
        OutputFormat::WebM => {
            if let Some(ref crop) = crop_filter {
                args.push("-vf".into());
                args.push(crop.clone());
            }
            args.extend([
                "-c:v".into(), "libvpx-vp9".into(),
                "-crf".into(), crf_webm.into(),
                "-b:v".into(), "0".into(),
                "-pix_fmt".into(), "yuv420p".into(),
                "-y".into(),
                output_str.clone(),
            ]);
        }
        OutputFormat::Gif => {
            let gif_vf = format!(
                "fps={gif_fps},scale='{gif_scale}':-2:flags=lanczos"
            );
            let vf = match crop_filter {
                Some(ref crop) => format!(
                    "{crop},{gif_vf},split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"
                ),
                None => format!(
                    "{gif_vf},split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"
                ),
            };
            args.push("-vf".into());
            args.push(vf);
            args.extend([
                "-loop".into(), "0".into(),
                "-y".into(),
                output_str.clone(),
            ]);
        }
    }

    let mut ffmpeg = match Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit()) // show ffmpeg errors in terminal — remove once confirmed working
        .spawn()
    {
        Ok(p) => p,
        Err(_) => {
            let _ = app.emit(
                "recording-error",
                "ffmpeg not found. Install it with: brew install ffmpeg",
            );
            capturer.stop_capture();
            return;
        }
    };

    let stdin = ffmpeg.stdin.as_mut().unwrap(); // TODO: handle error
    let _ = stdin.write_all(&first_data);

    let mut frame_count: u64 = 1;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        match capturer.get_next_frame() {
            Ok(Frame::Video(VideoFrame::BGRA(f))) => {
                let _ = stdin.write_all(&f.data);
                frame_count += 1;
                if frame_count % 30 == 0 {
                    let _ = app.emit("recording-tick", frame_count);
                }
            }
            Ok(_) => {} // ignore audio frames and other video formats
            Err(e) => {
                eprintln!("[pkap] capture error: {e}");
                break;
            }
        }
    }

    capturer.stop_capture();
    drop(ffmpeg.stdin.take());
    let _ = ffmpeg.wait();

    let _ = app.emit("recording-saved", output_str);
    let _ = app.emit("recording-status", false);

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}
