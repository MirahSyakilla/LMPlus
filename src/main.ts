import * as api from "./api";

const app = document.getElementById("app")!;
app.innerHTML = `
  <div id="app-container">
    <div id="toolbar">
      <button id="btn-app-path">App Path</button>
      <button id="btn-add-account">Add Account</button>
      <button id="btn-hotkeys">Hotkeys</button>
      <button id="btn-settings">Settings</button>
      <span id="license-label">License: (Checking...)</span>
    </div>
    <div id="account-list"></div>
    <div id="status-bar"></div>
  </div>
`;

const style = document.createElement("style");
style.textContent = `
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { font-family: "Segoe UI", Tahoma, sans-serif; font-size: 13px; background: #f0f0f0; }
  #app-container { display: flex; flex-direction: column; height: 100vh; }
  #toolbar { 
    display: flex; align-items: center; gap: 4px; padding: 4px 6px; 
    background: #e8e8e8; border-bottom: 1px solid #ccc; flex-wrap: wrap;
  }
  #toolbar button {
    padding: 3px 8px; border: 1px solid #999; border-radius: 3px;
    background: #fff; cursor: pointer; font-size: 12px;
  }
  #toolbar button:hover { background: #d0d0d0; }
  #license-label { font-size: 10px; color: green; font-weight: bold; margin-left: auto; }
  #account-list { 
    flex: 1; overflow-y: auto; padding: 6px;
    display: flex; flex-direction: column; gap: 4px;
  }
  .account-btn {
    width: 100%; padding: 8px; border: 1px solid #bbb; border-radius: 4px;
    background: #fff; cursor: pointer; text-align: left; font-size: 13px;
  }
  .account-btn:hover { background: #e0e8f0; }
  #status-bar { padding: 4px 8px; font-size: 11px; color: #666; background: #e8e8e8; border-top: 1px solid #ccc; }
  
  /* Modal styles */
  .modal-overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 1000;
  }
  .modal {
    background: #fff; border-radius: 6px; padding: 16px; min-width: 320px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.2);
  }
  .modal h3 { margin-bottom: 12px; font-size: 14px; }
  .modal input { width: 100%; padding: 6px; border: 1px solid #ccc; border-radius: 3px; margin-bottom: 8px; }
  .modal-buttons { display: flex; gap: 6px; justify-content: flex-end; }
  .modal-buttons button { padding: 4px 12px; border: 1px solid #999; border-radius: 3px; cursor: pointer; }
  .modal-buttons button.primary { background: #0078d7; color: #fff; border-color: #0078d7; }
  .modal-buttons button:hover { opacity: 0.85; }
  
  /* Hotkey dialog */
  .hotkey-tabs { display: flex; gap: 2px; margin-bottom: 8px; }
  .hotkey-tab { padding: 4px 12px; border: 1px solid #999; border-radius: 3px 3px 0 0; background: #e0e0e0; cursor: pointer; }
  .hotkey-tab.active { background: #fff; border-bottom-color: #fff; font-weight: bold; }
  .hotkey-table { width: 100%; border-collapse: collapse; margin-bottom: 8px; }
  .hotkey-table th { background: #f0f0f0; padding: 4px 8px; text-align: left; border: 1px solid #ccc; font-size: 12px; }
  .hotkey-table td { padding: 4px 8px; border: 1px solid #ccc; font-size: 12px; }
  .hotkey-table td.hotkey-cell { cursor: pointer; }
  .hotkey-table td.hotkey-cell:hover { background: #e8f0e8; }
  .hotkey-table td.hotkey-cell.recording { background: #90ee90; }
  
  /* Settings dialog */
  .settings-sort { border: 1px solid #ccc; border-radius: 3px; min-height: 100px; max-height: 200px; overflow-y: auto; }
  .settings-sort-item { padding: 4px 8px; cursor: grab; border-bottom: 1px solid #eee; }
  .settings-sort-item:hover { background: #e8f0e8; }
  
  /* Loading */
  .loading { display: flex; align-items: center; justify-content: center; padding: 20px; }
  .spinner { width: 40px; height: 40px; border: 4px solid #ccc; border-top-color: #0078d7; border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  
  .context-menu {
    position: fixed; background: #fff; border: 1px solid #ccc; border-radius: 4px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.15); z-index: 2000; min-width: 140px; padding: 4px 0;
  }
  .context-menu-item {
    padding: 4px 12px; cursor: pointer; font-size: 12px;
  }
  .context-menu-item:hover { background: #0078d7; color: #fff; }
`;

document.head.appendChild(style);

// --- State ---
let basePath = "";
let exePath = "";
let processName = "";
let accountsPath = "";
let fingerprint = "";
let startupTime = Date.now();

