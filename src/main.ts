import * as api from "./api";
import "./styles.css";
import { listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";

const app = document.getElementById("app")!;
const currentWindow = getCurrentWindow();
const currentWindowLabel = currentWindow.label;
const viewParams = new URL(window.location.href).searchParams;
const isDebugWindow = currentWindowLabel === "debug" || viewParams.get("debug") === "1";
const isLoginWindow = currentWindowLabel === "login" || viewParams.get("login") === "1";
document.body.dataset.view = isDebugWindow ? "debug" : isLoginWindow ? "login" : "main";

document.addEventListener("contextmenu", (event) => {
  event.preventDefault();
}, true);

document.addEventListener("keydown", (event) => {
  const key = event.key.toUpperCase();
  const isDevtoolsShortcut =
    key === "F12" ||
    (event.ctrlKey && event.shiftKey && (key === "I" || key === "J" || key === "C"));
  if (isDevtoolsShortcut) {
    event.preventDefault();
    event.stopPropagation();
  }
}, true);

const debugMarkup = `
  <div id="debug-container">
    <div class="debug-header">
      <div>
        <div class="debug-title">LMPlus Debug</div>
        <div class="about-line">Live app state and recent diagnostic events</div>
      </div>
      <div class="modal-buttons">
        <button id="debug-refresh">Refresh</button>
        <button id="debug-clear" class="danger">Clear</button>
      </div>
    </div>
    <div class="debug-grid">
      <section class="debug-panel">
        <h2>State</h2>
        <div id="debug-state" class="debug-kv"></div>
      </section>
      <section class="debug-panel">
        <h2>Events</h2>
        <div id="debug-log"></div>
      </section>
    </div>
  </div>
`;

const mainMarkup = `
  <div id="app-container">
    <header class="app-header">
      <div class="identity-panel">
        <div class="identity-row">
          <span>User:</span>
          <button id="user-link" type="button" class="text-link">Not linked</button>
        </div>
        <div class="identity-row">
          <span>License:</span>
          <strong id="license-label">checking...</strong>
        </div>
        <div class="identity-row key-row">
          <span>Key:</span>
          <button id="license-key-value" type="button" class="key-value" title="Copy license key">-</button>
          <button id="license-key-copy" type="button" class="small-pill">Copy</button>
        </div>
      </div>
      <div class="toolbar">
        <button id="btn-app-path">App Path</button>
        <button id="btn-add-account">Add Account</button>
        <button id="btn-hotkeys">Hotkeys</button>
        <button id="btn-settings">Settings</button>
      </div>
    </header>
    <div id="account-list"></div>
    <div id="status-bar"></div>
  </div>
`;

const licenseMarkup = `
  <div id="startup-shell">
    <div class="startup-panel">
      <div class="identity-panel">
        <div class="identity-row">
          <span>User:</span>
          <button id="user-link" type="button" class="text-link">Coming soon</button>
        </div>
        <div class="identity-row">
          <span>License:</span>
          <strong id="license-label">checking...</strong>
        </div>
        <div class="identity-row key-row">
          <span>Key:</span>
          <input id="license-key-input" type="text" autocomplete="off" spellcheck="false" />
        </div>
      </div>
      <div class="startup-actions">
        <button id="license-login" type="button" class="primary">Login</button>
      </div>
    </div>
  </div>
`;

function renderDebugShell() {
  app.innerHTML = debugMarkup;
}

function renderLicenseShell() {
  app.innerHTML = licenseMarkup;
}

function renderLockedShell() {
  app.innerHTML = '<div id="locked-shell"></div>';
}

function renderErrorState(container: HTMLElement, message: string) {
  container.replaceChildren();
  const error = document.createElement("div");
  error.className = "error-state";
  error.textContent = message;
  container.appendChild(error);
}

function renderMainShell() {
  app.innerHTML = mainMarkup;
  bindMainEvents();
  updateUserLink();
  updateLicenseKeyDisplay();
  updateLicenseLabel();
  updateStatusBar();
}

// --- State ---
let basePath = "";
let exePath = "";
let processName = "";
let accountsPath = "";
let fingerprint = "";
let startupTime = Date.now();
let hotkeyEventsReady = false;
let releaseChannel: "stable" | "beta" = "stable";
let installedChannel: "stable" | "beta" = "stable";
let debugFeaturesEnabled = false;
let autoLoginEnabled = true;
let closeInProgress = false;
let launchCount = 0;
let hideCount = 0;
let totalUptime = 0;
let accountCount = 0;
let licenseKey = "";
let statusRefreshBusy = false;
let activeHotkeyActions = new Map<string, string>();
let activeHotkeyRecorder: { cancel: () => void } | null = null;
let runningHotkeyAction: string | null = null;
const pressedRuntimeHotkeys = new Set<string>();
let licenseHeartbeatTimer: number | null = null;
let licenseReconnectTimer: number | null = null;
let licenseHeartbeatRunning = false;
let licenseHeartbeatLocked = false;
let licenseReconnectOverlay: HTMLElement | null = null;
let lastLicenseHeartbeatReason = "";
let updateChoiceActive = false;
let updateInstallActive = false;
let updateCheckBusy = false;

const DEBUG_LOG_KEY = "lmplus.debug.logs";
const MAX_DEBUG_LOGS = 180;
const LICENSE_HEARTBEAT_INTERVAL_MS = 60 * 1000;
const LICENSE_HEARTBEAT_RETRY_MS = 15 * 1000;

type DebugLevel = "info" | "warn" | "error";

interface DebugEntry {
  time: string;
  level: DebugLevel;
  message: string;
  details?: string;
}

function normalizeChannel(value: unknown, fallback: "stable" | "beta" = "stable"): "stable" | "beta" {
  return value === "beta" ? "beta" : value === "stable" ? "stable" : fallback;
}

function normalizeHotkey(value: string): string {
  return value
    .split("+")
    .map((part) => part.trim().toLocaleLowerCase())
    .filter(Boolean)
    .join("+");
}

function hotkeyFromKeyboardEvent(event: KeyboardEvent): string | null {
  const keyName = event.key;
  if (keyName === "Control" || keyName === "Alt" || keyName === "Shift" || keyName === "Meta") {
    return null;
  }
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Win");
  parts.push(keyName.length === 1 ? keyName.toUpperCase() : keyName);
  return parts.join("+");
}

function isEditableTarget(target: EventTarget | null): boolean {
  const el = target instanceof HTMLElement ? target : null;
  if (!el) return false;
  return Boolean(el.closest("input, textarea, select, [contenteditable='true']"));
}

function validateAccountNameInput(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) return "Account name cannot be empty.";
  if (trimmed.length > 48) return "Account name is too long.";
  if (trimmed === "." || trimmed === ".." || trimmed.endsWith(".")) {
    return "Account name is not allowed.";
  }
  if (!/^[A-Za-z0-9][A-Za-z0-9 _.-]*$/.test(trimmed)) {
    return "Use only letters, numbers, spaces, dash, underscore, or dot.";
  }
  return null;
}

function validateBasePathInput(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) return "App path cannot be empty.";
  if (trimmed.length > 260) return "App path is too long.";
  if (/[\0\r\n<>"|?*&^$`;]/.test(trimmed)) {
    return "App path contains unsupported characters.";
  }
  const hasWindowsDrive = /^[A-Za-z]:[\\/]/.test(trimmed);
  const hasUncPrefix = /^\\\\|^\/\//.test(trimmed);
  const hasUnixRoot = trimmed.startsWith("/");
  if (!hasWindowsDrive && !hasUncPrefix && !hasUnixRoot) {
    return "Use a full folder path, such as C:/LordsMobile.";
  }
  const pathWithoutDrive = hasWindowsDrive ? trimmed.slice(2) : trimmed;
  if (pathWithoutDrive.includes(":")) {
    return "App path contains unsupported characters.";
  }
  const last = trimmed.split(/[\\/]+/).filter(Boolean).pop()?.toLocaleLowerCase() || "";
  if (last === "accounts" || last === "userdata") {
    return "Select the Lords Mobile base folder, not a subfolder.";
  }
  return null;
}

function setupForegroundHotkeyHandler() {
  document.addEventListener("keydown", (event) => {
    if (!document.hasFocus() || activeHotkeyRecorder || isEditableTarget(event.target)) return;
    const hotkey = hotkeyFromKeyboardEvent(event);
    if (!hotkey) return;
    const normalized = normalizeHotkey(hotkey);
    const action = activeHotkeyActions.get(normalized);
    if (!action) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.repeat || pressedRuntimeHotkeys.has(normalized) || runningHotkeyAction) return;
    pressedRuntimeHotkeys.add(normalized);
    runningHotkeyAction = action;
    hideContextMenu();
    void executeHotkeyAction(action)
      .catch((error) => {
        addDebugLog("Hotkey action failed", "warn", String(error));
      })
      .finally(() => {
        runningHotkeyAction = null;
      });
  }, true);
  document.addEventListener("keyup", (event) => {
    const hotkey = hotkeyFromKeyboardEvent(event);
    if (hotkey) {
      pressedRuntimeHotkeys.delete(normalizeHotkey(hotkey));
    }
  }, true);
  window.addEventListener("blur", () => {
    pressedRuntimeHotkeys.clear();
  });
}

function cancelActiveHotkeyRecorder() {
  activeHotkeyRecorder?.cancel();
  activeHotkeyRecorder = null;
}

function readDebugLog(): DebugEntry[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(DEBUG_LOG_KEY) || "[]");
    return Array.isArray(parsed) ? parsed.slice(-MAX_DEBUG_LOGS) : [];
  } catch {
    return [];
  }
}

function writeDebugLog(entries: DebugEntry[]) {
  try {
    localStorage.setItem(DEBUG_LOG_KEY, JSON.stringify(entries.slice(-MAX_DEBUG_LOGS)));
  } catch {
    // Ignore storage failures; debug logging must never break the app.
  }
}

function addDebugLog(message: string, level: DebugLevel = "info", details?: unknown) {
  if (!debugFeaturesEnabled && !isDebugWindow) return;
  const entry: DebugEntry = {
    time: new Date().toLocaleTimeString(),
    level,
    message,
    details: details === undefined
      ? undefined
      : typeof details === "string"
        ? details
        : JSON.stringify(details),
  };
  writeDebugLog([...readDebugLog(), entry]);
}

function formatLicenseKey(key: string): string {
  if (!key) return "-";
  const trimmed = key.trim();
  if (trimmed.length <= 14) return trimmed;
  return `${trimmed.slice(0, 6)}...${trimmed.slice(-4)}`;
}

