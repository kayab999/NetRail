const state = {
  view: "search",
  mode: "web",
  searching: false,
  settings: {
    browser_id: null,
    private_mode: false,
    max_results: 25,
  },
  browsers: [],
  collections: [],
  saveTarget: null,
};

const els = {
  form: document.getElementById("search-form"),
  query: document.getElementById("query"),
  searchBtn: document.getElementById("search-btn"),
  tabs: document.querySelectorAll(".tab"),
  browserSelect: document.getElementById("browser-select"),
  privateMode: document.getElementById("private-mode"),
  state: document.getElementById("state"),
  results: document.getElementById("results"),
  historyPanel: document.getElementById("history-panel"),
  historyList: document.getElementById("history-list"),
  historyQuery: document.getElementById("history-query"),
  historySearchBtn: document.getElementById("history-search-btn"),
  historyPurgeBtn: document.getElementById("history-purge-btn"),
  operatorHint: document.getElementById("operator-hint"),
  saveDialog: document.getElementById("save-dialog"),
  saveForm: document.getElementById("save-form"),
  saveCollectionSelect: document.getElementById("save-collection-select"),
  saveNewCollection: document.getElementById("save-new-collection"),
  saveNotes: document.getElementById("save-notes"),
  saveTargetUrl: document.getElementById("save-target-url"),
  saveCancel: document.getElementById("save-cancel"),
};

async function api(path, options = {}) {
  const response = await fetch(path, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}));
    throw new Error(payload.detail || `Request failed (${response.status})`);
  }
  return response.json();
}

function setView(view, mode) {
  state.view = view;
  if (mode) state.mode = mode;

  els.tabs.forEach((tab) => {
    const tabView = tab.dataset.view;
    const tabMode = tab.dataset.mode;
    const active =
      tabView === view && (view !== "search" || tabMode === state.mode);
    tab.classList.toggle("active", active);
  });

  const isHistory = view === "history";
  els.operatorHint.hidden = isHistory;
  els.query.placeholder = isHistory
    ? "Switch to Web/Images to search the net"
    : 'Search the web — try site:gov filetype:pdf "exact phrase"';
  els.searchBtn.disabled = isHistory;
  els.query.disabled = isHistory;

  if (isHistory) {
    els.state.hidden = true;
    els.results.classList.add("hidden");
    els.historyPanel.classList.remove("hidden");
    loadHistory();
  } else {
    els.historyPanel.classList.add("hidden");
  }
}

function renderBrowsers() {
  els.browserSelect.innerHTML = "";
  if (!state.browsers.length) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "No browser detected";
    els.browserSelect.appendChild(option);
    return;
  }

  for (const browser of state.browsers) {
    const option = document.createElement("option");
    option.value = browser.id;
    option.textContent = browser.name;
    els.browserSelect.appendChild(option);
  }

  const selected =
    state.settings.browser_id &&
    state.browsers.some((b) => b.id === state.settings.browser_id)
      ? state.settings.browser_id
      : state.browsers[0].id;

  els.browserSelect.value = selected;
  state.settings.browser_id = selected;
}

async function persistSettings() {
  state.settings.browser_id = els.browserSelect.value || null;
  state.settings.private_mode = els.privateMode.checked;
  state.settings = await api("/api/settings", {
    method: "PUT",
    body: JSON.stringify(state.settings),
  });
}

function showState(title, message, isError = false) {
  els.state.classList.toggle("error", isError);
  els.state.innerHTML = `<h2>${title}</h2><p>${message}</p>`;
  els.state.hidden = false;
  els.results.classList.add("hidden");
}

function updateSovereignty(payload) {
  const pill = document.getElementById("sovereignty-pill");
  const label = document.getElementById("sovereignty-label");
  if (!pill || !label || !payload.sovereignty) return;
  const { step, total, label: text } = payload.sovereignty;
  label.textContent = `Step ${step}/${total} · ${text.toLowerCase()}`;
  pill.title = (payload.provenance_chain || []).join("\n");
}