// --- Context menu ---
let contextMenu: HTMLElement | null = null;
function showContextMenu(x: number, y: number, items: { label: string; action: () => void }[]) {
  hideContextMenu();
  const menu = document.createElement("div");
  menu.className = "context-menu";
  menu.style.left = `${x}px`;
  menu.style.top = `${y}px`;
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
function showModal(title: string, content: HTMLElement, width?: string): HTMLElement {
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
    if (e.target === overlay) overlay.remove();
  });
  document.body.appendChild(overlay);
  return overlay;
}

function closeModal(overlay: HTMLElement) {
  overlay.remove();
}

// --- Account list ---
async function refreshAccounts() {
  const list = document.getElementById("account-list")!;
  list.innerHTML = "";

  if (!accountsPath) return;

  try {
    const accounts = await api.loadAccounts(basePath);
    if (accounts.length === 0) {
      list.innerHTML = '<div style="padding:20px; text-align:center; color:#888;">Account folder is empty. Add userdata folders with the account name!</div>';
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
  } catch (e) {
    list.innerHTML = `<div style="padding:20px; color:red;">Error: ${e}</div>`;
  }
}

async function switchAccount(name: string) {
  try {
    await api.restartGame(exePath, processName, false);
    await refreshAccounts();
    await api.restartGame(exePath, processName, true);
  } catch (e) {
    alert(`Failed to switch account: ${e}`);
  }
}

async function renameAccount(oldName: string) {
  const newName = prompt("Enter new account name:", oldName);
  if (newName && newName !== oldName) {
    try {
      // Rename the directory via backend
      // We need to add this command... but for now use a workaround
      alert("Rename not yet implemented in Rust backend. Please rename the folder manually.");
      await refreshAccounts();
    } catch (e) {
      alert(`Failed to rename: ${e}`);
    }
  }
}

async function deleteAccount(name: string) {
  if (confirm(`Confirm delete '${name}'?`)) {
    try {
      alert("Delete not yet implemented in Rust backend. Please delete the folder manually.");
      await refreshAccounts();
    } catch (e) {
      alert(`Failed to delete: ${e}`);
    }
  }
}

// --- Add Account dialog ---
function showAddAccountDialog() {
  const content = document.createElement("div");
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
    if (!name) {
      alert("Account name cannot be empty.");
      return;
    }
    closeModal(overlay);
    try {
      // The original code copies userdata -> Accounts/<name>
      // We need backend support for this
      alert("Add account will copy userdata. Not yet implemented in Rust backend.");
      await refreshAccounts();
    } catch (e) {
      alert(`Error: ${e}`);
    }
  });
}

// --- Hotkey dialog ---
let currentHotkeyTab = 0;
let recordingRow = -1;
let recordingTable = 0;

async function showHotkeyDialog() {
  const content = document.createElement("div");

  const tabs = ["Account", "Formation", "Misc"];
  const tabBar = document.createElement("div");
  tabBar.className = "hotkey-tabs";
  tabs.forEach((t, i) => {
    const tab = document.createElement("div");
    tab.className = `hotkey-tab${i === currentHotkeyTab ? " active" : ""}`;
    tab.textContent = t;
    tab.addEventListener("click", () => {
      currentHotkeyTab = i;
      showHotkeyDialog();
      closeModal(document.querySelector(".modal-overlay")!);
    });
    tabBar.appendChild(tab);
  });
  content.appendChild(tabBar);

  const table = document.createElement("table");
  table.className = "hotkey-table";
  table.innerHTML = "<thead><tr><th>Name</th><th>Hotkey</th></tr></thead><tbody></tbody>";
  content.appendChild(table);

  const tbody = table.querySelector("tbody")!;

  if (currentHotkeyTab === 0) {
    try {
      const accounts = await api.loadAccounts(basePath);
      const hotkeys = await api.getHotkeySettings();
      for (const acc of accounts) {
        const tr = document.createElement("tr");
        tr.innerHTML = `<td>${acc}</td><td class="hotkey-cell">${hotkeys[`account.${acc}`] || ""}</td>`;
        tr.querySelector(".hotkey-cell")!.addEventListener("click", () => startRecording(0, accounts.indexOf(acc), tr));
        tbody.appendChild(tr);
      }
    } catch (e) {
      tbody.innerHTML = `<tr><td colspan="2">Error: ${e}</td></tr>`;
    }
  } else if (currentHotkeyTab === 1) {
    const formations = ["Inf Phal", "Range Phal", "Cav Phal", "Inf Wedge", "Range Wedge", "Cav Wedge"];
    const hotkeys = await api.getHotkeySettings();
    for (const f of formations) {
      const tr = document.createElement("tr");
      tr.innerHTML = `<td>${f}</td><td class="hotkey-cell">${hotkeys[`swap.${f}`] || ""}</td>`;
      const idx = formations.indexOf(f);
      tr.querySelector(".hotkey-cell")!.addEventListener("click", () => startRecording(1, idx, tr));
      tbody.appendChild(tr);
    }
  } else if (currentHotkeyTab === 2) {
    try {
      const macros = await api.loadMiscCfg();
      for (const m of macros) {
        const tr = document.createElement("tr");
        tr.innerHTML = `<td>${m.name}</td><td class="hotkey-cell">${m.hotkey || ""}</td>`;
        const idx = macros.indexOf(m);
        tr.querySelector(".hotkey-cell")!.addEventListener("click", () => startRecording(2, idx, tr));
        tbody.appendChild(tr);
      }
    } catch (e) {
      tbody.innerHTML = `<tr><td colspan="2">Error: ${e}</td></tr>`;
    }
  }

  const btnBar = document.createElement("div");
  btnBar.className = "modal-buttons";
  const closeBtn = document.createElement("button");
  closeBtn.textContent = "Close";
  closeBtn.addEventListener("click", () => closeModal(overlay));
  btnBar.appendChild(closeBtn);
  content.appendChild(btnBar);

  const overlay = showModal("Hotkey Assignments", content, "420px");
}

