const state = {
  mode: "web",
  searching: false,
  settings: {
    browser_id: null,
    private_mode: false,
    max_results: 25,
  },
  browsers: [],
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

function setMode(mode) {
  state.mode = mode;
  els.tabs.forEach((tab) => {
    tab.classList.toggle("active", tab.dataset.mode === mode);
  });
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

function renderResults(payload) {
  els.state.hidden = true;
  els.results.innerHTML = "";
  els.results.classList.remove("hidden");
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
    body.innerHTML = `
      <h3><a href="#" data-url="${encodeURIComponent(item.url)}">${escapeHtml(item.title)}</a> ${provenance}</h3>
      <span class="result-url">${escapeHtml(item.url)}</span>
      ${item.snippet ? `<p class="result-snippet">${escapeHtml(item.snippet)}</p>` : ""}
    `;
    li.appendChild(body);

    const openBtn = document.createElement("button");
    openBtn.className = "open-btn";
    openBtn.type = "button";
    openBtn.textContent = state.settings.private_mode ? "Open private" : "Open";
    openBtn.addEventListener("click", () => openLink(item.url));
    li.appendChild(openBtn);

    body.querySelector("a").addEventListener("click", (event) => {
      event.preventDefault();
      openLink(item.url);
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

async function openLink(url) {
  await persistSettings();
  const result = await api("/api/open", {
    method: "POST",
    body: JSON.stringify({
      url,
      browser_id: state.settings.browser_id,
      private_mode: state.settings.private_mode,
    }),
  });

  const modeLabel = result.mode === "private" ? " (private)" : "";
  els.state.hidden = false;
  els.state.classList.remove("error");
  els.state.innerHTML = `<h2>Opened in ${escapeHtml(result.browser)}${modeLabel}</h2><p>${escapeHtml(result.url)}</p>`;
}

async function runSearch() {
  const query = els.query.value.trim();
  if (!query || state.searching) return;

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
    els.searchBtn.disabled = false;
  }
}

async function bootstrap() {
  try {
    const [browsers, settings] = await Promise.all([
      api("/api/browsers"),
      api("/api/settings"),
    ]);
    state.browsers = browsers;
    state.settings = settings;
    els.privateMode.checked = Boolean(settings.private_mode);
    renderBrowsers();
  } catch (error) {
    showState("NetRail offline", error.message, true);
  }
}

els.form.addEventListener("submit", (event) => {
  event.preventDefault();
  runSearch();
});

els.tabs.forEach((tab) => {
  tab.addEventListener("click", () => setMode(tab.dataset.mode));
});

els.browserSelect.addEventListener("change", persistSettings);
els.privateMode.addEventListener("change", persistSettings);

bootstrap();