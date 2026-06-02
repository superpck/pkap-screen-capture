# pkap — Screen Recorder

> **Developer documentation.** For users: just run `./build.sh` and open the `.dmg`.

Cross-platform screen recorder built with **Rust + Tauri 2**.  
Records to MP4, WebM, and GIF. Runs natively on macOS Apple Silicon, Intel, Windows, and Linux.

**Repository:** https://github.com/superpck/pkap-screen-capture

---

## Tech Stack

| Layer | Technology |
|---|---|
| Framework | Tauri 2 |
| Backend | Rust |
| Frontend | Vanilla HTML + CSS + JavaScript (no bundler) |
| Screen capture | `scap 0.1.0-beta.1` |
| Video encoding | `ffmpeg` binary (subprocess) |
| Global hotkey | `tauri-plugin-global-shortcut` |
| Folder picker | `tauri-plugin-dialog` |
| Async | Tauri's built-in tokio runtime |
| Errors | `anyhow` |

---

## Prerequisites

```sh
# 1. Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Tauri CLI
cargo install tauri-cli

# 3. ffmpeg  (runtime dependency — users need this too unless you bundle it)
brew install ffmpeg

# 4. Clone
git clone https://github.com/superpck/pkap-screen-capture
cd pkap-screen-capture
```

---

## Run in Development

```sh
cargo tauri dev
```

The file watcher recompiles Rust on save and hot-reloads the frontend automatically.

---

## Build for Release

```sh
./build.sh                # macOS Universal (arm64 + x86_64)  ← default
./build.sh --arm          # Apple Silicon only
./build.sh --intel        # Intel only
./build.sh --linux        # Linux x86_64  (requires Docker)
./build.sh --windows      # Windows  (see notes inside script)
./build.sh --all          # all platforms
```

Output: `src-tauri/target/<triple>/release/bundle/`

---

## File Map

```
pkap/
├── build.sh                        ← release build script (all platforms)
├── make_icon.py                    ← regenerate app icon from Python/Pillow
│
├── src/                            ← frontend (static files, no bundler)
│   ├── index.html                  ← main window UI
│   ├── main.js                     ← main window logic (wires all buttons)
│   ├── styles.css                  ← all styles
│   ├── overlay.html / overlay.js   ← transparent region-select screen
│   └── preview.html / preview.js   ← post-recording preview window
│
└── src-tauri/
    ├── Cargo.toml                  ← Rust dependencies
    ├── tauri.conf.json             ← window config, asset protocol, icons
    ├── capabilities/default.json  ← JS permission allowlist (windows, plugins)
    └── src/
        ├── main.rs                 ← 6 lines: calls lib::run(), never edit
        ├── lib.rs                  ← Tauri builder, tray, global shortcut, startup
        ├── state.rs                ← AppState (all shared runtime data)
        ├── commands.rs             ← every #[tauri::command] the frontend calls
        ├── capture.rs              ← screen capture loop (runs on its own thread)
        └── settings_store.rs      ← load/save settings.json to disk
```

---

## Recommended Reading Order

Read the source files in this order to understand the full data flow in ~30 minutes.

### 1. `state.rs` — what data exists at runtime

All shared state lives in one struct, `AppState`, managed by Tauri.
Understanding this first makes every other file easier.

```
AppState
 ├── region              → CaptureRegion  (x, y, width, height in physical px)
 ├── recording           → AtomicBool     (true while capture thread is running)
 ├── stop_flag           → Arc<AtomicBool> (set to true to stop the capture thread)
 ├── selected_display_index → which monitor scap should capture
 ├── save_folder         → where to write the output file
 ├── output_format       → Mp4 | WebM | Gif
 ├── fps / quality       → capture settings
 ├── countdown_enabled   → show 3-2-1 before recording?
 └── pending_preview     → path + format waiting for the preview window to open
```

Key types: `CaptureRegion`, `OutputFormat`, `Quality`, `PreviewInfo`.

---

### 2. `lib.rs` — how the app starts

This is the real entry point (`main.rs` just calls `lib::run()`).

Read it to understand:
- How Tauri is configured (plugins, managed state)
- How the **tray icon** and its menu are built
- How the **global hotkey** (`Cmd+Shift+R`) is registered
- How **saved settings** are loaded from disk into `AppState` on startup
- The full list of registered `#[tauri::command]` functions

---

### 3. `commands.rs` — everything the frontend can call

Every function tagged `#[tauri::command]` is callable from JS via
`window.__TAURI__.core.invoke("function_name", { args })`.

Functions are grouped into sections:

| Section | Key functions |
|---|---|
| Monitor / region | `get_monitors`, `select_monitor`, `start_region_select`, `set_region` |
| Settings | `get_settings`, `set_fps`, `set_quality`, `set_countdown`, `persist()` |
| Profiles | `get_profiles`, `save_profile`, `apply_profile`, `delete_profile` |
| Preview | `open_preview_window`, `get_preview_info`, `discard_recording`, `close_preview` |
| Format / folder | `set_output_format`, `set_save_folder`, `get_save_folder` |
| Recording | `start_recording`, `stop_recording`, `get_recording_status` |

**The two most important non-command functions:**

`do_start(app, state)` — called by both the button command and the global hotkey.  
Reads state, generates the output file path, creates the `Arc<AtomicBool>` stop flag,
hides the main window, and spawns the capture thread.

`do_stop(app, state)` — sets the stop flag to `true`, updates `recording` state,
shows the main window, emits `recording-status: false`.

---

### 4. `capture.rs` — the capture thread

