// Preview window logic — loads the just-recorded file and offers Keep / Discard.
const { invoke, convertFileSrc } = window.__TAURI__.core;

let previewPath = null;

window.addEventListener("DOMContentLoaded", async () => {
  const wrap       = document.getElementById("media-wrap");
  const btnKeep    = document.getElementById("btn-keep");
  const btnDiscard = document.getElementById("btn-discard");

  // Ask Rust for the file we should preview.
  const info = await invoke("get_preview_info");

  if (!info) {
    wrap.innerHTML = `<p class="loading">No preview available.</p>`;
    return;
  }

  previewPath = info.path;

  // convertFileSrc converts a local filesystem path to an asset:// URL
  // that the Tauri webview is allowed to load (requires assetProtocol.enable in tauri.conf.json).
  const src = convertFileSrc(info.path);

  wrap.innerHTML = "";

  if (info.format === "gif") {
    const img = document.createElement("img");
    img.src = src;
    img.alt = "Recording preview";
    wrap.appendChild(img);
  } else {
    // MP4 and WebM are both playable with <video>.
    const video = document.createElement("video");
    video.src = src;
    video.controls = true;
    video.autoplay = true;
    video.loop = true;
    wrap.appendChild(video);
  }

  btnKeep.addEventListener("click", async () => {
    // File is already saved — just close the preview.
    await invoke("close_preview");
  });

  btnDiscard.addEventListener("click", async () => {
    await invoke("discard_recording", { path: previewPath });
  });
});
