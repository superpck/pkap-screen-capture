const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// Shorten a full path to "…/parent/folder" for display.
function shortenPath(fullPath) {
  const sep = fullPath.includes("/") ? "/" : "\\";
  const parts = fullPath.split(sep).filter(Boolean);
  if (parts.length <= 2) return fullPath;
  return `…${sep}${parts.slice(-2).join(sep)}`;
}

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

async function startWithCountdown() {
  const settings = await invoke("get_settings");
  const btn = document.getElementById("record-btn");
  const errLabel = document.getElementById("error-label");

  if (settings.countdown) {
    btn.disabled = true;
    for (let i = 3; i > 0; i--) {
      btn.textContent = String(i);
      await sleep(1000);
    }
    btn.disabled = false;
  }

  try {
    await invoke("start_recording");
  } catch (err) {
    btn.textContent = "Start Recording";
    btn.disabled = false;
    errLabel.textContent = err;
    setTimeout(() => { errLabel.textContent = ""; }, 6000);
  }
}

async function refreshProfiles() {
  const profiles = await invoke("get_profiles");
  const sel = document.getElementById("profile-select");
  const current = sel.value;
  sel.innerHTML = `<option value="">Profiles…</option>` +
    profiles.map(p => `<option value="${p.name}">${p.name}</option>`).join("");
  if (profiles.find(p => p.name === current)) sel.value = current;
}

function setQualityUI(q) {
  document.querySelectorAll(".qual-btn").forEach(btn => {
    btn.classList.toggle("active", btn.dataset.q === q);
  });
}

function setFormatUI(fmt) {
  document.querySelectorAll(".fmt-btn").forEach(btn => {
    btn.classList.toggle("active", btn.dataset.fmt === fmt);
  });
}

function showRegion(region) {
  const label = document.getElementById("region-label");
  if (!region) {
    label.textContent = "No region selected";
    label.classList.remove("set");
  } else {
    label.textContent = `${region.width} × ${region.height}  at (${region.x}, ${region.y})`;
    label.classList.add("set");
  }
}