function visitBadge(meta) {
  if (!meta || !meta.last_visited) return "";
  const last = new Date(meta.last_visited.replace(" ", "T"));
  const diffMs = Date.now() - last.getTime();
  const days = Math.floor(diffMs / 86400000);
  let ago = "today";
  if (days === 1) ago = "1d ago";
  else if (days > 1) ago = `${days}d ago`;
  const count = meta.visit_count > 1 ? ` · ${meta.visit_count}×` : "";
  return `<span class="revisit-badge">visited ${ago}${count}</span>`;
}

function renderResults(payload) {
  els.state.hidden = true;
  els.results.innerHTML = "";
  els.results.classList.remove("hidden");
  els.historyPanel.classList.add("hidden");
  updateSovereignty(payload);

  if (!payload.results.length) {
    showState("No results", `Nothing came back for “${payload.query}”. Try different operators.`);
    return;
  }

  for (const item of payload.results) {
    const li = document.createElement("li");
    li.className = `result-card${state.mode === "images" ? " image-card" : ""}`;

    if (state.mode === "images" && item.image) {
      const img = document.createElement("img");
      img.className = "thumb";
      img.src = item.image;
      img.alt = item.title;
      img.loading = "lazy";
      li.appendChild(img);
    }

    const body = document.createElement("div");
    body.className = "result-body";
    const provenance = item.provenance
      ? `<span class="provenance-badge" title="${escapeHtml(item.provenance)}">via ${escapeHtml(item.backend || "unknown")}</span>`
      : "";
    const revisit = visitBadge(item.visit_metadata);
    body.innerHTML = `
      <h3><a href="#" data-url="${encodeURIComponent(item.url)}">${escapeHtml(item.title)}</a> ${provenance}</h3>
      <span class="result-url">${escapeHtml(item.url)} ${revisit}</span>
      ${item.snippet ? `<p class="result-snippet">${escapeHtml(item.snippet)}</p>` : ""}
    `;
    li.appendChild(body);

    const actions = document.createElement("div");
    actions.className = "result-actions";

    const saveBtn = document.createElement("button");
    saveBtn.className = "icon-btn";
    saveBtn.type = "button";
    saveBtn.title = "Save to collection";
    saveBtn.textContent = "★";
    saveBtn.addEventListener("click", () => openSaveDialog(item));
    actions.appendChild(saveBtn);

    const openBtn = document.createElement("button");
    openBtn.className = "open-btn";
    openBtn.type = "button";
    openBtn.textContent = state.settings.private_mode ? "Open private" : "Open";
    openBtn.addEventListener("click", () => openLink(item.url, item.result_id));
    actions.appendChild(openBtn);

    li.appendChild(actions);

    body.querySelector("a").addEventListener("click", (event) => {
      event.preventDefault();
      openLink(item.url, item.result_id);
    });

    els.results.appendChild(li);
  }
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

async function openLink(url, resultId = null) {
  await persistSettings();
  const result = await api("/api/open", {
    method: "POST",
    body: JSON.stringify({
      url,
      browser_id: state.settings.browser_id,
      private_mode: state.settings.private_mode,
      result_id: resultId,
    }),
  });

  const modeLabel = result.mode === "private" ? " (private)" : "";
  els.state.hidden = false;
  els.state.classList.remove("error");
  els.state.innerHTML = `<h2>Opened in ${escapeHtml(result.browser)}${modeLabel}</h2><p>${escapeHtml(result.url)}</p>`;
}

async function runSearch() {
  const query = els.query.value.trim();
  if (!query || state.searching || state.view !== "search") return;

  state.searching = true;
  els.searchBtn.disabled = true;
  showState("Searching…", `Querying the web for “${escapeHtml(query)}”. Your machine, your request.`);

  try {
    await persistSettings();
    const payload = await api("/api/search", {
      method: "POST",
      body: JSON.stringify({
        query,
        mode: state.mode,
        max_results: state.settings.max_results,
      }),
    });
    renderResults(payload);
  } catch (error) {
    showState("Search failed", error.message, true);
  } finally {
    state.searching = false;
    els.searchBtn.disabled = state.view === "history";
  }
}

async function loadCollections() {
  try {
    state.collections = await api("/api/collections");
  } catch {
    state.collections = [];
  }
  els.saveCollectionSelect.innerHTML = "";
  if (!state.collections.length) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "— create new below —";
    els.saveCollectionSelect.appendChild(option);
    return;
  }
  for (const collection of state.collections) {
    const option = document.createElement("option");
    option.value = String(collection.id);
    option.textContent = `${collection.name} (${collection.item_count})`;
    els.saveCollectionSelect.appendChild(option);
  }
}

