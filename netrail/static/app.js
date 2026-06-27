const state = {
  view: "search",
  mode: "web",
  searching: false,
  highlightIndex: -1,
  lastPayload: null,
  visibleResultCount: RESULTS_PAGE_SIZE,
  settings: {
    browser_id: null,
    private_mode: false,
    max_results: 25,
  },
  browsers: [],
  collections: [],
  saveTarget: null,
};

const BACKEND_LABELS = {
  ddgs: "DDGS",
  searxng: "SearXNG",
  brave: "Brave",
  wikipedia: "Wikipedia",
};

const DONATE_URL = "https://buymeacoffee.com/kayabsoftware";
const RESULTS_PAGE_SIZE = 10;

const els = {
  form: document.getElementById("search-form"),
  query: document.getElementById("query"),
  searchBtn: document.getElementById("search-btn"),
  exportBtn: document.getElementById("export-btn"),
  tabs: document.querySelectorAll(".tab"),
  browserSelect: document.getElementById("browser-select"),
  privateMode: document.getElementById("private-mode"),
  state: document.getElementById("state"),
  results: document.getElementById("results"),
  resultsMeta: document.getElementById("results-meta"),
  resultsCount: document.getElementById("results-count"),
  resultsBackends: document.getElementById("results-backends"),
  loadMoreBtn: document.getElementById("load-more-btn"),
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
  helpMenuBtn: document.getElementById("help-menu-btn"),
  helpDropdown: document.getElementById("help-dropdown"),
  donateBtn: document.getElementById("donate-btn"),
  docDialog: document.getElementById("doc-dialog"),
  docTitle: document.getElementById("doc-title"),
  docBody: document.getElementById("doc-body"),
  docClose: document.getElementById("doc-close"),
  securityBanner: document.getElementById("security-banner"),
};

function showSecurityBanner(message) {
  if (!els.securityBanner || !message) return;
  els.securityBanner.textContent = message;
  els.securityBanner.classList.remove("hidden");
}

function applyHealthSecurity(health) {
  const message =
    health.history?.encryption_degraded_message ||
    health.history?.encryption_warning ||
    null;
  if (message) showSecurityBanner(message);
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}));
    const detail = payload.detail || payload.message || `Request failed (${response.status})`;
    const err = new Error(detail);
    if (payload.code) err.code = payload.code;
    throw err;
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
  els.state.replaceChildren();
  const h2 = document.createElement("h2");
  h2.textContent = title;
  const p = document.createElement("p");
  p.textContent = message;
  els.state.append(h2, p);
  els.state.hidden = false;
  els.results.classList.add("hidden");
  if (els.resultsMeta) els.resultsMeta.classList.add("hidden");
  if (els.loadMoreBtn) els.loadMoreBtn.classList.add("hidden");
  state.highlightIndex = -1;
  if (els.exportBtn) els.exportBtn.disabled = true;
}

