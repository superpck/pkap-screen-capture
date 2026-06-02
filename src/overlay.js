// Handles the region-selection drag on the transparent overlay window.
// Sends physical pixel coordinates to Rust when the user releases the mouse.

const { invoke } = window.__TAURI__.core;

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

// devicePixelRatio is 2.0 on Retina/HiDPI screens.
// We size the canvas in physical pixels so drawing is crisp, then scale ctx
// so that all our drawing coordinates stay in CSS (logical) pixels.
const dpr = window.devicePixelRatio || 1;
canvas.width = window.innerWidth * dpr;
canvas.height = window.innerHeight * dpr;
canvas.style.width = window.innerWidth + "px";
canvas.style.height = window.innerHeight + "px";
ctx.scale(dpr, dpr);

const W = window.innerWidth;
const H = window.innerHeight;

let dragging = false;
let startX = 0, startY = 0;
let curX = 0, curY = 0;

// Redraws the overlay: dims everything outside the selection, leaves selection clear.
function draw() {
  ctx.clearRect(0, 0, W, H);

  if (!dragging && startX === 0 && curX === 0) {
    // No drag started yet — dim the whole screen.
    ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
    ctx.fillRect(0, 0, W, H);
    return;
  }

  // Normalise: ensure x/y is always top-left regardless of drag direction.
  const x = Math.min(startX, curX);
  const y = Math.min(startY, curY);
  const w = Math.abs(curX - startX);
  const h = Math.abs(curY - startY);

  // Draw dim overlay in 4 rects that surround the selection.
  // This leaves the selection area transparent so the screen shows through.
  ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
  ctx.fillRect(0, 0, W, y);           // top strip
  ctx.fillRect(0, y + h, W, H - y - h); // bottom strip
  ctx.fillRect(0, y, x, h);            // left strip
  ctx.fillRect(x + w, y, W - x - w, h); // right strip

  // Draw a white border around the selection.
  ctx.strokeStyle = "rgba(255, 255, 255, 0.9)";
  ctx.lineWidth = 1.5;
  ctx.strokeRect(x, y, w, h);

  // Show pixel dimensions inside the selection once it's big enough to fit the label.
  if (w > 80 && h > 28) {
    const label = `${Math.round(w * dpr)} × ${Math.round(h * dpr)}`;
    ctx.font = "bold 12px -apple-system, sans-serif";
    const tw = ctx.measureText(label).width;
    ctx.fillStyle = "rgba(0, 0, 0, 0.65)";
    ctx.fillRect(x + 6, y + 6, tw + 12, 20);
    ctx.fillStyle = "#fff";
    ctx.fillText(label, x + 12, y + 20);
  }
}

canvas.addEventListener("mousedown", (e) => {
  dragging = true;
  startX = e.clientX;
  startY = e.clientY;
  curX = e.clientX;
  curY = e.clientY;
  draw();
});

canvas.addEventListener("mousemove", (e) => {
  if (!dragging) return;
  curX = e.clientX;
  curY = e.clientY;
  draw();
});

canvas.addEventListener("mouseup", async (e) => {
  if (!dragging) return;
  dragging = false;

  const x = Math.min(startX, e.clientX);
  const y = Math.min(startY, e.clientY);
  const w = Math.abs(e.clientX - startX);
  const h = Math.abs(e.clientY - startY);

  // Ignore accidental tiny clicks.
  if (w < 10 || h < 10) {
    startX = 0; curX = 0;
    draw();
    return;
  }

  // Multiply by dpr to convert CSS pixels → physical pixels.
  // scap outputs frames in physical pixels, so crop coordinates must match.
  await invoke("set_region", {
    x: Math.round(x * dpr),
    y: Math.round(y * dpr),
    width: Math.round(w * dpr),
    height: Math.round(h * dpr),
  });
});

window.addEventListener("keydown", async (e) => {
  if (e.key === "Escape") {
    await invoke("cancel_region_select");
  }
});

// Draw the initial dimmed state as soon as the overlay opens.
draw();
