const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

const REFRESH_MS = 2 * 60 * 1000;

// ── Pin (always on top) ────────────────────────────────────────

let pinned = localStorage.getItem("pinned") === "true";

async function applyPin(value) {
  pinned = value;
  await getCurrentWindow().setAlwaysOnTop(pinned);
  localStorage.setItem("pinned", pinned);
  const btn = document.getElementById("pin-btn");
  btn.classList.toggle("active", pinned);
  btn.setAttribute("aria-pressed", String(pinned));
}

applyPin(pinned);

// ── Countdown ──────────────────────────────────────────────────

function timeUntil(isoString) {
  if (!isoString) return null;
  const diff = Math.max(0, new Date(isoString) - Date.now());
  const h = Math.floor(diff / 3_600_000);
  const m = Math.floor((diff % 3_600_000) / 60_000);
  const s = Math.floor((diff % 60_000) / 1_000);
  if (h > 0)  return `resets in ${h}h ${m}m`;
  if (m > 0)  return `resets in ${m}m ${s}s`;
  if (s > 0)  return `resets in ${s}s`;
  return "resetting…";
}

function daysUntil(isoString) {
  if (!isoString) return null;
  const diff = Math.max(0, new Date(isoString) - Date.now());
  const days  = Math.floor(diff / 86_400_000);
  const hours = Math.floor((diff % 86_400_000) / 3_600_000);
  if (days > 1)  return `resets in ${days}d`;
  if (days === 1) return `resets in 1d ${hours}h`;
  return timeUntil(isoString);
}

// ── State ──────────────────────────────────────────────────────

let fhResetsAt = null;
let sdResetsAt = null;

function renderCountdowns() {
  document.getElementById("fh-resets").textContent = timeUntil(fhResetsAt) ?? "";
  document.getElementById("sd-resets").textContent = daysUntil(sdResetsAt) ?? "";
}

// ── Bar ────────────────────────────────────────────────────────

function setBar(id, pct) {
  const el = document.getElementById(id);
  el.style.width = Math.min(pct, 100) + "%";
  el.className = "progress-fill" +
    (pct >= 90 ? " danger" : pct >= 70 ? " warning" : "");
}

// ── Status ─────────────────────────────────────────────────────

function setStatus(msg, isError = false) {
  const el = document.getElementById("status");
  el.textContent = msg;
  el.className = isError ? "error" : "";
}

// ── Fetch ──────────────────────────────────────────────────────

async function refresh() {
  try {
    const data = await invoke("get_usage");

    document.getElementById("fh-pct").textContent = Math.round(data.five_hour_pct);
    document.getElementById("sd-pct").textContent = Math.round(data.seven_day_pct) + "%";
    setBar("fh-bar", data.five_hour_pct);
    setBar("sd-bar", data.seven_day_pct);

    fhResetsAt = data.five_hour_resets_at;
    sdResetsAt = data.seven_day_resets_at;
    renderCountdowns();

    document.getElementById("last-updated").textContent =
      new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

    setStatus("");
  } catch (err) {
    console.error("get_usage:", err);
    setStatus(String(err), true);
  }
}

// ── Init ───────────────────────────────────────────────────────

document.getElementById("pin-btn").addEventListener("click", () => applyPin(!pinned));
document.getElementById("refresh-btn").addEventListener("click", refresh);
setInterval(renderCountdowns, 1000);
setInterval(refresh, REFRESH_MS);
refresh();