async function copyText(value: string, label = "Value") {
  const text = value.trim();
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const fallback = document.createElement("textarea");
    fallback.value = text;
    fallback.style.position = "fixed";
    fallback.style.opacity = "0";
    document.body.appendChild(fallback);
    fallback.select();
    document.execCommand("copy");
    fallback.remove();
  }
  addDebugLog(`${label} copied`);
}

function updateUserLink() {
  const link = document.getElementById("user-link") as HTMLButtonElement | null;
  if (!link) return;
  link.textContent = "Coming soon";
  link.title = "User accounts will be linked here later.";
}

function updateLicenseKeyDisplay() {
  const valueButton = document.getElementById("license-key-value") as HTMLButtonElement | null;
  const copyButton = document.getElementById("license-key-copy") as HTMLButtonElement | null;
  if (!valueButton || !copyButton) return;
  const hasKey = Boolean(licenseKey.trim());
  valueButton.textContent = formatLicenseKey(licenseKey);
  valueButton.title = hasKey ? "Copy license key" : "No saved license key";
  valueButton.disabled = !hasKey;
  copyButton.disabled = !hasKey;
}

async function refreshAccountButtons() {
  if (statusRefreshBusy) return;
  statusRefreshBusy = true;
  updateStatusBar();
  try {
    await refreshAccounts();
    addDebugLog("Account list refreshed");
  } finally {
    statusRefreshBusy = false;
    updateStatusBar();
  }
}

// --- Context menu ---
let contextMenu: HTMLElement | null = null;
function showContextMenu(x: number, y: number, items: { label: string; action: () => void }[]) {
  hideContextMenu();
  const menu = document.createElement("div");
  menu.className = "context-menu";
  menu.style.visibility = "hidden";
  for (const item of items) {
    const el = document.createElement("div");
    el.className = "context-menu-item";
    el.textContent = item.label;
    el.addEventListener("click", () => {
      hideContextMenu();
      item.action();
    });
    menu.appendChild(el);
  }
  document.body.appendChild(menu);
  const rect = menu.getBoundingClientRect();
  const margin = 8;
  const left = Math.min(
    Math.max(margin, x),
    Math.max(margin, window.innerWidth - rect.width - margin),
  );
  const top = Math.min(
    Math.max(margin, y),
    Math.max(margin, window.innerHeight - rect.height - margin),
  );
  menu.style.left = `${left}px`;
  menu.style.top = `${top}px`;
  menu.style.visibility = "visible";
  contextMenu = menu;
  setTimeout(() => {
    document.addEventListener("click", hideContextMenu, { once: true });
  }, 0);
}

function hideContextMenu() {
  if (contextMenu) {
    contextMenu.remove();
    contextMenu = null;
  }
}

// --- Modal helper ---
function showModal(
  title: string,
  content: HTMLElement,
  width?: string,
  onDismiss?: () => void,
  dismissible = true,
): HTMLElement {
  const overlay = document.createElement("div");
  overlay.className = "modal-overlay";
  const modal = document.createElement("div");
  modal.className = "modal";
  if (width) modal.style.width = width;
  const h3 = document.createElement("h3");
  h3.textContent = title;
  modal.appendChild(h3);
  modal.appendChild(content);
  overlay.appendChild(modal);
  overlay.addEventListener("click", (e) => {
    if (dismissible && e.target === overlay) {
      overlay.remove();
      onDismiss?.();
    }
  });
  document.body.appendChild(overlay);
  return overlay;
}

function closeModal(overlay: HTMLElement) {
  overlay.remove();
}

function showMessageDialog(title: string, message: string): Promise<void> {
  return new Promise((resolve) => {
    const content = document.createElement("div");
    const text = document.createElement("div");
    text.className = "dialog-message";
    text.textContent = message;
    content.appendChild(text);

    const btnBar = document.createElement("div");
    btnBar.className = "modal-buttons";
    const ok = document.createElement("button");
    ok.className = "primary";
    ok.textContent = "OK";
    btnBar.appendChild(ok);
    content.appendChild(btnBar);

    let overlay: HTMLElement;
    let settled = false;
    const settle = () => {
      if (settled) return;
      settled = true;
      closeModal(overlay);
      resolve();
    };

    overlay = showModal(title, content, undefined, settle);
    ok.addEventListener("click", settle);
    ok.focus();
  });
}

function showConfirmDialog(
  title: string,
  message: string,
  okLabel = "OK",
  cancelLabel = "Cancel",
): Promise<boolean> {
  return new Promise((resolve) => {
    const content = document.createElement("div");
    const text = document.createElement("div");
    text.className = "dialog-message";
    text.textContent = message;
    content.appendChild(text);

    const btnBar = document.createElement("div");
    btnBar.className = "modal-buttons";
    const cancel = document.createElement("button");
    cancel.textContent = cancelLabel;
    const ok = document.createElement("button");
    ok.className = "primary";
    ok.textContent = okLabel;
    btnBar.append(cancel, ok);
    content.appendChild(btnBar);

    let overlay: HTMLElement;
    let settled = false;
    const settle = (value: boolean) => {
      if (settled) return;
      settled = true;
      closeModal(overlay);
      resolve(value);
    };

    overlay = showModal(title, content, undefined, () => settle(false));
    cancel.addEventListener("click", () => settle(false));
    ok.addEventListener("click", () => settle(true));
    ok.focus();
  });
}

function showCloseChoiceDialog(): Promise<"tray" | "exit" | "cancel"> {
  return new Promise((resolve) => {
    const content = document.createElement("div");
    content.className = "close-choice-dialog";
    const text = document.createElement("div");
    text.className = "dialog-message";
    text.textContent = "Choose what LMPlus should do when you close it.";
    content.appendChild(text);

    const btnBar = document.createElement("div");
    btnBar.className = "dialog-actions close-choice-actions";

    const cancel = document.createElement("button");
    cancel.textContent = "Cancel";
    const exit = document.createElement("button");
    exit.className = "danger";
    exit.textContent = "Exit";
    const tray = document.createElement("button");
    tray.className = "primary";
    tray.textContent = "Minimize to tray";

    btnBar.append(cancel, exit, tray);
    content.appendChild(btnBar);

    let overlay: HTMLElement;
    let settled = false;
    const settle = (value: "tray" | "exit" | "cancel") => {
      if (settled) return;
      settled = true;
      closeModal(overlay);
      resolve(value);
    };

    overlay = showModal("Close LMPlus", content, "488px", () => settle("cancel"));
    cancel.addEventListener("click", () => settle("cancel"));
    exit.addEventListener("click", () => settle("exit"));
    tray.addEventListener("click", () => settle("tray"));
    tray.focus();
  });
}

function formatByteSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  const digits = index === 0 ? 0 : value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${units[index]}`;
}

function formatTransferRate(bytesPerSecond?: number | null): string {
  if (!bytesPerSecond || bytesPerSecond <= 0) return "Calculating...";
  return `${formatByteSize(bytesPerSecond)}/s`;
}

function describeUpdatePhase(progress: api.UpdateProgress): string {
  switch (progress.phase) {
    case "preparing":
      return progress.message || "Preparing update...";
    case "downloading":
      return progress.message || "Downloading update package...";
    case "extracting":
      return progress.message || "Extracting update package...";
    case "installing":
      return progress.message || "Installing update...";
    case "restarting":
      return progress.message || "Restarting LMPlus...";
    case "error":
      return progress.message || "Update failed.";
    default:
      return "Updating LMPlus...";
  }
}

function sanitizeUpdateError(error: unknown): string {
  const raw = String(error ?? "").trim();
  const lower = raw.toLocaleLowerCase();
  if (
    lower.includes("download") ||
    lower.includes("request failed") ||
    lower.includes("timed out") ||
    lower.includes("timeout") ||
    lower.includes("network") ||
    lower.includes("dns")
  ) {
    return "Could not download the update. Please exit LMPlus and try again.";
  }
  if (lower.includes("github") || lower.includes("release") || lower.includes("asset")) {
    return "The update file could not be found. Please exit LMPlus and try again.";
  }
  if (lower.includes("signature") || lower.includes("zip") || lower.includes("extract")) {
    return "The update package could not be prepared. Please exit LMPlus and try again.";
  }
  return "LMPlus could not finish the update. Please exit and try again.";
}

async function showForcedUpdateChoice(update: api.UpdateInfo): Promise<"update" | "exit"> {
  updateChoiceActive = true;
  return new Promise((resolve) => {
    const latestChannel = normalizeChannel(update.release_channel, releaseChannel);
    const content = document.createElement("div");
    content.className = "update-choice-dialog";
    const text = document.createElement("div");
    text.className = "dialog-message";
    text.textContent = update.channel_switch_required
      ? `LMPlus must switch to the ${releaseChannelTitle(latestChannel)} build ${update.latest_version} before you continue.\nCurrent version: ${update.current_version}`
      : `LMPlus ${update.latest_version} is ready.\nCurrent version: ${update.current_version}\n\nPlease update now before continuing.`;
    content.appendChild(text);

    const btnBar = document.createElement("div");
    btnBar.className = "modal-buttons";
    const exit = document.createElement("button");
    exit.className = "danger";
    exit.textContent = "Exit";
    const updateBtn = document.createElement("button");
    updateBtn.className = "primary";
    updateBtn.textContent = "Update";
    btnBar.append(exit, updateBtn);
    content.appendChild(btnBar);

    let overlay: HTMLElement;
    let settled = false;
    const settle = (value: "update" | "exit") => {
      if (settled) return;
      settled = true;
      updateChoiceActive = false;
      closeModal(overlay);
      resolve(value);
    };

    overlay = showModal("Update Required", content, "430px", undefined, false);
    exit.addEventListener("click", () => settle("exit"));
    updateBtn.addEventListener("click", () => settle("update"));
    updateBtn.focus();
  });
}

async function performAppUpdateWithProgress(update: api.UpdateInfo): Promise<void> {
  if (updateInstallActive) return;
  updateInstallActive = true;
  const content = document.createElement("div");
  content.className = "update-progress-dialog";
  content.innerHTML = `
    <div class="about-line">Updating LMPlus ${update.latest_version}</div>
    <div class="update-progress-copy" id="update-progress-copy">Preparing update...</div>
    <div class="update-progress-track" aria-hidden="true">
      <div class="update-progress-bar indeterminate" id="update-progress-bar"></div>
    </div>
    <div class="update-progress-meta">
      <span id="update-progress-size">Waiting for download...</span>
      <span id="update-progress-speed">Calculating...</span>
    </div>
    <div class="update-progress-detail" id="update-progress-detail">LMPlus will close and reopen when the update is ready.</div>
    <div class="modal-buttons">
      <button id="update-progress-exit" class="danger" hidden>Exit</button>
    </div>
  `;

  const overlay = showModal("Updating LMPlus", content, "460px", undefined, false);
  overlay.dataset.updateProgress = "1";
  const copy = content.querySelector("#update-progress-copy") as HTMLElement;
  const bar = content.querySelector("#update-progress-bar") as HTMLElement;
  const size = content.querySelector("#update-progress-size") as HTMLElement;
  const speed = content.querySelector("#update-progress-speed") as HTMLElement;
  const detail = content.querySelector("#update-progress-detail") as HTMLElement;
  const exit = content.querySelector("#update-progress-exit") as HTMLButtonElement;

  const updateUi = (progress: api.UpdateProgress) => {
    copy.textContent = describeUpdatePhase(progress);
    const total = progress.total ?? 0;
    const hasFiniteTotal = total > 0 && Number.isFinite(total);
    bar.classList.remove("failed");
    exit.hidden = true;

    if (progress.phase === "downloading") {
      if (hasFiniteTotal) {
        const percent = Math.max(0, Math.min(100, (progress.downloaded / total) * 100));
        bar.classList.remove("indeterminate");
        bar.style.width = `${percent}%`;
        size.textContent = `${formatByteSize(progress.downloaded)} / ${formatByteSize(total)}`;
      } else {
        bar.classList.add("indeterminate");
        bar.style.width = "42%";
        size.textContent = `${formatByteSize(progress.downloaded)} downloaded`;
      }
      speed.textContent = formatTransferRate(progress.speed_bps);
      detail.textContent = "LMPlus will close and reopen when the update is ready.";
      return;
    }

    if (progress.phase === "extracting") {
      if (hasFiniteTotal) {
        const percent = Math.max(0, Math.min(100, (progress.downloaded / total) * 100));
        bar.classList.remove("indeterminate");
        bar.style.width = `${percent}%`;
        size.textContent = `${progress.downloaded} / ${total} files`;
      } else {
        bar.classList.add("indeterminate");
        bar.style.width = "42%";
        size.textContent = "Extracting files...";
      }
      speed.textContent = "Please wait";
      detail.textContent = progress.message || "Preparing the new LMPlus files.";
      return;
    }

    if (progress.phase === "error") {
      bar.classList.remove("indeterminate");
      bar.style.width = "100%";
      bar.classList.add("failed");
      size.textContent = "Update could not continue.";
      speed.textContent = "Exit required";
      detail.textContent = progress.message || "LMPlus could not finish the update.";
      exit.hidden = false;
      exit.focus();
      return;
    }

    bar.classList.add("indeterminate");
    bar.style.width = "42%";
    size.textContent = progress.message || "Working...";
    speed.textContent = "Please wait";
    detail.textContent = progress.phase === "restarting"
      ? "LMPlus is handing control to the updater now."
      : "LMPlus will close and reopen when the update is ready.";
  };

  updateUi({
    phase: "preparing",
    unit: "none",
    downloaded: 0,
    total: null,
    speed_bps: null,
    message: "Preparing update...",
  });

  exit.addEventListener("click", () => {
    void closeGameAndExit();
  });

  let stopListening: (() => void) | null = null;
  let failed = false;
  try {
    stopListening = await listen<api.UpdateProgress>("lmplus-update-progress", (event) => {
      updateUi(event.payload);
    });
    await api.performUpdate(update.download_url || update.zip_url);
  } catch (error) {
    failed = true;
    updateUi({
      phase: "error",
      unit: "none",
      downloaded: 0,
      total: null,
      speed_bps: null,
      message: sanitizeUpdateError(error),
    });
  } finally {
    stopListening?.();
    updateInstallActive = false;
    if (!failed) {
      updateUi({
        phase: "restarting",
        unit: "none",
        downloaded: 0,
        total: null,
        speed_bps: null,
        message: "Waiting for LMPlus to restart...",
      });
    }
  }
}

function showInputDialog(
  title: string,
  label: string,
  initialValue = "",
  okLabel = "OK",
): Promise<string | null> {
  return new Promise((resolve) => {
    const content = document.createElement("div");
    content.className = "input-dialog";
    const inputLabel = document.createElement("label");
    inputLabel.textContent = label;
    const input = document.createElement("input");
    input.type = "text";
    input.value = initialValue;
    content.append(inputLabel, input);

    const btnBar = document.createElement("div");
    btnBar.className = "modal-buttons";
    const cancel = document.createElement("button");
    cancel.textContent = "Cancel";
    const ok = document.createElement("button");
    ok.className = "primary";
    ok.textContent = okLabel;
    btnBar.append(cancel, ok);
    content.appendChild(btnBar);

    let overlay: HTMLElement;
    let settled = false;
    const settle = (value: string | null) => {
      if (settled) return;
      settled = true;
      closeModal(overlay);
      resolve(value);
    };

    overlay = showModal(title, content, undefined, () => settle(null));
    cancel.addEventListener("click", () => settle(null));
    ok.addEventListener("click", () => settle(input.value));
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") settle(input.value);
      if (e.key === "Escape") settle(null);
    });
    input.focus();
    input.select();
  });
}

// --- Account list ---
async function refreshAccounts() {
  const list = document.getElementById("account-list")!;
  list.innerHTML = "";

  if (!accountsPath) {
    accountCount = 0;
    updateStatusBar(0);
    return;
  }

  try {
    const accounts = await api.loadAccounts(basePath);
    accountCount = accounts.length;
    if (accounts.length === 0) {
      list.innerHTML = '<div class="empty-state">Account folder is empty.</div>';
      updateStatusBar(0);
      return;
    }
    for (const acc of accounts) {
      const btn = document.createElement("button");
      btn.className = "account-btn";
      btn.textContent = acc;
      btn.addEventListener("click", () => switchAccount(acc));
      btn.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        showContextMenu(e.clientX, e.clientY, [
          {
            label: "Rename",
            action: () => renameAccount(acc),
          },
          {
            label: "Delete",
            action: () => deleteAccount(acc),
          },
        ]);
      });
      list.appendChild(btn);
    }
    updateStatusBar(accounts.length);
  } catch (e) {
    renderErrorState(list, `Error: ${e}`);
    addDebugLog("Failed to refresh accounts", "error", String(e));
  }
}

async function executeHotkeyAction(action: string) {
  while (action.startsWith("direct:")) {
    action = action.slice("direct:".length);
  }
  if (DIRECT_ACTION_SPECS[action] || action.startsWith("zoom:")) {
    await executeDirectActionSpec(action);
  } else if (action.startsWith("swap_")) {
    await api.executeMacro(action.slice(5), exePath, processName);
  } else if (action.startsWith("misc_")) {
    await api.executeMacro(action.slice(5), exePath, processName);
  } else {
    await switchAccount(action);
  }
}

const DIRECT_ACTION_SPECS: Record<string, api.DirectAction> = {
  formation_inf_phalanx: { action: "formation", index: 0 },
  formation_range_phalanx: { action: "formation", index: 1 },
  formation_cav_phalanx: { action: "formation", index: 2 },
  formation_inf_wedge: { action: "formation", index: 3 },
  formation_range_wedge: { action: "formation", index: 4 },
  formation_cav_wedge: { action: "formation", index: 5 },
  map3dview_full: { action: "map3dview", mode: 0 },
  map3dview_balanced: { action: "map3dview", mode: 1 },
  map3dview_none: { action: "map3dview", mode: 2 },
  switch_account_direct: { action: "switch_account" },
};

async function executeDirectActionSpec(spec: string) {
  const known = DIRECT_ACTION_SPECS[spec];
  if (known) {
    await executeDirect(known, spec);
    return;
  }
  if (spec.startsWith("zoom:")) {
    const value = Number(spec.slice("zoom:".length));
    if (Number.isFinite(value)) {
      await executeDirect({ action: "mapzoom", value }, spec);
      return;
    }
  }
  addDebugLog("Unknown direct action", "warn", spec);
}

async function executeDirect(action: api.DirectAction, label: string) {
  try {
    const detail = await api.executeDirectAction(exePath, action);
    addDebugLog(`Direct action: ${label}`, "info", detail);
  } catch (e) {
    addDebugLog(`Direct action failed: ${label}`, "error", String(e));
    await showMessageDialog("Direct Action", `Failed to run ${label}:\n${e}`);
  }
}

async function setupHotkeyEvents() {
  if (hotkeyEventsReady) return;
  hotkeyEventsReady = true;
  setupForegroundHotkeyHandler();
  await listen<string>("lmplus-hotkey", async (event) => {
    await executeHotkeyAction(event.payload);
  });
}

async function buildHotkeyRegistrationJson() {
  const flat = await api.getHotkeySettings();
  const hotkeys: Record<string, string> = {};
  const swapHotkeys: Record<string, string> = {};

  for (const [key, value] of Object.entries(flat)) {
    if (!value) continue;
    if (key.startsWith("hotkeys.")) {
      hotkeys[key.slice("hotkeys.".length)] = value;
    } else if (key.startsWith("swap_hotkeys.")) {
      swapHotkeys[key.slice("swap_hotkeys.".length)] = value;
    } else if (key.startsWith("direct.")) {
      hotkeys[key.slice("direct.".length)] = value;
    }
  }

  const miscHotkeys = (await api.loadMiscCfg())
    .filter((m) => !!m.hotkey)
    .map((m) => ({ name: m.name, hotkey: m.hotkey }));

  return JSON.stringify({
    hotkeys,
    swap_hotkeys: swapHotkeys,
    misc_hotkeys: miscHotkeys,
  });
}

async function reregisterHotkeys() {
  try {
    const registered = await api.registerHotkeys(accountsPath, await buildHotkeyRegistrationJson());
    activeHotkeyActions = new Map(
      registered.map((entry) => {
        const action = entry.type === "swap"
          ? `swap_${entry.name}`
          : entry.type === "misc"
            ? `misc_${entry.name}`
            : DIRECT_ACTION_SPECS[entry.name]
              ? `direct:${entry.name}`
              : entry.name;
        return [normalizeHotkey(entry.hotkey), action];
      }),
    );
    addDebugLog("Hotkeys registered");
  } catch (e) {
    console.warn("Hotkey registration failed:", e);
    activeHotkeyActions.clear();
    addDebugLog("Hotkey registration failed", "warn", String(e));
  }
}

let switchInFlight = false;

async function switchAccount(name: string) {
  if (switchInFlight) {
    addDebugLog("Account switch already in progress, ignoring", "warn", name);
    return;
  }
  switchInFlight = true;
  try {
    addDebugLog("Switching account", "info", name);
    await api.switchAccount(basePath, name);
    await api.restartGame(exePath, processName, true);
    addDebugLog("Account switched", "info", name);
  } catch (e) {
    addDebugLog("Account switch failed", "error", String(e));
    await showMessageDialog("Switch Account", `Failed to switch account:\n${e}`);
  } finally {
    switchInFlight = false;
  }
}

async function renameAccount(oldName: string) {
  const newName = await showInputDialog("Rename Account", "Enter new account name:", oldName);
  if (newName && newName.trim() && newName !== oldName) {
    const validationError = validateAccountNameInput(newName);
    if (validationError) {
      await showMessageDialog("Rename Account", validationError);
      return;
    }
    try {
      await api.renameAccount(basePath, oldName, newName.trim());
      await refreshAccounts();
      await reregisterHotkeys();
      addDebugLog("Account renamed", "info", `${oldName} -> ${newName.trim()}`);
    } catch (e) {
      addDebugLog("Rename account failed", "warn", String(e));
      await showMessageDialog("Rename Account", "LMPlus could not rename that account.");
    }
  }
}

async function deleteAccount(name: string) {
  if (await showConfirmDialog("Delete Account", `Confirm delete '${name}'?`, "Delete")) {
    try {
      await api.deleteAccount(basePath, name);
      await refreshAccounts();
      await reregisterHotkeys();
      addDebugLog("Account deleted", "info", name);
    } catch (e) {
      await showMessageDialog("Delete Account", `Failed to delete:\n${e}`);
    }
  }
}

// --- Add Account dialog ---
function showAddAccountDialog() {
  const content = document.createElement("div");
  content.className = "input-dialog";
  content.innerHTML = `
    <label>Enter Account Name:</label>
    <input id="account-name-input" type="text" />
    <div class="modal-buttons">
      <button id="btn-cancel">Cancel</button>
      <button id="btn-ok" class="primary">OK</button>
    </div>
  `;
  const overlay = showModal("Add Account", content);

  content.querySelector("#btn-cancel")!.addEventListener("click", () => closeModal(overlay));
  content.querySelector("#btn-ok")!.addEventListener("click", async () => {
    const name = (content.querySelector("#account-name-input") as HTMLInputElement).value.trim();
    const validationError = validateAccountNameInput(name);
    if (validationError) {
      await showMessageDialog("Add Account", validationError);
      return;
    }
    closeModal(overlay);
    try {
      await api.addAccount(basePath, name);
      await refreshAccounts();
      await reregisterHotkeys();
      addDebugLog("Account added", "info", name);
    } catch (e) {
      addDebugLog("Add account failed", "warn", String(e));
      const message = String(e ?? "").toLocaleLowerCase().includes("already exists")
        ? "An account with that name already exists."
        : "LMPlus could not save that account. Check the app path and account name.";
      await showMessageDialog("Add Account", message);
    }
  });
}

// --- Hotkey dialog ---
let currentHotkeyTab = 0;
let hotkeyDialogOverlay: HTMLElement | null = null;

interface HotkeyRow {
  name: string;
  hotkey: string;
}

async function showHotkeyDialog() {
  cancelActiveHotkeyRecorder();
  let shell: HTMLElement;
  if (!hotkeyDialogOverlay || !hotkeyDialogOverlay.isConnected) {
    shell = document.createElement("div");
    shell.className = "hotkey-dialog hotkey-dialog-shell";
    hotkeyDialogOverlay = showModal("Hotkey Assignments", shell, "540px", () => {
      cancelActiveHotkeyRecorder();
      hotkeyDialogOverlay = null;
    });
  } else {
    const existingShell = hotkeyDialogOverlay.querySelector(".hotkey-dialog-shell");
    if (!existingShell) {
      hotkeyDialogOverlay.remove();
      hotkeyDialogOverlay = null;
      void showHotkeyDialog();
      return;
    }
    shell = existingShell as HTMLElement;
  }

  shell.innerHTML = "";

  const tabs = ["Account", "Formation", "Misc"];
  const tabBar = document.createElement("div");
  tabBar.className = "hotkey-tabs";
  tabs.forEach((t, i) => {
    const tab = document.createElement("div");
    tab.className = `hotkey-tab${i === currentHotkeyTab ? " active" : ""}`;
    tab.textContent = t;
    tab.addEventListener("click", () => {
      if (currentHotkeyTab === i) return;
      cancelActiveHotkeyRecorder();
      currentHotkeyTab = i;
      void showHotkeyDialog();
    });
    tabBar.appendChild(tab);
  });
  shell.appendChild(tabBar);

  const body = document.createElement("div");
  body.className = "hotkey-dialog-body";
  shell.appendChild(body);

  if (currentHotkeyTab === 2) {
    const warn = document.createElement("div");
    warn.className = "hotkey-warning";
    warn.textContent = "Set Lords Mobile to the lowest resolution before configuring macros!";
    body.appendChild(warn);
  }

  const tableWrap = document.createElement("div");
  tableWrap.className = "hotkey-table-wrap";
  const table = document.createElement("table");
  table.className = "hotkey-table";
  table.innerHTML =
    `<thead><tr><th>${currentHotkeyTab === 1 ? "Formation" : currentHotkeyTab === 2 ? "Name" : "Account"}</th><th>Hotkey</th></tr></thead><tbody></tbody>`;
  tableWrap.appendChild(table);
  body.appendChild(tableWrap);
  const tbody = table.querySelector("tbody")!;

  const rows: HotkeyRow[] = [];
  try {
    if (currentHotkeyTab === 0) {
      const accounts = await api.loadAccounts(basePath);
      const hotkeys = await api.getHotkeySettings();
      for (const acc of accounts) {
        rows.push({ name: acc, hotkey: (hotkeys[`hotkeys.${acc}`] as string) || "" });
      }
    } else if (currentHotkeyTab === 1) {
      // Six native formation slots, invoked directly in-game via the agent.
      const formations = [
        "Inf Phal", "Range Phal", "Cav Phal", "Inf Wedge", "Range Wedge", "Cav Wedge",
      ];
      const directNames: Record<string, string> = {
        "Inf Phal": "formation_inf_phalanx",
        "Range Phal": "formation_range_phalanx",
        "Cav Phal": "formation_cav_phalanx",
        "Inf Wedge": "formation_inf_wedge",
        "Range Wedge": "formation_range_wedge",
        "Cav Wedge": "formation_cav_wedge",
      };
      const hotkeys = await api.getHotkeySettings();
      for (const f of formations) {
        rows.push({ name: f, hotkey: (hotkeys[`direct.${directNames[f]}`] as string) || "" });
      }
    } else {
      const macros = await api.loadMiscCfg();
      for (const m of macros) {
        rows.push({ name: m.name, hotkey: m.hotkey || "" });
      }
    }
  } catch (e) {
    tbody.replaceChildren();
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 2;
    td.textContent = `Error: ${e}`;
    tr.appendChild(td);
    tbody.appendChild(tr);
  }

  const renderRows = () => {
    tbody.innerHTML = "";
    rows.forEach((row) => {
      const tr = document.createElement("tr");
      const nameCell = document.createElement("td");
      nameCell.textContent = row.name;
      nameCell.title = row.name;
      const cell = document.createElement("td");
      cell.className = "hotkey-cell";
      cell.textContent = row.hotkey || "Unassigned";
      cell.title = row.hotkey || "Unassigned";
      tr.append(nameCell, cell);
      cell.addEventListener("click", () => startRecording(row, cell));
      tr.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        const items = [{ label: "Reset Hotkey", action: () => resetHotkey(row) }];
        if (currentHotkeyTab === 2) {
          items.push({ label: "Delete Macro", action: () => deleteMacro(row) });
        }
        showContextMenu(e.clientX, e.clientY, items);
      });
      tbody.appendChild(tr);
    });
    if (currentHotkeyTab === 2) {
      const addTr = document.createElement("tr");
      const addCell = document.createElement("td");
      addCell.colSpan = 2;
      addCell.className = "hotkey-add-row";
      addCell.textContent = "+ Add Macro";
      addTr.appendChild(addCell);
      addTr.addEventListener("click", () => addMacro());
      tbody.appendChild(addTr);
    }
  };

  const resetHotkey = async (row: HotkeyRow) => {
    try {
      if (currentHotkeyTab === 0) {
        await api.removeHotkey("hotkeys", row.name);
      } else if (currentHotkeyTab === 1) {
        await api.removeHotkey("direct", FORMATION_ROW_TO_DIRECT[row.name]?.slice("direct:".length) || row.name);
      } else {
        row.hotkey = "";
        const macros = await api.loadMiscCfg();
        const updated = macros.map((m) => (m.name === row.name ? { ...m, hotkey: "" } : m));
        await api.saveMiscCfg(updated);
      }
      row.hotkey = "";
      renderRows();
      await reregisterHotkeys();
    } catch (e) {
      await showMessageDialog("Hotkeys", `Failed to reset hotkey:\n${e}`);
    }
  };

  const addMacro = async () => {
    const name = await showInputDialog("Add Macro", "Enter macro name:");
    if (!name || !name.trim()) return;
    try {
      const macros = await api.loadMiscCfg();
      if (macros.some((m) => m.name === name.trim())) {
        await showMessageDialog("Add Macro", "A macro with this name already exists.");
        return;
      }
      macros.push({ name: name.trim(), hotkey: "", points: [] });
      await api.saveMiscCfg(macros);
      rows.push({ name: name.trim(), hotkey: "" });
      renderRows();
      await reregisterHotkeys();
    } catch (e) {
      await showMessageDialog("Add Macro", `Failed to add macro:\n${e}`);
    }
  };

  const deleteMacro = async (row: HotkeyRow) => {
    try {
      const macros = await api.loadMiscCfg();
      await api.saveMiscCfg(macros.filter((m) => m.name !== row.name));
      const idx = rows.indexOf(row);
      if (idx !== -1) rows.splice(idx, 1);
      renderRows();
      await reregisterHotkeys();
    } catch (e) {
      await showMessageDialog("Delete Macro", `Failed to delete macro:\n${e}`);
    }
  };

  const FORMATION_ROW_TO_DIRECT: Record<string, string> = {
    "Inf Phal": "direct:formation_inf_phalanx",
    "Range Phal": "direct:formation_range_phalanx",
    "Cav Phal": "direct:formation_cav_phalanx",
    "Inf Wedge": "direct:formation_inf_wedge",
    "Range Wedge": "direct:formation_range_wedge",
    "Cav Wedge": "direct:formation_cav_wedge",
  };
  const actionForRow = (row: HotkeyRow, tab: number) => (
    tab === 1 ? (FORMATION_ROW_TO_DIRECT[row.name] || `swap_${row.name}`)
      : tab === 2 ? `misc_${row.name}` : row.name
  );

  const startRecording = (row: HotkeyRow, cell: HTMLElement) => {
    cancelActiveHotkeyRecorder();
    const tabAtStart = currentHotkeyTab;
    const originalHotkey = row.hotkey || "";
    cell.classList.add("recording");
    cell.textContent = "Press key...";

    const finish = () => {
      document.removeEventListener("keydown", handler, true);
      if (activeHotkeyRecorder?.cancel === cancel) {
        activeHotkeyRecorder = null;
      }
    };
    const cancel = () => {
      finish();
      cell.classList.remove("recording");
      cell.textContent = originalHotkey || "Unassigned";
      cell.title = originalHotkey || "Unassigned";
    };

    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const keyName = e.key;
      if (keyName === "Control" || keyName === "Alt" || keyName === "Shift" || keyName === "Meta") {
        return;
      }
      if (keyName === "Escape") {
        cancel();
        return;
      }
      if (keyName === " " || keyName === "Enter" || keyName === "Backspace") {
        void showMessageDialog("Hotkeys", "Space, Enter, and Backspace cannot be used as hotkeys.");
        cancel();
        return;
      }
      const hotkey = hotkeyFromKeyboardEvent(e);
      if (!hotkey) return;

      const normalized = normalizeHotkey(hotkey);
      const currentAction = actionForRow(row, tabAtStart);
      const taken = rows.some((r) => r !== row && normalizeHotkey(r.hotkey) === normalized)
        || Boolean(activeHotkeyActions.get(normalized) && activeHotkeyActions.get(normalized) !== currentAction);
      if (taken) {
        void showMessageDialog("Hotkeys", "This hotkey is already assigned.");
        cancel();
        return;
      }

      finish();
      row.hotkey = hotkey;
      cell.classList.remove("recording");
      cell.textContent = hotkey;
      cell.title = hotkey;

      (async () => {
        try {
          if (tabAtStart === 0) {
            await api.setHotkey("hotkeys", row.name, hotkey);
          } else if (tabAtStart === 1) {
            await api.setHotkey("direct", FORMATION_ROW_TO_DIRECT[row.name]?.slice("direct:".length) || row.name, hotkey);
          } else {
            const macros = await api.loadMiscCfg();
            const updated = macros.map((m) => (m.name === row.name ? { ...m, hotkey } : m));
            await api.saveMiscCfg(updated);
          }
          await reregisterHotkeys();
        } catch (err) {
          row.hotkey = originalHotkey;
          cell.textContent = originalHotkey || "Unassigned";
          cell.title = originalHotkey || "Unassigned";
          await showMessageDialog("Hotkeys", `Failed to save hotkey:\n${err}`);
        }
      })();
    };
    activeHotkeyRecorder = { cancel };
    document.addEventListener("keydown", handler, true);
  };
  renderRows();

  const btnBar = document.createElement("div");
  btnBar.className = "modal-buttons hotkey-actions";
  const closeBtn = document.createElement("button");
  closeBtn.textContent = "Close";
  closeBtn.addEventListener("click", () => {
    cancelActiveHotkeyRecorder();
    hotkeyDialogOverlay?.remove();
    hotkeyDialogOverlay = null;
  });
  btnBar.appendChild(closeBtn);
  shell.appendChild(btnBar);
}

// --- Settings dialog ---
async function showSettingsDialog() {
  const settings = await api.getSettings().catch(() => ({
    basePath: basePath,
    accountOrder: [],
    releaseChannel: releaseChannel,
    appliedReleaseChannel: releaseChannel,
    debugFeatures: debugFeaturesEnabled,
    autoLogin: autoLoginEnabled,
  }));
  releaseChannel = normalizeChannel(settings.releaseChannel, releaseChannel);
  installedChannel = normalizeChannel(settings.appliedReleaseChannel, releaseChannel);
  debugFeaturesEnabled = Boolean(settings.debugFeatures);
  autoLoginEnabled = settings.autoLogin !== false;

  const content = document.createElement("div");
  content.className = "settings-dialog";
  content.innerHTML = `
    <div class="settings-actions">
      <button id="settings-map-zoom">Map Zoom</button>
      <button id="settings-update-lm">Update LM</button>
      <button id="settings-about">About</button>
    </div>
    <div class="release-channel-row">
      <span class="release-channel-label">Channel</span>
      <div class="release-channel-toggle" id="release-channel-toggle">
        <button type="button" data-release-channel="stable">Stable</button>
        <button type="button" data-release-channel="beta">Beta</button>
      </div>
      <span id="release-channel-status"></span>
    </div>
    <div class="settings-sort" id="sort-list"></div>
    <div class="sort-actions">
      <button id="settings-sort" class="primary">Save Sortings</button>
      <button id="settings-reset">Reset Sortings</button>
    </div>
  `;

  const overlay = showModal("Settings", content, "460px");
  const status = content.querySelector("#release-channel-status") as HTMLElement;
  const channelButtons = Array.from(
    content.querySelectorAll<HTMLButtonElement>("#release-channel-toggle button"),
  );

  const syncChannelUi = (value: "stable" | "beta") => {
    channelButtons.forEach((btn) => {
      const active = btn.dataset.releaseChannel === value;
      btn.classList.toggle("active", active);
    });
    status.textContent = installedChannel === value
      ? `${releaseChannelTitle(value)} build installed`
      : `${releaseChannelTitle(installedChannel)} build installed`;
  };

  syncChannelUi(releaseChannel);
  for (const btn of channelButtons) {
    btn.addEventListener("click", async () => {
      const next = btn.dataset.releaseChannel as "stable" | "beta" | undefined;
      if (!next || next === releaseChannel) return;
      try {
        await api.setSettings({ releaseChannel: next });
        releaseChannel = next;
        syncChannelUi(next);
        updateStatusBar();
        addDebugLog("Release channel selected", "info", next);
        await checkForAppUpdate(true);
        const refreshed = await api.getSettings();
        installedChannel = normalizeChannel(refreshed.appliedReleaseChannel, installedChannel);
        syncChannelUi(releaseChannel);
      } catch (e) {
        await showMessageDialog("Settings", `Failed to switch channel:\n${e}`);
      }
    });
  }

  const sortList = content.querySelector("#sort-list") as HTMLElement;
  let draggedItem: HTMLElement | null = null;
  let draggedPointerId: number | null = null;
  try {
    const accounts = await api.loadAccounts(basePath);
    for (const acc of accounts) {
      const item = document.createElement("div");
      item.className = "settings-sort-item";
      item.textContent = acc;
      item.title = acc;
      item.addEventListener("pointerdown", (event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        draggedItem = item;
        draggedPointerId = event.pointerId;
        item.setPointerCapture(event.pointerId);
        item.classList.add("dragging");
      });
      item.addEventListener("pointermove", (event) => {
        if (draggedItem !== item) return;
        event.preventDefault();
        moveDraggedItem(event.clientY);
      });
      item.addEventListener("pointerup", () => endPointerDrag());
      item.addEventListener("pointercancel", () => endPointerDrag());
      sortList.appendChild(item);
    }
  } catch (e) {
      renderErrorState(sortList, `Error: ${e}`);
  }

  const moveDraggedItem = (clientY: number) => {
    if (!draggedItem) return;
    const afterElement = getDragAfterElement(sortList, clientY);
    if (afterElement === null) {
      sortList.appendChild(draggedItem);
    } else if (afterElement !== draggedItem) {
      sortList.insertBefore(draggedItem, afterElement);
    }
  };

  const endPointerDrag = () => {
    if (!draggedItem) return;
    if (draggedPointerId !== null && draggedItem.hasPointerCapture(draggedPointerId)) {
      draggedItem.releasePointerCapture(draggedPointerId);
    }
    draggedItem.classList.remove("dragging");
    draggedItem = null;
    draggedPointerId = null;
  };

  sortList.addEventListener("pointermove", (e) => {
    if (!draggedItem) return;
    e.preventDefault();
    moveDraggedItem(e.clientY);
  });

  content.querySelector("#settings-sort")!.addEventListener("click", async () => {
    const items = sortList.querySelectorAll(".settings-sort-item");
    const order = Array.from(items).map((el) => el.textContent!);
    await api.setAccountOrder(order);
    await refreshAccounts();
    await showMessageDialog("Settings", "Order saved.");
  });

  content.querySelector("#settings-reset")!.addEventListener("click", async () => {
    await api.setAccountOrder([]);
    await refreshAccounts();
    closeModal(overlay);
    showSettingsDialog();
  });

  content.querySelector("#settings-map-zoom")!.addEventListener("click", async () => {
    showMapZoomSlider();
  });

  content.querySelector("#settings-update-lm")!.addEventListener("click", async () => {
    try {
      await api.restartGame(exePath, processName, false);
      await api.launchLmUpdater();
    } catch (e) {
      await showMessageDialog("Update LM", `Error:\n${e}`);
    }
  });

  content.querySelector("#settings-about")!.addEventListener("click", async () => {
    await showAboutDialog();
  });
}

function showMapZoomSlider() {
  const content = document.createElement("div");
  content.className = "input-dialog";
  content.innerHTML = `
    <label>Map Zoom <span id="zoom-level-label">—</span></label>
    <input id="zoom-slider" type="range" min="0" max="100" step="1" value="50" style="width:100%" />
    <div class="modal-buttons">
      <button id="btn-zoom-out">−</button>
      <button id="btn-zoom-in">+</button>
      <button id="btn-close" class="primary">Close</button>
    </div>
  `;
  const overlay = showModal("Map Zoom", content);
  const slider = content.querySelector("#zoom-slider") as HTMLInputElement;
  const label = content.querySelector("#zoom-level-label") as HTMLSpanElement;
  let lastApplied = -1;

  const apply = async () => {
    const level = Number(slider.value) / 100;
    if (level === lastApplied) return;
    lastApplied = level;
    label.textContent = `${slider.value}%`;
    try {
      await api.executeDirectAction(exePath, { action: "mapzoom", value: level });
      addDebugLog("Map zoom", "info", `${slider.value}%`);
    } catch (e) {
      addDebugLog("Map zoom failed", "error", String(e));
      label.textContent = "error";
    }
  };

  let debounceTimer: number | undefined;
  slider.addEventListener("input", () => {
    label.textContent = `${slider.value}%`;
    window.clearTimeout(debounceTimer);
    debounceTimer = window.setTimeout(apply, 120);
  });
  content.querySelector("#btn-zoom-out")!.addEventListener("click", () => {
    slider.value = String(Math.max(0, Number(slider.value) - 10));
    void apply();
  });
  content.querySelector("#btn-zoom-in")!.addEventListener("click", () => {
    slider.value = String(Math.min(100, Number(slider.value) + 10));
    void apply();
  });
  content.querySelector("#btn-close")!.addEventListener("click", () => closeModal(overlay));
}

function getDragAfterElement(container: Element, y: number): Element | null {
  const draggableElements = Array.from(
    container.querySelectorAll<HTMLElement>(".settings-sort-item:not(.dragging)"),
  );

  return draggableElements.reduce<{ offset: number; element: Element | null }>(
    (closest, child) => {
      const box = child.getBoundingClientRect();
      const offset = y - box.top - box.height / 2;
      if (offset < 0 && offset > closest.offset) {
        return { offset, element: child };
      }
      return closest;
    },
    { offset: Number.NEGATIVE_INFINITY, element: null },
  ).element;
}

async function showAboutDialog() {
  const ver = await api.getVersion();
  const settings = await api.getSettings().catch(() => ({
    debugFeatures: debugFeaturesEnabled,
    autoLogin: autoLoginEnabled,
    releaseChannel,
    appliedReleaseChannel: installedChannel,
  }));
  debugFeaturesEnabled = Boolean(settings.debugFeatures);
  autoLoginEnabled = settings.autoLogin !== false;

  const content = document.createElement("div");
  content.className = "about-block";
  content.innerHTML = `
    <div class="about-line">LMPlus ${ver}</div>
    <div class="about-line">Developer: Meow<br>Contact: @.meow._<br>Copyrighted by NbP</div>
    <div class="about-toggle-group">
      <label class="debug-toggle">
        <input id="about-debug-toggle" type="checkbox" ${debugFeaturesEnabled ? "checked" : ""} />
        <span>Debugging features</span>
      </label>
      <label class="debug-toggle">
        <input id="about-auto-login-toggle" type="checkbox" ${autoLoginEnabled ? "checked" : ""} />
        <span>Auto Login</span>
      </label>
    </div>
    <div class="modal-buttons">
      <button id="about-open-debug" ${debugFeaturesEnabled ? "" : "hidden"}>Open Debug Window</button>
      <button id="about-check-update">Check for Update</button>
      <button id="about-close" class="primary">Close</button>
    </div>
  `;

  const overlay = showModal("About LMPlus", content, "420px");
  const debugToggle = content.querySelector("#about-debug-toggle") as HTMLInputElement;
  const autoLoginToggle = content.querySelector("#about-auto-login-toggle") as HTMLInputElement;
  const openDebug = content.querySelector("#about-open-debug") as HTMLButtonElement;

  debugToggle.addEventListener("change", async () => {
    if (debugToggle.checked) {
      const confirmed = await showConfirmDialog(
        "Debugging Features",
        "Debugging features keep extra logs and may reduce some performance while enabled.",
        "Enable",
      );
      if (!confirmed) {
        debugToggle.checked = false;
        return;
      }
      debugFeaturesEnabled = true;
      await api.setSettings({ debugFeatures: true });
      addDebugLog("Debugging features enabled");
      openDebug.hidden = false;
      await openDebugWindow();
    } else {
      debugFeaturesEnabled = false;
      await api.setSettings({ debugFeatures: false });
      openDebug.hidden = true;
      writeDebugLog([]);
      const debugWindow = await WebviewWindow.getByLabel("debug");
      if (debugWindow) {
        await debugWindow.close().catch(() => undefined);
      }
    }
  });

  autoLoginToggle.addEventListener("change", async () => {
    autoLoginEnabled = autoLoginToggle.checked;
    await api.setSettings({ autoLogin: autoLoginEnabled });
    addDebugLog("Auto login preference updated", "info", { autoLoginEnabled });
  });

  openDebug.addEventListener("click", () => {
    void openDebugWindow();
  });
  content.querySelector("#about-check-update")!.addEventListener("click", async () => {
    closeModal(overlay);
    await checkForAppUpdate(true);
  });
  content.querySelector("#about-close")!.addEventListener("click", () => closeModal(overlay));
}

async function openDebugWindow() {
  try {
    const existing = await WebviewWindow.getByLabel("debug");
    if (existing) {
      await existing.show();
      await existing.setFocus();
      return;
    }

    const debugWindow = new WebviewWindow("debug", {
      url: "index.html",
      title: "LMPlus Debug",
      width: 720,
      height: 560,
      minWidth: 560,
      minHeight: 420,
      center: true,
      resizable: true,
      decorations: true,
      devtools: false,
    });
    debugWindow.once("tauri://error", async (event) => {
      await showMessageDialog("Debug Window", `Failed to open debug window:\n${event.payload}`);
    });
  } catch (e) {
    await showMessageDialog("Debug Window", `Failed to open debug window:\n${e}`);
  }
}

// --- App Path dialog ---
function showAppPathDialog() {
  const content = document.createElement("div");
  content.className = "input-dialog";
  content.innerHTML = `
    <label>Base Path:</label>
    <input id="base-path-input" type="text" />
    <div id="base-path-suggestions" class="path-suggestions"></div>
    <div class="modal-buttons">
      <button id="btn-cancel">Cancel</button>
      <button id="btn-ok" class="primary">OK</button>
    </div>
  `;
  const overlay = showModal("Select Lords Mobile Base Directory", content);
  const input = content.querySelector("#base-path-input") as HTMLInputElement;
  const suggestions = content.querySelector("#base-path-suggestions") as HTMLElement;
  suggestions.hidden = true;
  input.value = basePath;

  let suggestionRequestId = 0;
  let pausedSuggestionPath: string | null = null;

  const canonicalPath = (value: string) => value.trim().replace(/[\\/]+$/, "").toLocaleLowerCase();
  const wantsChildSuggestions = (value: string) => /[\\/]$/.test(value.trim());
  const hideSuggestions = () => {
    suggestionRequestId += 1;
    suggestions.innerHTML = "";
    suggestions.hidden = true;
  };
  const pauseSuggestionsAt = (value: string) => {
    pausedSuggestionPath = canonicalPath(value);
    hideSuggestions();
  };
  const shouldPauseSuggestions = (value: string) => {
    if (!pausedSuggestionPath) return false;
    const trimmed = value.trim();
    if (canonicalPath(trimmed) === pausedSuggestionPath && !wantsChildSuggestions(trimmed)) {
      return true;
    }
    const lower = trimmed.toLocaleLowerCase();
    if (lower.startsWith(pausedSuggestionPath) && !wantsChildSuggestions(trimmed)) {
      return true;
    }
    if (!lower.startsWith(pausedSuggestionPath)) {
      pausedSuggestionPath = null;
    }
    return false;
  };

  const renderSuggestions = async () => {
    const rawValue = input.value.trim();
    if (!rawValue || shouldPauseSuggestions(rawValue)) {
      hideSuggestions();
      return;
    }
    if (wantsChildSuggestions(rawValue)) {
      pausedSuggestionPath = null;
    }
    const requestId = ++suggestionRequestId;
    try {
      const matches = await api.suggestDirectories(rawValue);
      if (requestId !== suggestionRequestId) return;
      if (matches.some((match) => canonicalPath(match) === canonicalPath(rawValue)) && !wantsChildSuggestions(rawValue)) {
        pauseSuggestionsAt(rawValue);
        return;
      }
      suggestions.innerHTML = "";
      if (matches.length === 0) {
        suggestions.hidden = true;
        return;
      }
      matches.forEach((match) => {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "path-suggestion";
        button.textContent = match;
        button.title = match;
        button.addEventListener("mousedown", (event) => {
          event.preventDefault();
          input.value = match;
          pauseSuggestionsAt(match);
          input.setSelectionRange(match.length, match.length);
        });
        suggestions.appendChild(button);
      });
      suggestions.hidden = false;
    } catch {
      hideSuggestions();
    }
  };

  input.addEventListener("input", () => {
    if (wantsChildSuggestions(input.value)) {
      pausedSuggestionPath = null;
    }
    void renderSuggestions();
  });
  input.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      hideSuggestions();
    }
    if (event.key === "Enter") {
      event.preventDefault();
      (content.querySelector("#btn-ok") as HTMLButtonElement).click();
    }
  });
  void renderSuggestions();

  content.querySelector("#btn-cancel")!.addEventListener("click", () => closeModal(overlay));
  content.querySelector("#btn-ok")!.addEventListener("click", async () => {
    const newPath = input.value.trim();
    const validationError = validateBasePathInput(newPath);
    if (validationError) {
      await showMessageDialog("App Path", validationError);
      return;
    }

    closeModal(overlay);
    try {
      await api.setBasePath(newPath);
      basePath = newPath;
      const paths = await api.getAppPaths();
      exePath = paths.exe_path;
      processName = paths.process_name;
      accountsPath = paths.accounts_path;
      await api.setSettings({ basePath: newPath });
      await api.launchGame(exePath, processName);
      await refreshAccounts();
      updateStatusBar();
    } catch (e) {
      addDebugLog("App path update failed", "warn", String(e));
      await showMessageDialog("App Path", "LMPlus could not use that app path. Choose the Lords Mobile base folder.");
    }
  });
}

// --- License verification ---
function sanitizeLicenseError(error: unknown): string {
  const raw = String(error ?? "").trim();
  const lower = raw.toLocaleLowerCase();

  if (lower.includes("banned")) {
    return "License key is banned.";
  }
  if (lower.includes("expired")) {
    return "License key is expired.";
  }
  if (
    lower.includes("device") ||
    lower.includes("fingerprint") ||
    lower.includes("machine") ||
    lower.includes("does not match") ||
    lower.includes("mismatch")
  ) {
    return "License key does not match this device.";
  }
  if (lower.includes("not found") || lower.includes("unknown key")) {
    return "License key not found.";
  }
  if (lower.includes("invalid")) {
    return "License key is invalid.";
  }
  if (
    lower.includes("request failed") ||
    lower.includes("timed out") ||
    lower.includes("timeout") ||
    lower.includes("network") ||
    lower.includes("dns")
  ) {
    return "Could not reach the license server.";
  }
  if (lower.includes("server error")) {
    return "License service is not available right now.";
  }
  if (lower.includes("signature") || lower.includes("parse json")) {
    return "License response could not be verified.";
  }

  const simple = raw.split(/\r?\n/)[0]?.trim() || "";
  if (simple && simple.length <= 90 && /^[\w .,'()/-]+$/.test(simple)) {
    return simple;
  }
  return "License verification failed.";
}

function formatGraceTime(ms: number): string {
  const totalSeconds = Math.max(0, Math.ceil(ms / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}

function clearLicenseReconnectTimer() {
  if (licenseReconnectTimer !== null) {
    window.clearInterval(licenseReconnectTimer);
    licenseReconnectTimer = null;
  }
}

function hideLicenseReconnectOverlay() {
  licenseReconnectOverlay?.remove();
  licenseReconnectOverlay = null;
  delete document.body.dataset.licenseLocked;
}

function showLicenseReconnectOverlay(status: api.LicenseHeartbeatStatus) {
  if (!licenseReconnectOverlay) {
    const overlay = document.createElement("div");
    overlay.className = "license-lock-overlay";
    overlay.innerHTML = `
      <div class="license-lock-card">
        <div class="spinner license-lock-spinner" aria-hidden="true"></div>
        <div class="license-lock-title">License reconnect required</div>
        <div class="license-lock-copy"></div>
        <div class="license-lock-status"></div>
        <button type="button" class="danger license-lock-exit">Exit</button>
      </div>
    `;
    overlay.querySelector<HTMLButtonElement>(".license-lock-exit")?.addEventListener("click", () => {
      void closeGameAndExit();
    });
    document.body.appendChild(overlay);
    licenseReconnectOverlay = overlay;
  }

  document.body.dataset.licenseLocked = "1";
  const copy = licenseReconnectOverlay.querySelector(".license-lock-copy") as HTMLElement;
  const statusLine = licenseReconnectOverlay.querySelector(".license-lock-status") as HTMLElement;
  copy.textContent = status.terminal
    ? "LMPlus needs to validate the current license state before it can continue."
    : "LMPlus is reconnecting to the license service before it can continue.";
  const pieces = [
    status.reason || "Waiting for the next license check.",
    `Attempt ${status.failure_count}`,
    `Retrying every ${Math.round(LICENSE_HEARTBEAT_RETRY_MS / 1000)}s`,
  ];
  if (status.grace_remaining_ms > 0) {
    pieces.push(`${formatGraceTime(status.grace_remaining_ms)} grace left`);
  }
  statusLine.textContent = pieces.join(" | ");
}

async function handleLicenseHeartbeatStatus(
  status: api.LicenseHeartbeatStatus,
  source: "interval" | "retry",
) {
  const normalizedReason = status.reason ? sanitizeLicenseError(status.reason) : undefined;

  if (status.status === "ok") {
    if (licenseHeartbeatLocked) {
      addDebugLog("License heartbeat restored");
    }
    licenseHeartbeatLocked = false;
    lastLicenseHeartbeatReason = "";
    clearLicenseReconnectTimer();
    hideLicenseReconnectOverlay();
    updateLicenseLabel();
    return;
  }

  const detail = {
    source,
    failureCount: status.failure_count,
    graceRemaining: status.grace_remaining_ms,
    terminal: status.terminal,
    reason: normalizedReason || "License verification failed.",
  };

  if (!status.locked) {
    addDebugLog("License heartbeat grace active", "warn", detail);
    return;
  }

  if (!licenseHeartbeatLocked || lastLicenseHeartbeatReason !== normalizedReason) {
    addDebugLog("License heartbeat locked", status.terminal ? "error" : "warn", detail);
  }
  licenseHeartbeatLocked = true;
  lastLicenseHeartbeatReason = normalizedReason || "";
  showLicenseReconnectOverlay({
    ...status,
    reason: normalizedReason,
  });
  if (licenseReconnectTimer === null) {
    licenseReconnectTimer = window.setInterval(() => {
      void performLicenseHeartbeatCheck("retry");
    }, LICENSE_HEARTBEAT_RETRY_MS);
  }
}

async function performLicenseHeartbeatCheck(source: "interval" | "retry") {
  if (!fingerprint || licenseHeartbeatRunning || closeInProgress) return;
  licenseHeartbeatRunning = true;
  try {
    const status = await api.checkLicenseHeartbeat(fingerprint);
    await handleLicenseHeartbeatStatus(status, source);
  } catch (error) {
    const reason = sanitizeLicenseError(error);
    addDebugLog("License heartbeat transport error", "warn", { source, reason });
    if (licenseHeartbeatLocked) {
      showLicenseReconnectOverlay({
        status: "locked",
        locked: true,
        terminal: false,
        failure_count: 0,
        grace_remaining_ms: 0,
        reason,
      });
    }
  } finally {
    licenseHeartbeatRunning = false;
  }
}

function startLicenseHeartbeat() {
  if (licenseHeartbeatTimer !== null) return;
  licenseHeartbeatTimer = window.setInterval(() => {
    void performLicenseHeartbeatCheck("interval");
  }, LICENSE_HEARTBEAT_INTERVAL_MS);
}

async function verifyLicense() {
  if (!document.getElementById("license-key-input")) {
    renderLicenseShell();
  }

  const input = document.getElementById("license-key-input") as HTMLInputElement;
  const login = document.getElementById("license-login") as HTMLButtonElement;
  const label = document.getElementById("license-label") as HTMLElement;
  const userLink = document.getElementById("user-link") as HTMLButtonElement | null;

  const setStatus = (message: string, tone: "normal" | "success" | "error" = "normal") => {
    label.textContent = message;
    label.dataset.tone = tone;
  };
  const setBusy = (busy: boolean) => {
    login.disabled = busy;
    input.disabled = busy;
    login.textContent = busy ? "Checking..." : "Login";
  };

  let autoLoginTimeout: number | undefined;
  let autoLoginCountdown: number | undefined;
  let checking = false;
  let settled = false;

  const clearAutoLogin = (message?: string) => {
    if (autoLoginTimeout !== undefined) {
      window.clearTimeout(autoLoginTimeout);
      autoLoginTimeout = undefined;
    }
    if (autoLoginCountdown !== undefined) {
      window.clearInterval(autoLoginCountdown);
      autoLoginCountdown = undefined;
    }
    if (message && !checking && !settled) {
      setStatus(message);
    }
  };

  const startAutoLogin = (attempt: () => void) => {
    let remaining = 3;
    setStatus(`Auto login in ${remaining}s...`);
    autoLoginCountdown = window.setInterval(() => {
      remaining -= 1;
      if (remaining > 0) {
        setStatus(`Auto login in ${remaining}s...`);
      }
    }, 1000);
    autoLoginTimeout = window.setTimeout(() => {
      clearAutoLogin();
      attempt();
    }, 3000);
  };

  return new Promise<boolean>(async (resolve) => {
    const resolveLicensed = () => {
      if (settled) return;
      settled = true;
      clearAutoLogin();
      resolve(true);
    };

    const attemptLogin = async () => {
      if (checking || settled) return;
      clearAutoLogin();
      const key = input.value.trim();
      if (!key) {
        setStatus("waiting for key", "error");
        return;
      }

      checking = true;
      setBusy(true);
      setStatus("checking...");
      try {
        const result = await api.verifyLicense(key, fingerprint);
        if (result.status !== "ok") {
          throw new Error(result.status || "License verification failed");
        }
        licenseKey = key;
        setStatus("verified", "success");
        addDebugLog("License verified");
        if (isLoginWindow) {
          resolveLicensed();
          await api.showAuthorizedMainWindow();
          await currentWindow.destroy().catch(() => undefined);
          return;
        }
        resolveLicensed();
      } catch (e) {
        const reason = sanitizeLicenseError(e);
        addDebugLog("License verification failed", "warn", reason);
        setStatus("failed", "error");
        setBusy(false);
        await showMessageDialog("License Verification", `License error:\n${reason}`);
        input.focus();
      } finally {
        checking = false;
        if (!settled) {
          setBusy(false);
        }
      }
    };

    const cancelAutoLogin = () => {
      clearAutoLogin("waiting for login");
    };

    try {
      setBusy(true);
      fingerprint = await api.getDeviceFingerprint();
      const loginSettings = await api.getLoginSettings().catch(() => ({ autoLogin: true }));
      autoLoginEnabled = loginSettings.autoLogin !== false;
      const signatureOk = await api.getConfigSignatureStatus(fingerprint).catch(() => false);
      const saved = signatureOk
        ? await api.loadSavedLicense(fingerprint).catch(() => ({ key: null }))
        : { key: null };
      licenseKey = saved.key || "";
      input.value = licenseKey;
      setBusy(false);
      setStatus(licenseKey ? "saved key loaded" : "waiting for key");
      if (!signatureOk) {
        addDebugLog("Saved license config signature missing or invalid", "warn");
      }

      userLink?.addEventListener("click", () => {
        void showMessageDialog("User", "User account linking will be added in a future update.");
      });
      input.addEventListener("pointerdown", cancelAutoLogin);
      input.addEventListener("focus", cancelAutoLogin);
      input.addEventListener("input", () => {
        licenseKey = input.value;
        clearAutoLogin("waiting for login");
      });
      input.addEventListener("keydown", (event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          void attemptLogin();
        }
      });
      login.addEventListener("click", () => {
        void attemptLogin();
      });

      if (licenseKey) {
        if (autoLoginEnabled) {
          startAutoLogin(() => {
            void attemptLogin();
          });
        } else {
          void attemptLogin();
        }
      } else {
        input.focus();
      }
    } catch (e) {
      const reason = sanitizeLicenseError(e);
      addDebugLog("License startup error", "error", reason);
      setBusy(false);
      setStatus("failed", "error");
      await showMessageDialog("License Verification", `License error:\n${reason}`);
      resolve(false);
    }
  });
}

function updateLicenseLabel() {
  const label = document.getElementById("license-label");
  if (!label) return;
  api.getLicenseInfo().then((info) => {
    label.textContent = info;
    label.style.color = "var(--success)";
  });
}

function releaseChannelTitle(value: "stable" | "beta"): string {
  return value === "beta" ? "Beta" : "Stable";
}

function updateStatusBar(count = accountCount) {
  const bar = document.getElementById("status-bar")!;
  bar.innerHTML = `
    <span class="status-copy">${count} account${count === 1 ? "" : "s"} loaded</span>
    <button id="status-refresh" type="button" class="small-pill" ${statusRefreshBusy ? "disabled" : ""}>
      ${statusRefreshBusy ? "Refreshing..." : "Refresh"}
    </button>
  `;
  bar.querySelector<HTMLButtonElement>("#status-refresh")?.addEventListener("click", () => {
    void refreshAccountButtons();
  });
}

async function checkForAppUpdate(interactive = false): Promise<boolean> {
  if (updateChoiceActive || updateInstallActive || updateCheckBusy) {
    return false;
  }
  updateCheckBusy = true;
  try {
    const update = await api.checkForUpdate();
    addDebugLog("Update check completed", "info", update);
    if (!update.update_available) {
      if (interactive) {
        await showMessageDialog("Update", `LMPlus ${update.current_version} is already up to date.`);
      }
      return false;
    }

    const choice = await showForcedUpdateChoice(update);
    if (choice === "exit") {
      await closeGameAndExit();
      return true;
    }
    await performAppUpdateWithProgress(update);
    return true;
  } catch (e) {
    addDebugLog("Update check failed", "warn", String(e));
    if (interactive) {
      await showMessageDialog("Update", sanitizeUpdateError(e));
    }
    return false;
  } finally {
    updateCheckBusy = false;
  }
}

async function renderDebugWindow() {
  const refresh = async () => {
    const stateEl = document.getElementById("debug-state")!;
    const logEl = document.getElementById("debug-log")!;

    try {
      const settings = await api.getSettings();
      const paths = await api.getAppPaths().catch(() => ({
        base_path: "",
        exe_path: "",
        process_name: "",
        accounts_path: "",
        userdata_path: "",
      }));
      const running = paths.exe_path
        ? await api.isGameRunning(paths.exe_path, paths.process_name).catch(() => false)
        : false;
      const version = await api.getVersion().catch(() => "unknown");
      const buildChannel = normalizeChannel(settings.appliedReleaseChannel, releaseChannel);
      const logs = readDebugLog();

      stateEl.innerHTML = "";
      const rows = [
        ["Version", version],
        ["Build channel", releaseChannelTitle(buildChannel)],
        ["Selected channel", releaseChannelTitle(normalizeChannel(settings.releaseChannel, buildChannel))],
        ["Debugging", settings.debugFeatures ? "Enabled" : "Disabled"],
        ["Base path", paths.base_path || settings.basePath || basePath || "(unset)"],
        ["Exe path", paths.exe_path || "(unset)"],
        ["Accounts path", paths.accounts_path || "(unset)"],
        ["Game running", running ? "Yes" : "No"],
      ];
      for (const [key, value] of rows) {
        const label = document.createElement("div");
        label.textContent = key;
        const val = document.createElement("div");
        val.textContent = value;
        stateEl.append(label, val);
      }

      logEl.innerHTML = "";
      if (logs.length === 0) {
        const empty = document.createElement("div");
        empty.className = "empty-state";
        empty.textContent = "No debug events yet.";
        logEl.appendChild(empty);
        return;
      }

      for (const entry of logs.slice().reverse()) {
        const row = document.createElement("div");
        row.className = "debug-log-row";
        const time = document.createElement("span");
        time.textContent = entry.time;
        const level = document.createElement("span");
        level.className = entry.level;
        level.textContent = entry.level.toUpperCase();
        const msg = document.createElement("span");
        msg.textContent = entry.details ? `${entry.message} — ${entry.details}` : entry.message;
        row.append(time, level, msg);
        logEl.appendChild(row);
      }
    } catch (e) {
      renderErrorState(stateEl, `Failed to load debug data: ${e}`);
    }
  };

  document.getElementById("debug-refresh")!.addEventListener("click", () => {
    void refresh();
  });
  document.getElementById("debug-clear")!.addEventListener("click", () => {
    writeDebugLog([]);
    void refresh();
  });

  await refresh();
  setInterval(() => {
    void refresh();
  }, 1500);
}

async function closeGameAndExit() {
  if (closeInProgress) return;
  closeInProgress = true;
  addDebugLog("Exit requested");
  try {
    await api.unregisterHotkeys().catch(() => undefined);
    let killExePath = exePath;
    let killProcessName = processName;
    if (!killExePath || !killProcessName) {
      const paths = await api.getAppPaths().catch(() => null);
      if (paths) {
        killExePath = paths.exe_path;
        killProcessName = paths.process_name;
      }
    }
    if (killExePath && killProcessName) {
      const killed = await api.killGame(killExePath, killProcessName).catch(() => false);
      addDebugLog("Game kill requested before exit", "info", { killed });
    }
  } finally {
    await api.exitApp().catch(async () => {
      await currentWindow.destroy();
    });
  }
}

async function setupWindowControls() {
  await currentWindow.onCloseRequested(async (event) => {
    if (closeInProgress) {
      return;
    }
    event.preventDefault();
    if (updateChoiceActive || updateInstallActive) {
      await closeGameAndExit();
      return;
    }
    if (licenseHeartbeatLocked) {
      await closeGameAndExit();
      return;
    }
    const choice = await showCloseChoiceDialog();
    if (choice === "cancel") {
      return;
    }
    if (choice === "tray") {
      try {
        await api.incrementHideToTray();
        hideCount += 1;
        await currentWindow.hide();
        addDebugLog("Window hidden to tray");
      } catch (e) {
        await showMessageDialog("LMPlus", `Failed to hide to tray:\n${e}`);
      }
      return;
    }
    await closeGameAndExit();
  });
}

let mainWindowStarted = false;

async function initAuthorizedMainWindow() {
  if (mainWindowStarted) return;
  const authorized = await api.isLicenseVerified().catch(() => false);
  if (!authorized) {
    renderLockedShell();
    return;
  }
  mainWindowStarted = true;

  try {
    if (await api.isAnotherInstanceRunning()) {
      await showMessageDialog("LMPlus", "Another instance of LMPlus is already running.");
      return;
    }
  } catch {
    // Ignore on non-Windows
  }

  try {
    fingerprint = await api.getDeviceFingerprint();
    const saved = await api.loadSavedLicense(fingerprint).catch(() => ({ key: null }));
    licenseKey = saved.key || "";
    const settings = await api.getSettings();
    basePath = settings.basePath || "";
    releaseChannel = normalizeChannel(settings.releaseChannel, releaseChannel);
    installedChannel = normalizeChannel(settings.appliedReleaseChannel, releaseChannel);
    debugFeaturesEnabled = Boolean(settings.debugFeatures);
    autoLoginEnabled = settings.autoLogin !== false;
    addDebugLog("Settings loaded", "info", {
      basePath,
      releaseChannel,
      installedChannel,
      debugFeaturesEnabled,
      autoLoginEnabled,
    });
    if (!basePath) {
      basePath = "D:\\LordsMobile - Apps\\Lords Mobile PC - Mod";
    }
    await api.setBasePath(basePath);
    const paths = await api.getAppPaths();
    exePath = paths.exe_path;
    processName = paths.process_name;
    accountsPath = paths.accounts_path;
  } catch (e) {
    console.error("Settings load error:", e);
    addDebugLog("Settings load error", "warn", String(e));
  }

  renderMainShell();
  await api.showAuthorizedMainWindow();
  const loginWindow = await WebviewWindow.getByLabel("login").catch(() => null);
  await loginWindow?.destroy().catch(() => undefined);

  try {
    if (await checkForAppUpdate(false)) return;
  } catch {
    // Ignore update errors
  }

  try {
    await api.launchGame(exePath, processName);
    launchCount += 1;
    addDebugLog("Game launch requested", "info", { exePath, processName });
  } catch (e) {
    console.error("Launch error:", e);
    addDebugLog("Launch error", "warn", String(e));
  }

  await refreshAccounts();
  await setupHotkeyEvents();
  await reregisterHotkeys();
  updateStatusBar();

  startupTime = Date.now();
  setInterval(async () => {
    try {
      totalUptime += 1800;
      await api.collectAndSendTelemetry(
        basePath, startupTime, hideCount, launchCount, totalUptime, fingerprint,
      );
    } catch {
      // Ignore
    }
  }, 1800000);

  startLicenseHeartbeat();

  if (debugFeaturesEnabled) {
    void openDebugWindow();
  }
}

async function setupLoginWindowControls() {
  await currentWindow.onCloseRequested(async (event) => {
    if (await api.isLicenseVerified().catch(() => false)) {
      return;
    }
    event.preventDefault();
    await api.exitApp().catch(() => undefined);
  });
}

function bindMainEvents() {
  document.getElementById("btn-app-path")!.addEventListener("click", showAppPathDialog);
  document.getElementById("btn-add-account")!.addEventListener("click", showAddAccountDialog);
  document.getElementById("btn-hotkeys")!.addEventListener("click", showHotkeyDialog);
  document.getElementById("btn-settings")!.addEventListener("click", showSettingsDialog);
  document.getElementById("user-link")!.addEventListener("click", () => {
    void showMessageDialog("User", "User account linking will be added in a future update.");
  });
  document.getElementById("license-key-value")!.addEventListener("click", () => {
    void copyText(licenseKey, "License key");
  });
  document.getElementById("license-key-copy")!.addEventListener("click", () => {
    void copyText(licenseKey, "License key");
  });
}

// --- Event bindings ---
if (isDebugWindow) {
  renderDebugShell();
  renderDebugWindow().catch((e) => {
    renderErrorState(document.getElementById("app")!, `Debug startup error: ${e}`);
  });
} else if (isLoginWindow) {
  renderLicenseShell();
  setupLoginWindowControls()
    .then(() => verifyLicense())
    .catch((e) => {
      renderErrorState(document.getElementById("app")!, `Login startup error: ${e}`);
    });
} else {
  renderLockedShell();
  setupWindowControls()
    .then(async () => {
      await listen("lmplus-license-ok", () => {
        void initAuthorizedMainWindow();
      });
      await currentWindow.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          void initAuthorizedMainWindow();
        }
      });
      await initAuthorizedMainWindow();
    })
    .catch((e) => {
      renderErrorState(document.getElementById("app")!, `Startup error: ${e}`);
    });
}
