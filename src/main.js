// Tauri v2 exposes invoke at window.__TAURI__.core.invoke
const { invoke } = window.__TAURI__.core;

// Input pricing in USD per 1M tokens. Update from https://www.anthropic.com/pricing
// when rates change — these are approximations used only for the local estimate.
const PRICING = {
  "claude-opus-4-7":             15.0,
  "claude-opus-4-6":             15.0,
  "claude-sonnet-4-6":            3.0,
  "claude-haiku-4-5-20251001":    1.0,
};

const $ = (id) => document.getElementById(id);
const els = {
  input:    $("input"),
  tokens:   $("tokens"),
  cost:     $("cost"),
  model:    $("model"),
  status:   $("status"),
  countBtn: $("count-btn"),
  settings: $("settings-btn"),
  dialog:   $("settings-dialog"),
  apiKey:   $("api-key"),
  saveKey:  $("save-key"),
  clearKey: $("clear-key"),
  closeDlg: $("close-dialog"),
};

let lastTokens = null;
let lastText   = "";
let lastModel  = "";

function setStatus(msg, isError = false) {
  els.status.textContent = msg || "";
  els.status.classList.toggle("error", !!isError);
}

function renderCost(tokens, model) {
  const rate = PRICING[model];
  if (rate == null || tokens === 0) {
    els.cost.textContent = "no estimate";
    return;
  }
  const usd = (tokens / 1_000_000) * rate;
  els.cost.textContent = usd < 0.0001
    ? `≈ $${usd.toExponential(2)}`
    : `≈ $${usd.toFixed(usd < 0.01 ? 6 : 4)}`;
}

async function countNow() {
  const text  = els.input.value;
  const model = els.model.value;

  if (!text.trim()) {
    lastTokens = 0;
    lastText   = text;
    lastModel  = model;
    els.tokens.classList.remove("counting", "error");
    els.tokens.textContent = "0";
    renderCost(0, model);
    setStatus("");
    return;
  }

  els.countBtn.disabled = true;
  els.tokens.classList.add("counting");
  els.tokens.classList.remove("error");
  setStatus("Counting…");

  try {
    const tokens = await invoke("count_tokens", { text, model });
    lastTokens = tokens;
    lastText   = text;
    lastModel  = model;
    els.tokens.classList.remove("counting", "error");
    els.tokens.textContent = tokens.toLocaleString();
    renderCost(tokens, model);
    setStatus("");
  } catch (err) {
    els.tokens.classList.remove("counting");
    els.tokens.classList.add("error");
    els.tokens.textContent = "error";
    els.cost.textContent = "—";
    setStatus(String(err), true);
  } finally {
    els.countBtn.disabled = false;
  }
}

// Recompute cost instantly when the model changes if text is unchanged.
els.model.addEventListener("change", () => {
  if (lastTokens != null && els.input.value === lastText) {
    renderCost(lastTokens, els.model.value);
    lastModel = els.model.value;
  }
});

// Mark count as stale when the user edits the textarea.
els.input.addEventListener("input", () => {
  if (els.input.value !== lastText && lastTokens !== null) {
    els.tokens.classList.add("counting");
  }
});

els.countBtn.addEventListener("click", countNow);

// Cmd/Ctrl+Enter to count.
els.input.addEventListener("keydown", (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
    e.preventDefault();
    countNow();
  }
});

// Settings dialog.
els.settings.addEventListener("click", () => els.dialog.showModal());
els.closeDlg.addEventListener("click", () => els.dialog.close());

els.saveKey.addEventListener("click", async () => {
  const key = els.apiKey.value.trim();
  if (!key) { setStatus("Key is empty.", true); return; }
  try {
    await invoke("save_api_key", { key });
    els.apiKey.value = "";
    els.dialog.close();
    setStatus("API key saved.");
    setTimeout(() => setStatus(""), 2200);
  } catch (err) {
    setStatus(`Save failed: ${err}`, true);
  }
});

els.clearKey.addEventListener("click", async () => {
  try {
    await invoke("clear_api_key");
    setStatus("API key cleared.");
    setTimeout(() => setStatus(""), 2200);
  } catch (err) {
    setStatus(String(err), true);
  }
});

// On startup, prompt for key if missing.
(async () => {
  try {
    const has = await invoke("has_api_key");
    if (!has) els.dialog.showModal();
  } catch { /* ignore */ }
})();