function updateSovereignty(payload) {
  const pill = document.getElementById("sovereignty-pill");
  const label = document.getElementById("sovereignty-label");
  if (!pill || !label || !payload.sovereignty) return;
  const { step, total, label: text } = payload.sovereignty;
  const strategy = payload.search_strategy === "fanout" ? " · fanout" : "";
  label.textContent = `Step ${step}/${total} · ${text.toLowerCase()}${strategy}`;
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

function backendPill(backend, provenance) {
  const label = BACKEND_LABELS[backend] || (backend || "?").toUpperCase();
  const cls = backend ? `backend-pill ${backend}` : "backend-pill";
  const title = provenance ? escapeHtml(provenance) : "";
  return `<span class="${cls}" title="${title}">[${escapeHtml(label)}]</span>`;
}

function setHighlight(index) {
  const cards = els.results.querySelectorAll(".result-card");
  if (!cards.length) {
    state.highlightIndex = -1;
    return;
  }
  const clamped = Math.max(0, Math.min(index, cards.length - 1));
  state.highlightIndex = clamped;
  cards.forEach((card, i) => {
    card.classList.toggle("highlighted", i === clamped);
    if (i === clamped) {
      card.scrollIntoView({ block: "nearest", behavior: "smooth" });
    }
  });
}

function highlightedItem() {
  if (!state.lastPayload?.results?.length) return null;
  if (state.highlightIndex < 0) return state.lastPayload.results[0];
  return state.lastPayload.results[state.highlightIndex] || null;
}

function renderResults(payload) {
  els.state.hidden = true;
  els.results.innerHTML = "";
  els.results.classList.remove("hidden");
  els.historyPanel.classList.add("hidden");
  state.lastPayload = payload;
  state.highlightIndex = payload.results.length ? 0 : -1;
  updateSovereignty(payload);

  if (els.exportBtn) {
    els.exportBtn.disabled = !payload.results.length;
  }

  let errorsEl = document.getElementById("fanout-errors");
  if (!errorsEl) {
    errorsEl = document.createElement("div");
    errorsEl.id = "fanout-errors";
    errorsEl.className = "fanout-errors hidden";
    errorsEl.setAttribute("aria-live", "polite");
    els.results.parentNode.insertBefore(errorsEl, els.results);
  }
  if (payload.errors?.length) {
    errorsEl.textContent = `Some backends failed: ${payload.errors.join("; ")}`;
    errorsEl.classList.remove("hidden");
  } else {
    errorsEl.classList.add("hidden");
    errorsEl.textContent = "";
  }

  if (!payload.results.length) {
    if (els.resultsMeta) els.resultsMeta.classList.add("hidden");
    if (els.loadMoreBtn) els.loadMoreBtn.classList.add("hidden");
    showState("No results", `Nothing came back for “${payload.query}”. Try different operators.`);
    return;
  }

  state.visibleResultCount = Math.min(RESULTS_PAGE_SIZE, payload.results.length);
  updateResultsMeta(payload, state.visibleResultCount);

  const slice = payload.results.slice(0, state.visibleResultCount);
  slice.forEach((item, index) => {
    els.results.appendChild(buildResultCard(item, index));
  });

  updateLoadMoreButton(payload.results.length, state.visibleResultCount);
}

function loadMoreResults() {
  if (!state.lastPayload?.results?.length) return;
  const total = state.lastPayload.results.length;
  const nextCount = Math.min(total, state.visibleResultCount + RESULTS_PAGE_SIZE);
  const existing = state.visibleResultCount;

  for (let index = existing; index < nextCount; index += 1) {
    els.results.appendChild(buildResultCard(state.lastPayload.results[index], index));
  }

  state.visibleResultCount = nextCount;
  updateResultsMeta(state.lastPayload, nextCount);
  updateLoadMoreButton(total, nextCount);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function resolveDisplayUrl(raw) {
  if (!raw) return "";
  let url = String(raw).trim();
  try {
    const parsed = new URL(url);
    const host = (parsed.hostname || "").toLowerCase();
    if (host === "duckduckgo.com" || host.endsWith(".duckduckgo.com") || host === "duck.com") {
      const inner = parsed.searchParams.get("uddg");
      if (inner) return resolveDisplayUrl(inner);
    }
    return parsed.href;
  } catch {
    try {
      return decodeURIComponent(url);
    } catch {
      return url;
    }
  }
}

function decodeForDisplay(value) {
  if (!value) return "";
  let text = String(value);
  for (let i = 0; i < 3; i += 1) {
    if (!/%[0-9A-Fa-f]{2}/.test(text)) break;
    try {
      const next = decodeURIComponent(text);
      if (next === text) break;
      text = next;
    } catch {
      break;
    }
  }
  return text;
}

function truncateText(text, maxLen = 88) {
  const clean = decodeForDisplay(text).replace(/\s+/g, " ").trim();
  if (clean.length <= maxLen) return clean;
  return `${clean.slice(0, maxLen - 1)}…`;
}

function formatDisplayUrl(raw, maxLen = 72) {
  const resolved = resolveDisplayUrl(raw);
  try {
    const parsed = new URL(resolved);
    const host = parsed.hostname.replace(/^www\./i, "");
    const path = decodeForDisplay(parsed.pathname + parsed.search);
    const suffix = path && path !== "/" ? path : "";
    const full = host + suffix;
    if (full.length <= maxLen) return full;
    if (suffix.length > maxLen - host.length - 4) {
      return `${host}${truncateText(suffix, maxLen - host.length)}`;
    }
    return truncateText(full, maxLen);
  } catch {
    return truncateText(resolved, maxLen);
  }
}

function formatResultTitle(item) {
  const title = decodeForDisplay(item.title || "").trim();
  const url = resolveDisplayUrl(item.url);
  if (!title || title === item.url || title === url || /%2f|%3a|uddg=/i.test(title)) {
    return formatDisplayUrl(url, 96) || "Untitled result";
  }
  return truncateText(title, 140);
}

function formatSnippet(item) {
  const snippet = decodeForDisplay(item.snippet || "").trim();
  if (snippet) return truncateText(snippet, 320);
  return "";
}

function updateResultsMeta(payload, visibleCount) {
  if (!els.resultsMeta || !els.resultsCount) return;
  const total = payload.results.length;
  if (!total) {
    els.resultsMeta.classList.add("hidden");
    return;
  }
  const shown = Math.min(visibleCount, total);
  const backends = (payload.backends_used || [])
    .map((b) => BACKEND_LABELS[b] || b)
    .join(", ");
  els.resultsCount.textContent =
    shown < total
      ? `Showing ${shown} of ${total} results for “${payload.query}”`
      : `${total} result${total === 1 ? "" : "s"} for “${payload.query}”`;
  if (els.resultsBackends) {
    els.resultsBackends.textContent = backends ? `via ${backends}` : "";
  }
  els.resultsMeta.classList.remove("hidden");
}

function updateLoadMoreButton(total, visibleCount) {
  if (!els.loadMoreBtn) return;
  if (visibleCount < total) {
    const remaining = Math.min(RESULTS_PAGE_SIZE, total - visibleCount);
    els.loadMoreBtn.textContent = `Show ${remaining} more`;
    els.loadMoreBtn.classList.remove("hidden");
  } else {
    els.loadMoreBtn.classList.add("hidden");
  }
}

function buildResultCard(item, index) {
  const li = document.createElement("li");
  li.className = `result-card${state.mode === "images" ? " image-card" : ""}${index === state.highlightIndex ? " highlighted" : ""}`;
  li.dataset.index = String(index);

  if (state.mode === "images" && item.image) {
    const img = document.createElement("img");
    img.className = "thumb";
    img.src = item.image;
    img.alt = formatResultTitle(item);
    img.loading = "lazy";
    li.appendChild(img);
  }

  const body = document.createElement("div");
  body.className = "result-body";
  const pill = backendPill(item.backend, item.provenance);
  const revisit = visitBadge(item.visit_metadata);
  const title = formatResultTitle(item);
  const displayUrl = formatDisplayUrl(item.url);
  const snippet = formatSnippet(item);
  const resolvedUrl = resolveDisplayUrl(item.url);

  body.innerHTML = `
    <h3><a href="#" data-url="${encodeURIComponent(resolvedUrl)}" title="${escapeHtml(resolvedUrl)}">${escapeHtml(title)}</a> ${pill}</h3>
    <span class="result-url" title="${escapeHtml(resolvedUrl)}">${escapeHtml(displayUrl)} ${revisit}</span>
    ${snippet ? `<p class="result-snippet">${escapeHtml(snippet)}</p>` : '<p class="result-snippet result-snippet--empty">No description available.</p>'}
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
  openBtn.addEventListener("click", () => openLink(resolvedUrl, item.result_id));
  actions.appendChild(openBtn);

  li.appendChild(actions);

  body.querySelector("a").addEventListener("click", (event) => {
    event.preventDefault();
    openLink(resolvedUrl, item.result_id);
  });

  li.addEventListener("mouseenter", () => setHighlight(index));

  return li;
}

async function openLink(url, resultId = null, forcePrivate = null) {
  await persistSettings();
  const privateMode = forcePrivate ?? state.settings.private_mode;
  const result = await api("/api/open", {
    method: "POST",
    body: JSON.stringify({
      url,
      browser_id: state.settings.browser_id,
      private_mode: privateMode,
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
  showState("Searching…", `Fanout query for “${escapeHtml(query)}”. Your machine, your request.`);

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

function exportResults(fmt = "json") {
  const payload = state.lastPayload;
  if (!payload?.results?.length) return;

  const exported = {
    query: payload.query,
    mode: payload.mode,
    search_strategy: payload.search_strategy,
    backends_used: payload.backends_used,
    exported_at: new Date().toISOString(),
    results: payload.results.map((r) => ({
      title: r.title,
      url: r.url,
      snippet: r.snippet,
      backend: r.backend,
      provenance: r.provenance,
    })),
  };

  let content;
  let mime;
  let filename;

  if (fmt === "csv") {
    const rows = ["title,url,snippet,backend"];
    for (const r of exported.results) {
      const esc = (v) => `"${String(v || "").replaceAll('"', '""')}"`;
      rows.push([esc(r.title), esc(r.url), esc(r.snippet), esc(r.backend)].join(","));
    }
    content = rows.join("\n");
    mime = "text/csv";
    filename = "netrail-results.csv";
  } else {
    content = JSON.stringify(exported, null, 2);
    mime = "application/json";
    filename = "netrail-results.json";
  }

  const blob = new Blob([content], { type: mime });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  link.click();
  URL.revokeObjectURL(link.href);
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

function handleKeyboard(event) {
  if (state.view !== "search" || !state.lastPayload?.results?.length) return;

  const active = document.activeElement;
  const queryFocused = active === els.query;
  const count = state.lastPayload.results.length;

  if (queryFocused && event.ctrlKey && event.key.toLowerCase() === "c") {
    const item = highlightedItem();
    if (item?.url) {
      event.preventDefault();
      navigator.clipboard.writeText(item.url).catch(() => {});
    }
    return;
  }

  if (queryFocused && !["ArrowDown", "ArrowUp", "Enter"].includes(event.key)) {
    return;
  }

  if (event.key === "ArrowDown") {
    event.preventDefault();
    setHighlight(state.highlightIndex < 0 ? 0 : state.highlightIndex + 1);
    return;
  }

  if (event.key === "ArrowUp") {
    event.preventDefault();
    setHighlight(state.highlightIndex <= 0 ? 0 : state.highlightIndex - 1);
    return;
  }

  if (event.key === "Enter" && state.highlightIndex >= 0) {
    event.preventDefault();
    const item = state.lastPayload.results[state.highlightIndex];
    if (!item) return;
    openLink(item.url, item.result_id, event.shiftKey ? true : null);
  }
}

function closeHelpMenu() {
  if (!els.helpDropdown) return;
  els.helpDropdown.hidden = true;
  if (els.helpMenuBtn) {
    els.helpMenuBtn.setAttribute("aria-expanded", "false");
  }
}

function toggleHelpMenu() {
  if (!els.helpDropdown || !els.helpMenuBtn) return;
  const open = els.helpDropdown.hidden;
  els.helpDropdown.hidden = !open;
  els.helpMenuBtn.setAttribute("aria-expanded", open ? "true" : "false");
}

async function openDocView(slug) {
  closeHelpMenu();
  if (!els.docDialog || !els.docTitle || !els.docBody) return;

  els.docTitle.textContent = "Loading…";
  els.docBody.innerHTML = "<p>Loading document…</p>";
  els.docDialog.showModal();

  try {
    const payload = await api(`/api/docs/${slug}`);
    els.docTitle.textContent = payload.title;
    els.docBody.innerHTML = window.renderMarkdown(payload.markdown);
  } catch (error) {
    els.docTitle.textContent = "Document unavailable";
    els.docBody.textContent = error.message;
  }
}

async function openDonate() {
  closeHelpMenu();
  try {
    await persistSettings();
    await api("/api/open", {
      method: "POST",
      body: JSON.stringify({
        url: DONATE_URL,
        private_mode: false,
      }),
    });
  } catch {
    window.open(DONATE_URL, "_blank", "noopener,noreferrer");
  }
}

function handleDocHash() {
  const match = window.location.hash.match(/^#doc=(manual|about)$/);
  if (match) {
    openDocView(match[1]);
    history.replaceState(null, "", window.location.pathname + window.location.search);
  }
}

function dismissSplash() {
  const splash = document.getElementById("splash");
  if (!splash) return;
  splash.classList.add("splash-hidden");
  splash.setAttribute("aria-hidden", "true");
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
    applyHealthSecurity(health);
    if (health.history?.queries > 0) {
      const label = document.getElementById("sovereignty-label");
      if (label) label.textContent = "Step 4/5 · local history and corpus";
    }
    await loadCollections();
  } catch (error) {
    showState("NetRail offline", error.message, true);
  } finally {
    window.setTimeout(dismissSplash, 420);
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

if (els.exportBtn) {
  els.exportBtn.addEventListener("click", (event) => {
    exportResults(event.shiftKey ? "csv" : "json");
  });
}

if (els.loadMoreBtn) {
  els.loadMoreBtn.addEventListener("click", loadMoreResults);
}

document.addEventListener("keydown", handleKeyboard);

if (els.helpMenuBtn) {
  els.helpMenuBtn.addEventListener("click", (event) => {
    event.stopPropagation();
    toggleHelpMenu();
  });
}

if (els.helpDropdown) {
  els.helpDropdown.querySelectorAll("[data-doc]").forEach((button) => {
    button.addEventListener("click", () => openDocView(button.dataset.doc));
  });
}

if (els.donateBtn) {
  els.donateBtn.addEventListener("click", openDonate);
}

if (els.docClose && els.docDialog) {
  els.docClose.addEventListener("click", () => els.docDialog.close());
}

document.addEventListener("click", (event) => {
  if (!els.helpDropdown || els.helpDropdown.hidden) return;
  if (event.target === els.helpMenuBtn || els.helpMenuBtn?.contains(event.target)) return;
  if (els.helpDropdown.contains(event.target)) return;
  closeHelpMenu();
});

window.addEventListener("hashchange", handleDocHash);
window.netrailOpenDoc = openDocView;
window.netrailDonate = openDonate;

if (window.__TAURI__?.event?.listen) {
  window.__TAURI__.event
    .listen("security:encryption-degraded", (event) => {
      showSecurityBanner(event.payload);
    })
    .catch(() => {});
}

bootstrap().then(handleDocHash);