function startRecording(table: number, row: number, tr: HTMLElement) {
  recordingRow = row;
  recordingTable = table;
  const cells = tr.querySelectorAll("td");
  cells.forEach((c) => c.classList.remove("recording"));
  cells[1].classList.add("recording");
  cells[1].textContent = "Press key...";

  const handler = (e: KeyboardEvent) => {
    e.preventDefault();
    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Win");

    let keyName = e.key;
    if (keyName === "Control" || keyName === "Alt" || keyName === "Shift" || keyName === "Meta") {
      return;
    }
    if (keyName === " ") {
      alert("Space cannot be used as a hotkey.");
      return;
    }
    if (keyName.length === 1) {
      keyName = keyName.toUpperCase();
    }
    parts.push(keyName);

    const hotkey = parts.join("+");
    cells[1].textContent = hotkey;
    cells[1].classList.remove("recording");
    recordingRow = -1;
    document.removeEventListener("keydown", handler);
  };
  document.addEventListener("keydown", handler);
}

// --- Settings dialog ---
async function showSettingsDialog() {
  const content = document.createElement("div");
  content.innerHTML = `
    <div style="display:flex; gap:4px; margin-bottom:8px; flex-wrap:wrap;">
      <button id="settings-sort" class="primary">Sortings</button>
      <button id="settings-refresh">Refresh Buttons</button>
      <button id="settings-map-zoom">Map Zoom</button>
      <button id="settings-update-lm">Update LM</button>
      <button id="settings-about">About</button>
    </div>
    <div class="settings-sort" id="sort-list"></div>
    <div style="margin-top: 8px;">
      <button id="settings-reset">Reset Sortings</button>
    </div>
    <div style="margin-top: 8px; font-size: 11px; color: gray;">Drag and drop to reorder accounts.</div>
  `;

  const overlay = showModal("Settings", content, "420px");

  const sortList = content.querySelector("#sort-list")!;
  try {
    const accounts = await api.loadAccounts(basePath);
    for (const acc of accounts) {
      const item = document.createElement("div");
      item.className = "settings-sort-item";
      item.textContent = acc;
      item.draggable = true;
      sortList.appendChild(item);
    }
  } catch (e) {
    sortList.innerHTML = `<div>Error: ${e}</div>`;
  }

  content.querySelector("#settings-sort")!.addEventListener("click", async () => {
    const items = sortList.querySelectorAll(".settings-sort-item");
    const order = Array.from(items).map((el) => el.textContent!);
    await api.setAccountOrder(order);
    await refreshAccounts();
    alert("Order saved!");
  });

  content.querySelector("#settings-reset")!.addEventListener("click", async () => {
    await api.setAccountOrder([]);
    await refreshAccounts();
    closeModal(overlay);
    showSettingsDialog();
  });

  content.querySelector("#settings-refresh")!.addEventListener("click", async () => {
    await refreshAccounts();
    alert("Account list refreshed.");
  });

  content.querySelector("#settings-map-zoom")!.addEventListener("click", async () => {
    if (confirm("Ensure 'Kingdom 3D Map' is set to Balanced before pressing Ok!\n\nMake sure to be in map before pressing Ok!")) {
      const persistent = confirm("Enable Persistent Map Zoom?");
      try {
        await api.performMapZoom(exePath, persistent);
        alert("Map zoom applied! This will reset if you change the 'Kingdom 3D Map' setting.");
      } catch (e) {
        alert(`Error: ${e}`);
      }
    }
  });

  content.querySelector("#settings-update-lm")!.addEventListener("click", async () => {
    try {
      await api.restartGame(exePath, processName, false);
      alert("Game stopped. Please run the updater manually.");
    } catch (e) {
      alert(`Error: ${e}`);
    }
  });

  content.querySelector("#settings-about")!.addEventListener("click", async () => {
    const ver = await api.getVersion();
    alert(`LMPlus\nVersion: ${ver}\nDeveloper: Meow\nContact: @.meow._\n\nCopyrighted by NbP`);
  });
}

