# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# pkap — Screen Recorder (Kap Clone)

A cross-platform screen recorder built with Rust + Tauri 2.
Goal: export to GIF, MP4, and WebM. Runs natively on macOS Apple Silicon, Windows, and Linux.

---

## My Rust Level

I am a **complete beginner** in Rust. Assume I know nothing about:
- Ownership and borrowing
- Lifetimes
- Traits and generics
- Async/await with Tokio
- Cargo and crate ecosystem

**Always explain WHY, not just WHAT.**
If there are two ways to do something, show the simpler one first and note the tradeoff.

---

## Commands

No Node.js / npm in this project — the frontend is plain static files served directly by Tauri. All commands are Rust/Cargo.

```sh
# Run the app in development mode (hot-reloads frontend changes)
cargo tauri dev

# Production build
cargo tauri build

# Run all tests
cargo test

# Run a single test by name
cargo test <test_name>

# Lint
cargo clippy

# Format
cargo fmt
```

---

## Architecture

### How Tauri 2 structures the app

`src-tauri/src/main.rs` is a one-liner that calls `kcap_app_lib::run()`. All real setup — registering commands, adding plugins, configuring the builder — lives in `src-tauri/src/lib.rs`. **When adding a new `#[tauri::command]`, register it in `lib.rs`, not `main.rs`.**

### Frontend: no bundler, no npm

`tauri.conf.json` sets `frontendDist: "../src"`, so Tauri serves `src/` as static files directly. There is no build step for the frontend. Because `withGlobalTauri: true` is set in `tauri.conf.json`, the Tauri API is available as a browser global — call it like this, not via npm imports:

```js
const { invoke } = window.__TAURI__.core;
await invoke("command_name", { arg: value });
```

### Planned source layout (not yet created)

The following modules are the target structure. Create them as features are built:

```
src-tauri/src/
├── main.rs      ← one-liner, do not touch
├── lib.rs       ← Tauri builder, plugin registration, command handler list
├── commands.rs  ← all #[tauri::command] functions
├── capture.rs   ← screen capture logic (scap crate)
├── encoder.rs   ← FFmpeg encoding, GIF/MP4/WebM export
└── state.rs     ← shared app state (recording status, config)
```

---

## Project Stack

| Layer     | Technology                          |
|-----------|-------------------------------------|
| Framework | Tauri 2                             |
| Language  | Rust (backend) + JavaScript (frontend) |
| UI        | HTML + CSS + vanilla JS             |
| Capture   | `scap` crate (not yet added)        |
| Encoding  | `ffmpeg-next` crate (not yet added) |
| Async     | `tokio` (not yet added)             |
| Errors    | `anyhow` (not yet added)            |

---

## Coding Rules

### Rust
- Use `anyhow::Result` for ALL error handling — no custom error types yet
- Use `tokio` for all async work
- Prefer `.clone()` over fighting lifetimes — note the tradeoff in a comment
- Use `.unwrap()` only during early prototyping, always add a `// TODO: handle error` comment
- Add a plain-English comment above every `fn` explaining what it does
- Add inline comments next to any ownership/borrowing code explaining why
- Always show the full function, not just the changed lines
- Always show `Cargo.toml` changes when adding a new crate

### JavaScript / Frontend
- Use `window.__TAURI__.core.invoke()` to call Rust commands (no npm imports)
- Keep UI simple — no heavy frameworks (no React, no Vue)
- Use plain `fetch`-style event handling with `addEventListener`
- Use `window.__TAURI__.event.listen()` for progress updates from Rust

### General
- Do NOT use `unsafe` without explaining exactly why it's necessary
- Do NOT use Rust nightly features
- Do NOT skip error handling silently
- Do NOT write OS-specific code unless explicitly asked
- Always write code that compiles on macOS, Windows, and Linux

---

## Tauri Command Pattern

```rust
// lib.rs — register the command here
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        commands::start_recording,
    ])
```

```rust
// commands.rs — define the command here

// Starts recording the screen.
// Called from the frontend when the user clicks "Record".
#[tauri::command]
pub async fn start_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // anyhow errors must be converted to String for Tauri
    your_function().await.map_err(|e| e.to_string())
}
```

---

## Key Crates to Add

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
scap = "0.1"           # screen capture, Apple Silicon compatible
ffmpeg-next = "7"      # video encoding
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

---

## Response Format

When implementing a feature, always structure the response like this:

### 1. What we're building
One or two sentences describing the feature.

### 2. Cargo.toml changes
Show the exact lines to add (if any new crates are needed).

### 3. Rust code
Full file or full function — never partial snippets I have to guess how to merge.
Comments on every function and any tricky lines.

### 4. Frontend code
HTML/JS changes if the feature needs UI.

### 5. How to test
Exact command or steps to verify it works.

---

## When I Paste a Compiler Error

1. Explain what the error means in plain language (pretend I've never seen this before)
2. Show the fixed code in full
3. Explain WHY the fix works — the concept behind it, not just the syntax

---

## Feature Roadmap (build in this order)

- [ ] 1. Basic Tauri window + menu bar icon
- [ ] 2. Select capture region (full screen or drag to select area)
- [ ] 3. Start / stop recording with hotkey
- [ ] 4. Encode and save as MP4
- [ ] 5. Export as GIF
- [ ] 6. Export as WebM
- [ ] 7. Preview before saving
- [ ] 8. Settings panel (fps, quality, save location)

When asked "what should we build next?", refer to this list.

---

## Things to AVOID

- `unsafe` without explanation
- Nightly Rust features (`#![feature(...)]`)
- Silent `unwrap()` without a TODO comment
- OS-specific APIs (use cross-platform crates instead)
- Partial code snippets — always show full functions
- Heavy frontend frameworks unless asked