`capture_loop(app, stop, region, output_path, display_index, format, fps, quality)`
runs on a dedicated `std::thread` (not tokio — `scap` blocks, which would stall async).

**Step-by-step inside `capture_loop`:**

```
1. Check macOS Screen Recording permission (scap::has_permission)
2. Build scap Options:
     - target = the selected monitor (Target::Display at display_index)
     - fps    = user setting
     - output_type = BGRAFrame
3. capturer.start_capture()
4. Get first frame → read width × height for ffmpeg
5. Build ffmpeg command args:
     MP4  → libx264, CRF from quality setting
     WebM → libvpx-vp9, CRF from quality setting
     GIF  → fps filter + palette generation (two-pass in one command)
     + crop filter if region is a sub-area of the display
6. Spawn ffmpeg subprocess (stdin = piped)
7. Loop: get_next_frame() → write frame.data to ffmpeg stdin
          every 30 frames → emit "recording-tick" event to frontend
          check stop flag each iteration
8. Stop: set stop flag → drop stdin → ffmpeg.wait() (finalises file)
9. Emit "recording-saved" with file path
10. Show main window again
```

**Why `std::thread` instead of `tokio::spawn`?**  
`scap::get_next_frame()` is a *blocking* call. Blocking inside a tokio task starves
the async runtime. A dedicated OS thread is allowed to block freely.

**Why ffmpeg subprocess instead of `ffmpeg-next` crate?**  
`ffmpeg-next` requires FFmpeg C headers at *build* time (complex setup).
The subprocess approach only requires the `ffmpeg` binary at *runtime* and
reduces the Rust code from ~300 lines to ~50.

---

### 5. `settings_store.rs` — persistence

Simple JSON file at `~/Library/Application Support/pkap/settings.json` (macOS).

```rust
pub fn load(config_dir: &PathBuf) -> SettingsFile  // deserialise from disk
pub fn save(config_dir: &PathBuf, settings: &SettingsFile)  // serialise to disk
```

Every `set_*` command calls the internal `persist()` helper which reads the
current `AppState` and calls `settings_store::save()`.

---

### 6. Frontend files

**`main.js`** — the main window.  
Reads state on startup (`get_settings`, `get_region`, `get_output_format`, `get_profiles`),
wires every button to an `invoke()` call, and listens for these Rust events:

| Event | When emitted | What JS does |
|---|---|---|
| `recording-status` | start / stop | updates button + status dot |
| `recording-tick` | every ~1 s during capture | shows frame count |
| `recording-output-path` | just after start | shows "Saving to…" |
| `recording-saved` | ffmpeg finishes | opens preview window |
| `recording-discarded` | user discards | shows "Recording discarded." |
| `region-selected` | monitor/region chosen | updates region label |

**`overlay.js`** — drag-to-select region.  
Canvas covers the selected monitor. `mousedown/move/up` tracks the drag rectangle.
On release: multiplies CSS coords by `devicePixelRatio` to get physical pixels,
calls `invoke("set_region", {x, y, width, height})`.
`Escape` calls `invoke("cancel_region_select")`.

**`preview.js`** — keep or discard.  
Calls `get_preview_info()` on load, converts the file path to an `asset://` URL
via `convertFileSrc()` (requires `assetProtocol.enable: true` in `tauri.conf.json`),
loads into `<video>` or `<img>` depending on format.
Keep → `close_preview()`. Discard → `discard_recording(path)` then close.

---

## Key Data Flow: one recording

```
User clicks "Start"
  │
  ▼
main.js: startWithCountdown()
  │  (optional 3-2-1 in the button)
  ▼
invoke("start_recording")
  │
  ▼
commands::do_start()
  ├── reads: region, display_index, format, fps, quality, save_folder
  ├── generates: output_path  (e.g. ~/Movies/pkap-1717000000.mp4)
  ├── creates: Arc<AtomicBool> stop_flag
  ├── hides: main window
  └── spawns: std::thread → capture_loop()
                │
                ├── scap captures BGRA frames from selected display
                └── pipes frames → ffmpeg subprocess → output file

User clicks "Stop"  (or presses Cmd+Shift+R)
  │
  ▼
commands::do_stop()
  ├── sets: stop_flag = true  (capture thread exits loop)
  ├── emits: "recording-status" false
  └── shows: main window

capture thread finishes
  ├── ffmpeg.wait() → file is complete
  ├── emits: "recording-saved" with path
  └── shows: main window (safety net)

main.js receives "recording-saved"
  └── invoke("open_preview_window", {path, format})
        │
        └── new Tauri window → preview.html
              ├── loads file via asset:// URL
              ├── Keep    → close_preview()
              └── Discard → discard_recording(path) → delete file
```

---

## Common Gotchas

| Problem | Cause | Fix |
|---|---|---|
| 0-byte output file | H.264 requires even pixel dimensions | crop w/h are rounded down: `(n/2)*2` in `capture.rs` |
| Overlay on wrong monitor | Overlay window position must match selected monitor | `start_region_select` reads `selected_display_index` and positions at `monitor.position()` |
| `transparent()` compile error on macOS | Requires `macos-private-api` feature | Add to Cargo.toml features + `macOSPrivateApi: true` in tauri.conf.json |
| folder picker deadlock | `blocking_pick_folder()` blocks macOS main thread | Use `window.__TAURI__.dialog.open()` from JS instead |
| scap Frame enum mismatch | scap 0.1.0-beta.1 changed variant to `Frame::Video(VideoFrame::BGRA(f))` | Matched in `capture.rs` |
| Settings not persisting | `set_*` commands must call `persist()` | Every setter calls the private `persist(app, state)` helper |