// --- App Path dialog ---
function showAppPathDialog() {
  const content = document.createElement("div");
  content.innerHTML = `
    <label>Base Path:</label>
    <input id="base-path-input" type="text" value="${basePath}" />
    <div class="modal-buttons">
      <button id="btn-cancel">Cancel</button>
      <button id="btn-ok" class="primary">OK</button>
    </div>
  `;
  const overlay = showModal("Select Lords Mobile Base Directory", content);

  content.querySelector("#btn-cancel")!.addEventListener("click", () => closeModal(overlay));
  content.querySelector("#btn-ok")!.addEventListener("click", async () => {
    const newPath = (content.querySelector("#base-path-input") as HTMLInputElement).value.trim();
    if (newPath) {
      closeModal(overlay);
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
    }
  });
}

// --- License verification ---
async function verifyLicense() {
  try {
    fingerprint = await api.getDeviceFingerprint();
    const saved = await api.loadSavedLicense(fingerprint);
    if (saved.key) {
      const result = await api.verifyLicense(saved.key, fingerprint);
      if (result.status === "ok") {
        await api.setLicenseInfo(result.license_info || "Unknown");
        updateLicenseLabel();
        return true;
      }
    }

    // Show license input
    const key = prompt("Enter license key:");
    if (!key) return false;

    const result = await api.verifyLicense(key, fingerprint);
    if (result.status === "ok") {
      await api.setLicenseInfo(result.license_info || "Unknown");
      updateLicenseLabel();
      return true;
    } else {
      alert("License verification failed.");
      return false;
    }
  } catch (e) {
    alert(`License error: ${e}`);
    return false;
  }
}

function updateLicenseLabel() {
  const label = document.getElementById("license-label")!;
  api.getLicenseInfo().then((info) => {
    label.textContent = `License: ${info}`;
    label.style.color = "green";
  });
}

function updateStatusBar() {
  const bar = document.getElementById("status-bar")!;
  bar.textContent = `Base: ${basePath} | Exe: ${exePath}`;
}

// --- Init ---
async function init() {
  // Check for another instance
  try {
    if (await api.isAnotherInstanceRunning()) {
      alert("Another instance of LMPlus is already running!");
      return;
    }
  } catch (e) {
    // Ignore on non-Windows
  }

  // Check for updates
  try {
    const update = await api.checkForUpdate();
    if (update.update_available) {
      if (confirm(`New version ${update.latest_version} available (current: ${update.current_version}). Update now?`)) {
        await api.performUpdate(update.zip_url);
        return;
      }
    }
  } catch (e) {
    // Ignore update errors
  }

  // License verification
  const licensed = await verifyLicense();
  if (!licensed) return;

  // Load settings
  try {
    const settings = await api.getSettings();
    basePath = settings.basePath || "";
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
  }

  // Launch game
  try {
    await api.launchGame(exePath, processName);
  } catch (e) {
    console.error("Launch error:", e);
  }

  // Refresh UI
  await refreshAccounts();
  updateStatusBar();

  // Start telemetry timer
  startupTime = Date.now();
  let launchCount = 0;
  let hideCount = 0;
  let totalUptime = 0;

  setInterval(async () => {
    try {
      await api.collectAndSendTelemetry(
        basePath, startupTime, hideCount, launchCount, totalUptime, fingerprint,
      );
    } catch (e) {
      // Ignore
    }
  }, 1800000);

  // Periodic license check
  setInterval(async () => {
    try {
      const saved = await api.loadSavedLicense(fingerprint);
      if (saved.key) {
        await api.verifyLicense(saved.key, fingerprint);
      }
    } catch (e) {
      // Ignore
    }
  }, 5 * 60 * 1000);
}

// --- Event bindings ---
document.getElementById("btn-app-path")!.addEventListener("click", showAppPathDialog);
document.getElementById("btn-add-account")!.addEventListener("click", showAddAccountDialog);
document.getElementById("btn-hotkeys")!.addEventListener("click", showHotkeyDialog);
document.getElementById("btn-settings")!.addEventListener("click", showSettingsDialog);

// --- Start ---
init().catch((e) => {
  document.getElementById("app")!.innerHTML = `<div style="padding: 20px; color: red;">Startup error: ${e}</div>`;
});