function openSaveDialog(item) {
  state.saveTarget = item;
  els.saveTargetUrl.textContent = item.url;
  els.saveNewCollection.value = "";
  els.saveNotes.value = "";
  loadCollections().then(() => els.saveDialog.showModal());
}

async function saveToCollection(event) {
  event.preventDefault();
  const item = state.saveTarget;
  if (!item) return;

  let collectionId = els.saveCollectionSelect.value;
  const newName = els.saveNewCollection.value.trim();

  if (newName) {
    const created = await api("/api/collections", {
      method: "POST",
      body: JSON.stringify({ name: newName }),
    });
    collectionId = String(created.id);
  }

  if (!collectionId) {
    alert("Choose or create a collection.");
    return;
  }

  await api(`/api/collections/${collectionId}/items`, {
    method: "POST",
    body: JSON.stringify({
      url: item.url,
      title: item.title,
      notes: els.saveNotes.value.trim() || null,
    }),
  });

  els.saveDialog.close();
  await loadCollections();
}

async function loadHistory() {
  const q = els.historyQuery.value.trim();
  const path = q
    ? `/api/history?q=${encodeURIComponent(q)}&limit=100`
    : "/api/history?limit=100";

  try {
    const payload = await api(path);
    els.historyList.innerHTML = "";
    if (!payload.items.length) {
      els.historyList.innerHTML = "<li class='history-empty'>No local history yet.</li>";
      return;
    }
    for (const entry of payload.items) {
      const li = document.createElement("li");
      li.className = "history-item";
      li.innerHTML = `
        <div>
          <strong>${escapeHtml(entry.query)}</strong>
          <span class="history-meta">${escapeHtml(entry.mode)} · ${entry.result_count} results · ${escapeHtml(entry.timestamp)}</span>
        </div>
        <div class="history-actions">
          <button type="button" class="ghost-btn rerun-btn">Re-run</button>
          <button type="button" class="ghost-btn danger delete-btn">Delete</button>
        </div>
      `;
      li.querySelector(".rerun-btn").addEventListener("click", () => {
        els.query.value = entry.query;
        setView("search", entry.mode);
        runSearch();
      });
      li.querySelector(".delete-btn").addEventListener("click", async () => {
        await api(`/api/history/${entry.id}`, { method: "DELETE" });
        loadHistory();
      });
      els.historyList.appendChild(li);
    }
  } catch (error) {
    els.historyList.innerHTML = `<li class='history-empty'>${escapeHtml(error.message)}</li>`;
  }
}

async function purgeHistory() {
  if (!confirm("Delete all local search history?")) return;
  await api("/api/history", { method: "DELETE" });
  loadHistory();
}

async function bootstrap() {
  try {
    const [browsers, settings, health] = await Promise.all([
      api("/api/browsers"),
      api("/api/settings"),
      api("/api/health"),
    ]);
    state.browsers = browsers;
    state.settings = settings;
    els.privateMode.checked = Boolean(settings.private_mode);
    renderBrowsers();
    if (health.history?.queries > 0) {
      const label = document.getElementById("sovereignty-label");
      if (label) label.textContent = "Step 4/5 · local history and corpus";
    }
    await loadCollections();
  } catch (error) {
    showState("NetRail offline", error.message, true);
  }
}

els.form.addEventListener("submit", (event) => {
  event.preventDefault();
  runSearch();
});

els.tabs.forEach((tab) => {
  tab.addEventListener("click", () => {
    const view = tab.dataset.view;
    const mode = tab.dataset.mode || state.mode;
    setView(view, mode);
  });
});

els.browserSelect.addEventListener("change", persistSettings);
els.privateMode.addEventListener("change", persistSettings);
els.historySearchBtn.addEventListener("click", loadHistory);
els.historyPurgeBtn.addEventListener("click", purgeHistory);
els.saveForm.addEventListener("submit", saveToCollection);
els.saveCancel.addEventListener("click", () => els.saveDialog.close());

bootstrap();