// Syncs all recording UI elements to the given boolean state.
// Called from both the button handler and the Rust event listener so the
// button and the global hotkey always show the same state.
function setRecordingUI(isRecording) {
  const btn      = document.getElementById("record-btn");
  const dot      = document.getElementById("status-dot");
  const label    = document.getElementById("status-label");
  const ticks    = document.getElementById("frame-count");
  const savePath = document.getElementById("save-path");

  if (isRecording) {
    btn.textContent = "Stop Recording";
    btn.classList.add("recording");
    dot.className = "status-dot recording";
    label.textContent = "Recording…";
    ticks.textContent = "";
    savePath.textContent = "";
  } else {
    btn.textContent = "Start Recording";
    btn.classList.remove("recording");
    dot.className = "status-dot idle";
    label.textContent = "Idle";
    ticks.textContent = "";
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  const recordBtn = document.getElementById("record-btn");
  const monitorSelect    = document.getElementById("monitor-select");
  const btnSelect        = document.getElementById("btn-select-region");
  const saveFolderLabel  = document.getElementById("save-folder-label");
  const btnChangeFolder  = document.getElementById("btn-change-folder");
  const ticks     = document.getElementById("frame-count");
  const savePath  = document.getElementById("save-path");
  const errLabel  = document.getElementById("error-label");

  // Restore UI to match whatever state Rust already has (e.g. after a page reload).
  showRegion(await invoke("get_region"));
  setRecordingUI(await invoke("get_recording_status"));

  const savedFmt = await invoke("get_output_format");
  setFormatUI(savedFmt);

  // Restore all settings on startup.
  const settings = await invoke("get_settings");
  document.getElementById("fps-select").value = String(settings.fps);
  setQualityUI(settings.quality);
  document.getElementById("countdown-toggle").checked = settings.countdown;

  // Load profiles into dropdown.
  await refreshProfiles();

  // Show the current save folder on startup.
  const defaultFolder = await invoke("get_save_folder");
  saveFolderLabel.textContent = shortenPath(defaultFolder);

  // Rust emits this when recording starts or stops (from button OR hotkey).
  await listen("recording-status", (e) => setRecordingUI(e.payload));

  // Rust emits this every ~30 frames so we can show a live counter.
  await listen("recording-tick", (e) => {
    ticks.textContent = `${e.payload} frames captured`;
  });

  // Rust emits the output path as soon as recording starts (before encoding finishes).
  await listen("recording-output-path", (e) => {
    savePath.textContent = `Saving to: ${e.payload}`;
  });

  // Rust emits this after ffmpeg finishes writing — open preview window.
  await listen("recording-saved", async (e) => {
    ticks.textContent = "";
    savePath.textContent = `Saved: ${e.payload}`;
    const fmt = await invoke("get_output_format");
    await invoke("open_preview_window", { path: e.payload, format: fmt });
  });

  // User clicked Discard in preview window.
  await listen("recording-discarded", () => {
    savePath.textContent = "Recording discarded.";
    setTimeout(() => { savePath.textContent = ""; }, 3000);
  });

  // Rust emits this if something went wrong (e.g. permission denied, ffmpeg missing).
  await listen("recording-error", (e) => {
    errLabel.textContent = e.payload;
    savePath.textContent = "";
    setTimeout(() => { errLabel.textContent = ""; }, 8000);
  });

  await listen("region-selected", (e) => showRegion(e.payload));

  // Populate monitor dropdown. Each option's value is the monitor index.
  try {
    const monitors = await invoke("get_monitors");
    monitorSelect.innerHTML = monitors.map((m, i) =>
      `<option value="${i}">${m.name} (${m.width}×${m.height})</option>`
    ).join("");

    // Selecting a monitor immediately sets its full area as the capture region.
    monitorSelect.addEventListener("change", () => {
      invoke("select_monitor", { index: parseInt(monitorSelect.value) });
    });

    // Auto-select the first monitor on startup so there's always a region set.
    if (monitors.length > 0) {
      await invoke("select_monitor", { index: 0 });
    }
  } catch (err) {
    monitorSelect.innerHTML = `<option value="0">Display 1</option>`;
    await invoke("select_full_screen");
  }

  btnSelect.addEventListener("click", () => invoke("start_region_select"));

  document.getElementById("fps-select").addEventListener("change", (e) => {
    invoke("set_fps", { fps: parseInt(e.target.value) });
  });

  document.querySelectorAll(".qual-btn").forEach(btn => {
    btn.addEventListener("click", async () => {
      const q = btn.dataset.q;
      await invoke("set_quality", { quality: q });
      setQualityUI(q);
    });
  });

  document.querySelectorAll(".fmt-btn").forEach(btn => {
    btn.addEventListener("click", async () => {
      const fmt = btn.dataset.fmt;
      await invoke("set_output_format", { format: fmt });
      setFormatUI(fmt);
    });
  });

  btnChangeFolder.addEventListener("click", async () => {
    // Open the native folder picker from the JS side — avoids main-thread
    // deadlocks that occur when calling blocking_pick_folder from Rust.
    const chosen = await window.__TAURI__.dialog.open({
      directory: true,
      multiple: false,
      title: "Choose Recording Save Location",
    });
    if (chosen) {
      await invoke("set_save_folder", { path: chosen });
      saveFolderLabel.textContent = shortenPath(chosen);
    }
  });

  recordBtn.addEventListener("click", async () => {
    const isRecording = await invoke("get_recording_status");
    if (isRecording) {
      await invoke("stop_recording");
    } else {
      await startWithCountdown();
    }
  });

  document.getElementById("countdown-toggle").addEventListener("change", (e) => {
    invoke("set_countdown", { enabled: e.target.checked });
  });

  document.getElementById("btn-save-profile").addEventListener("click", async () => {
    const name = window.prompt("Profile name:");
    if (!name?.trim()) return;
    await invoke("save_profile", { name: name.trim() });
    await refreshProfiles();
    document.getElementById("profile-select").value = name.trim();
  });

  document.getElementById("btn-apply-profile").addEventListener("click", async () => {
    const name = document.getElementById("profile-select").value;
    if (!name) return;
    const s = await invoke("apply_profile", { name });
    document.getElementById("fps-select").value = String(s.fps);
    setQualityUI(s.quality);
    const fmt = await invoke("get_output_format");
    setFormatUI(fmt);
  });

  document.getElementById("btn-delete-profile").addEventListener("click", async () => {
    const name = document.getElementById("profile-select").value;
    if (!name) return;
    if (!window.confirm(`Delete profile "${name}"?`)) return;
    await invoke("delete_profile", { name });
    await refreshProfiles();
  });
});
