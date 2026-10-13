(() => {
  'use strict';

  const STORAGE_KEY = 'nmbs-vertrekbord-v1';
  const BOARD_W = 1280;
  const BOARD_H = 720;
  const HEADER_H = 76;
  const BASE_ROW_H = (BOARD_H - HEADER_H) / 8;

  const DEFAULT_COLORS = {
    header: '#1560bd',
    rowA: '#12356a',
    rowB: '#163b73',
    separator: '#2c5a95',
    dest: '#ffd400',
    text: '#ffffff',
    alert: '#e2231a',
    info: '#1673d6',
    newTimeBg: '#ffffff',
    newTimeText: '#0b2550',
    platformChange: '#ffd400',
  };

  const DEFAULT_SETTINGS = {
    title: '',
    language: 'nl',
    cycleInterval: 8,
    rowsVisible: 8,
    showLogo: true,
    showSeconds: false,
    blinkColon: false,
    marquee: true,
    showViaWord: true,
    alternateRows: false,
    showEmptyRows: true,
    clockMode: 'live',
    clockOffset: 0,
    customStart: '17:20:00',
    clockSpeed: 1,
    autoDepart: false,
    departGrace: 1,
    autoFill: false,
    autoSort: false,
    autoArriving: 0,
    colors: { ...DEFAULT_COLORS },
  };

  const DEFAULT_TRAINS = [
    { time: '17:17', dest: 'Quévry', via: 'Halle, ‘s-Gravenbrakel, Bergen', platform: '19', type: 'P', delay: 6 },
    { time: '17:17', dest: 'Gent-Sint-Pieters', via: 'Denderleeuw, Aalst, Wetteren', platform: '7', type: 'IC' },
    { time: '17:17', dest: 'Antwerpen-Centraal', via: 'Mechelen', platform: '13', type: 'IC', delay: 90, expected: '18:55' },
    { time: '17:25', dest: 'Louvain-la-Neuve', via: 'Brussel-Schuman, Etterbeek', platform: '18', type: 'S8', status: 'arriving' },
    { time: '17:25', dest: 'Dortmund Hbf', via: '', platform: '3', type: 'ICE', icon: 'int' },
    { time: '17:25', dest: 'Binche', via: 'Halle, ‘s-Gravenbrakel, La Louvière-Zuid', platform: '21', type: 'IC' },
    { time: '17:25', dest: 'Oostende', via: '', remark: 'Rijdt niet verder dan Brugge', platform: '15', type: 'IC', status: 'limited' },
    { time: '17:33', dest: 'Nijvel', via: 'Sint-Genesius-Rode, Waterloo, Eigenbrakel', platform: '4', type: 'S1' },
  ];

  const ROUTES = [
    { dest: 'Quévry', via: 'Halle, ‘s-Gravenbrakel, Bergen', types: ['P', 'IC'] },
    { dest: 'Gent-Sint-Pieters', via: 'Denderleeuw, Aalst, Wetteren', types: ['IC'] },
    { dest: 'Antwerpen-Centraal', via: 'Mechelen', types: ['IC'] },
    { dest: 'Louvain-la-Neuve', via: 'Brussel-Schuman, Etterbeek', types: ['S8'] },
    { dest: 'Dortmund Hbf', via: '', types: ['ICE'], icon: 'int' },
    { dest: 'Frankfurt (Main) Hbf', via: 'Liège-Guillemins, Aachen Hbf, Köln Hbf', types: ['ICE'], icon: 'int' },
    { dest: 'Paris-Nord', via: '', types: ['EUR'], icon: 'int' },
    { dest: 'Amsterdam-Centraal', via: 'Antwerpen-Centraal, Rotterdam Centraal', types: ['EC'], icon: 'int' },
    { dest: 'Binche', via: 'Halle, ‘s-Gravenbrakel, La Louvière-Zuid', types: ['IC'] },
    { dest: 'Oostende', via: 'Gent-Sint-Pieters, Brugge', types: ['IC'] },
    { dest: 'Nijvel', via: 'Sint-Genesius-Rode, Waterloo, Eigenbrakel', types: ['S1'] },
    { dest: 'Luik-Guillemins', via: 'Leuven, Tienen, Ans', types: ['IC'] },
    { dest: 'Hasselt', via: 'Leuven, Aarschot, Diest', types: ['IC'] },
    { dest: 'Namen', via: 'Brussel-Luxemburg, Ottignies, Gembloers', types: ['IC'] },
    { dest: 'Charleroi-Centraal', via: 'Nijvel, Luttre', types: ['IC'] },
    { dest: 'Kortrijk', via: 'Gent-Sint-Pieters, Deinze, Waregem', types: ['IC'] },
    { dest: 'Brugge', via: 'Gent-Sint-Pieters', types: ['IC'] },
    { dest: 'Knokke', via: 'Gent-Sint-Pieters, Brugge', types: ['IC'] },
    { dest: 'Blankenberge', via: 'Gent-Sint-Pieters, Brugge', types: ['IC'] },
    { dest: 'Brussels Airport - Zaventem', via: 'Brussel-Noord', types: ['IC', 'S9'] },
    { dest: 'Mechelen', via: 'Brussel-Noord, Vilvoorde', types: ['S1', 'L'] },
    { dest: 'Leuven', via: 'Brussel-Noord, Zaventem, Kortenberg', types: ['S2', 'L'] },
    { dest: 'Dendermonde', via: 'Jette, Asse, Londerzeel', types: ['S10'] },
    { dest: 'Geraardsbergen', via: 'Halle, Edingen', types: ['S6', 'L'] },
    { dest: 'Aalst', via: 'Jette, Dilbeek, Denderleeuw', types: ['S10'] },
    { dest: 'Ottignies', via: 'Etterbeek, Watermaal, Genval', types: ['S8'] },
    { dest: 'Eupen', via: 'Leuven, Luik-Guillemins, Verviers-Centraal', types: ['IC'] },
    { dest: 'Turnhout', via: 'Mechelen, Lier, Herentals', types: ['IC'] },
    { dest: 'Sint-Niklaas', via: 'Dendermonde', types: ['IC'] },
    { dest: 'Luxembourg', via: 'Namen, Ciney, Libramont, Arlon', types: ['IC'], icon: 'int' },
    { dest: 'Lille-Flandres', via: 'Halle, Doornik', types: ['IC'], icon: 'int' },
  ];

  const LANG = {
    nl: { title: 'Vertrek', arriving: 'Komt aan', limited: 'Beperkt', cancelled: 'Afgeschaft', via: 'via' },
    fr: { title: 'Départ', arriving: 'Arrive', limited: 'Limité', cancelled: 'Supprimé', via: 'via' },
    de: { title: 'Abfahrt', arriving: 'Kommt an', limited: 'Begrenzt', cancelled: 'Fällt aus', via: 'über' },
    en: { title: 'Departure', arriving: 'Arriving', limited: 'Limited', cancelled: 'Cancelled', via: 'via' },
  };

  const INT_ICON = `<svg viewBox="0 0 50 50" aria-label="internationaal"><circle cx="25" cy="25" r="21" fill="none" stroke="currentColor" stroke-width="3"/><g stroke="currentColor" stroke-width="3" stroke-linecap="round"><path d="M25 11v28M12.9 18l24.2 14M12.9 32l24.2-14"/></g><circle cx="25" cy="25" r="4" fill="currentColor"/></svg>`;

  const $ = (sel, root = document) => root.querySelector(sel);
  const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

  // ---------- State ----------
  let settings;
  let trains;
  let uid = 1;
  const clockAnchor = { real: Date.now(), sec: 0 };

  function makeTrain(t = {}) {
    return {
      id: uid++,
      time: t.time || '12:00',
      dest: t.dest || '',
      via: t.via || '',
      remark: t.remark || '',
      platform: t.platform != null ? String(t.platform) : '',
      type: t.type || '',
      icon: t.icon || '',
      delay: Number(t.delay) || 0,
      expected: t.expected || '',
      status: t.status || '',
      customStatus: t.customStatus || '',
      customColor: t.customColor || '#1673d6',
      platformChanged: !!t.platformChanged,
    };
  }

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        applyData(JSON.parse(raw));
        return;
      }
    } catch (e) { /* ignore */ }
    resetAll(false);
  }

  function applyData(data) {
    settings = { ...DEFAULT_SETTINGS, ...(data.settings || {}) };
    settings.colors = { ...DEFAULT_COLORS, ...((data.settings && data.settings.colors) || {}) };
    trains = (Array.isArray(data.trains) ? data.trains : []).map(makeTrain);
  }

  function resetAll(doSave = true) {
    settings = JSON.parse(JSON.stringify(DEFAULT_SETTINGS));
    trains = DEFAULT_TRAINS.map(makeTrain);
    if (doSave) save();
  }

  let saveTimer = null;
  function save() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(exportData()));
      } catch (e) { /* ignore */ }
    }, 200);
  }

  function exportData() {
    return {
      settings,
      trains: trains.map(({ id, ...rest }) => rest),
    };
  }

  // ---------- Time helpers ----------
  const pad = (n) => String(n).padStart(2, '0');

  function parseHM(str) {
    if (!str) return null;
    const m = /^(\d{1,2}):(\d{2})(?::(\d{2}))?$/.exec(str.trim());
    if (!m) return null;
    return (+m[1]) * 60 + (+m[2]) + (m[3] ? (+m[3]) / 60 : 0);
  }

  function fmtHM(min) {
    const total = ((Math.round(min) % 1440) + 1440) % 1440;
    return `${pad(Math.floor(total / 60))}:${pad(total % 60)}`;
  }

  /** Minutes from now until `targetMin`, normalised to [-720, 720). */
  function minutesUntil(targetMin, nowMin) {
    let d = (((targetMin - nowMin) % 1440) + 1440) % 1440;
    if (d >= 720) d -= 1440;
    return d;
  }

  function anchorClock() {
    clockAnchor.real = Date.now();
    clockAnchor.sec = (parseHM(settings.customStart) || 0) * 60;
  }

  function reanchorKeepingTime() {
    if (settings.clockMode === 'custom') {
      const now = getNowSeconds();
      clockAnchor.real = Date.now();
      clockAnchor.sec = now;
    }
  }

  /** Current simulated time as seconds of the day. */
  function getNowSeconds() {
    if (settings.clockMode === 'custom') {
      const elapsed = (Date.now() - clockAnchor.real) / 1000 * Number(settings.clockSpeed || 0);
      return (((clockAnchor.sec + elapsed) % 86400) + 86400) % 86400;
    }
    let d = new Date();
    if (settings.clockMode === 'offset') {
      d = new Date(d.getTime() + (Number(settings.clockOffset) || 0) * 60000);
    }
    return d.getHours() * 3600 + d.getMinutes() * 60 + d.getSeconds() + d.getMilliseconds() / 1000;
  }

  const nowMinutes = () => getNowSeconds() / 60;

  // ---------- Language ----------
  function currentLang() {
    const langs = settings.language.split('-');
    if (langs.length === 1) return LANG[langs[0]] ? langs[0] : 'nl';
    const interval = Math.max(2, Number(settings.cycleInterval) || 8);
    return langs[Math.floor(Date.now() / 1000 / interval) % langs.length];
  }

  // ---------- Train logic ----------
  function expectedMin(t) {
    const exp = parseHM(t.expected);
    if (exp != null) return exp;
    return (parseHM(t.time) || 0) + (Number(t.delay) || 0);
  }

  function sortTrains() {
    const now = nowMinutes();
    trains.sort((a, b) => minutesUntil(parseHM(a.time) || 0, now) - minutesUntil(parseHM(b.time) || 0, now));
  }

  function randomTrain(afterMin) {
    const r = ROUTES[Math.floor(Math.random() * ROUTES.length)];
    const type = r.types[Math.floor(Math.random() * r.types.length)];
    const t = {
      time: fmtHM(afterMin),
      dest: r.dest,
      via: r.via,
      type,
      icon: r.icon || '',
      platform: String(1 + Math.floor(Math.random() * 22)),
    };
    const roll = Math.random();
    if (roll < 0.18) t.delay = [2, 3, 5, 6, 8, 10, 15, 20][Math.floor(Math.random() * 8)];
    else if (roll < 0.21) t.status = 'cancelled';
    else if (roll < 0.23) t.platformChanged = true;
    return makeTrain(t);
  }

  function nextFreeTime() {
    const now = nowMinutes();
    let last = 2;
    trains.forEach((t) => {
      const u = minutesUntil(parseHM(t.time) || 0, now);
      if (u > last) last = u;
    });
    return Math.ceil(now + last + Math.floor(Math.random() * 7));
  }

  function shiftToNow() {
    if (!trains.length) return;
    const now = nowMinutes();
    let first = Infinity;
    trains.forEach((t) => { first = Math.min(first, minutesUntil(parseHM(t.time) || 0, now)); });
    const delta = Math.ceil(3 - first);
    trains.forEach((t) => {
      t.time = fmtHM((parseHM(t.time) || 0) + delta);
      if (t.expected) t.expected = fmtHM((parseHM(t.expected) || 0) + delta);
    });
  }

  /** Runs every tick: auto depart / fill / sort. Returns true if trains changed. */
  function simulate() {
    let changed = false;
    const now = nowMinutes();
    if (settings.autoDepart) {
      const grace = Number(settings.departGrace) || 0;
      const before = trains.length;
      trains = trains.filter((t) => {
        const ref = t.status === 'cancelled' ? (parseHM(t.time) || 0) : expectedMin(t);
        return -minutesUntil(ref, now) < grace;
      });
      if (trains.length !== before) changed = true;
    }
    if (settings.autoFill) {
      let guard = 0;
      while (trains.length < settings.rowsVisible && guard++ < 20) {
        trains.push(randomTrain(nextFreeTime()));
        changed = true;
      }
    }
    if (settings.autoSort) {
      const order = trains.map((t) => t.id).join();
      sortTrains();
      if (trains.map((t) => t.id).join() !== order) changed = true;
    }
    return changed;
  }

  // ---------- Board rendering ----------
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

  function badgeHTML(t, L, now) {
    const statusText = {
      arriving: L.arriving,
      limited: L.limited,
      cancelled: L.cancelled,
    };
    if (t.status === 'custom' && t.customStatus) {
      return `<div class="badge"><span class="b-status" style="background:${esc(t.customColor)}">${esc(t.customStatus)}</span></div>`;
    }
    if (t.status === 'limited' || t.status === 'cancelled') {
      return `<div class="badge alert"><span class="b-status">${esc(statusText[t.status])}</span></div>`;
    }
    if (t.status === 'arriving') {
      return `<div class="badge"><span class="b-status">${esc(L.arriving)}</span></div>`;
    }
    const delay = Number(t.delay) || 0;
    if (delay > 0 || t.expected) {
      const exp = expectedMin(t);
      const shownDelay = delay > 0 ? delay : Math.round(minutesUntil(exp, parseHM(t.time) || 0));
      if (shownDelay > 0) {
        return `<div class="badge"><span class="b-delay">+${shownDelay}'</span><span class="b-new">${fmtHM(exp)}</span></div>`;
      }
    }
    const autoArr = Number(settings.autoArriving) || 0;
    if (autoArr > 0) {
      const u = minutesUntil(expectedMin(t), now);
      if (u >= 0 && u <= autoArr) {
        return `<div class="badge"><span class="b-status">${esc(L.arriving)}</span></div>`;
      }
    }
    return '';
  }

  function rowHTML(t, L, now) {
    if (!t) return '<div class="row empty"></div>';
    let second = '';
    if (t.remark) second = t.remark;
    else if (t.via) second = settings.showViaWord ? `${L.via} ${t.via}` : t.via;
    const typeCell = t.icon === 'int' ? INT_ICON : esc(t.type);
    const platCls = t.platformChanged ? 'platform changed' : 'platform';
    return `<div class="row${t.status === 'cancelled' ? ' cancelled' : ''}">
      <div class="cell-time"><div class="time">${esc(t.time)}</div>${badgeHTML(t, L, now)}</div>
      <div class="cell-main"><div class="dest">${esc(t.dest)}</div><div class="via"><span class="via-inner">${esc(second)}</span></div></div>
      <div class="cell-platform">${t.platform !== '' ? `<div class="${platCls}">${esc(t.platform)}</div>` : ''}</div>
      <div class="cell-type">${typeCell}</div>
    </div>`;
  }

  let lastBoardHTML = '';
  let lastLayoutKey = '';

  function renderBoard() {
    const board = $('#board');
    const n = Math.max(3, Math.min(14, Number(settings.rowsVisible) || 8));
    const rowH = (BOARD_H - HEADER_H) / n;
    const k = Math.max(0.5, Math.min(1.25, rowH / BASE_ROW_H));
    const layoutKey = `${n}|${settings.alternateRows}|${settings.showLogo}`;
    if (layoutKey !== lastLayoutKey) {
      board.style.setProperty('--row-h', `${rowH / k}px`);
      board.style.setProperty('--row-zoom', k);
      board.classList.toggle('alt', !!settings.alternateRows);
      $('#boardLogo').classList.toggle('hidden', !settings.showLogo);
      lastLayoutKey = layoutKey;
      lastBoardHTML = '';
    }

    const lang = currentLang();
    const L = LANG[lang];
    $('#boardTitle').textContent = settings.title || L.title;

    const now = nowMinutes();
    const visible = trains.slice(0, n);
    const rows = [];
    for (let i = 0; i < n; i++) {
      if (visible[i]) rows.push(rowHTML(visible[i], L, now));
      else if (settings.showEmptyRows) rows.push(rowHTML(null));
    }
    const html = rows.join('');
    if (html !== lastBoardHTML) {
      const container = $('#boardRows');
      container.innerHTML = html;
      $$('.row', container).forEach((r) => {
        r.style.zoom = k;
        r.style.width = `${BOARD_W / k}px`;
      });
      lastBoardHTML = html;
      setupMarquees();
    }
  }

  function setupMarquees() {
    $$('#boardRows .via').forEach((el) => {
      el.classList.remove('scroll');
      if (!settings.marquee) return;
      const inner = el.firstElementChild;
      const overflow = inner.scrollWidth - el.clientWidth;
      if (overflow > 4) {
        el.style.setProperty('--dist', `${overflow + 10}px`);
        el.style.setProperty('--dur', `${Math.max(6, overflow / 40 + 5)}s`);
        el.classList.add('scroll');
      }
    });
  }

  function renderClock() {
    const sec = getNowSeconds();
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    const s = Math.floor(sec % 60);
    const colonOff = settings.blinkColon && Math.floor(Date.now() / 500) % 2 === 1;
    const colon = `<span class="colon${colonOff ? ' off' : ''}">:</span>`;
    let html = `${pad(h)}${colon}${pad(m)}`;
    if (settings.showSeconds) html += `<span class="sec">${colon}${pad(s)}</span>`;
    const el = $('#boardClock');
    if (el.innerHTML !== html) el.innerHTML = html;
  }

  function applyColors() {
    const root = document.documentElement;
    Object.entries(settings.colors).forEach(([k, v]) => root.style.setProperty(`--c-${k}`, v));
  }

  function fitBoard() {
    const stage = $('#stage');
    const s = Math.min(stage.clientWidth / BOARD_W, stage.clientHeight / BOARD_H);
    $('#board').style.transform = `scale(${s})`;
  }

  // ---------- Editor ----------
  function renderEditor() {
    const list = $('#trainList');
    list.innerHTML = '';
    if (!trains.length) {
      list.innerHTML = '<div class="empty-note">Geen treinen. Voeg er een toe met “+ Trein”.</div>';
      return;
    }
    const tpl = $('#trainTemplate');
    trains.forEach((t) => {
      const card = tpl.content.firstElementChild.cloneNode(true);
      card.dataset.id = t.id;
      $$('[data-field]', card).forEach((inp) => {
        const f = inp.dataset.field;
        if (inp.type === 'checkbox') inp.checked = !!t[f];
        else inp.value = t[f] ?? '';
      });
      updateCardTitle(card, t);
      list.appendChild(card);
    });
  }

  function updateCardTitle(card, t) {
    $('.train-card-title', card).textContent = `${t.time || '--:--'}  ${t.dest || '(geen bestemming)'}`;
  }

  function findTrain(el) {
    const card = el.closest('.train-card');
    if (!card) return {};
    const id = Number(card.dataset.id);
    const idx = trains.findIndex((t) => t.id === id);
    return { card, idx, train: trains[idx] };
  }

  function onTrainInput(e) {
    const inp = e.target;
    const f = inp.dataset.field;
    if (!f) return;
    const { card, train } = findTrain(inp);
    if (!train) return;
    if (inp.type === 'checkbox') train[f] = inp.checked;
    else if (inp.type === 'number') train[f] = Math.max(0, parseInt(inp.value, 10) || 0);
    else train[f] = inp.value;
    updateCardTitle(card, train);
    save();
    renderBoard();
  }

  function onTrainAction(e) {
    const btn = e.target.closest('button[data-action]');
    if (!btn) return;
    const { idx, train } = findTrain(btn);
    if (!train) return;
    switch (btn.dataset.action) {
      case 'up':
        if (idx > 0) [trains[idx - 1], trains[idx]] = [trains[idx], trains[idx - 1]];
        break;
      case 'down':
        if (idx < trains.length - 1) [trains[idx + 1], trains[idx]] = [trains[idx], trains[idx + 1]];
        break;
      case 'duplicate': {
        const { id, ...rest } = train;
        trains.splice(idx + 1, 0, makeTrain(rest));
        break;
      }
      case 'delete':
        trains.splice(idx, 1);
        break;
      default:
        return;
    }
    changedTrains();
  }

  function changedTrains() {
    save();
    renderEditor();
    renderBoard();
  }

  /** Re-render the editor, but not while the user is typing in it. */
  let editorDirty = false;
  function refreshEditorSoon() {
    if ($('#trainList').contains(document.activeElement)) editorDirty = true;
    else renderEditor();
  }

  // ---------- Settings UI ----------
  function syncSettingsUI() {
    $$('[data-setting]').forEach((el) => {
      const v = settings[el.dataset.setting];
      if (el.type === 'checkbox') el.checked = !!v;
      else el.value = v ?? '';
    });
    $$('[data-color]').forEach((el) => { el.value = settings.colors[el.dataset.color]; });
    updateShowIf();
  }

  function updateShowIf() {
    $$('[data-show-if]').forEach((el) => {
      const [key, val] = el.dataset.showIf.split('=');
      el.style.display = String(settings[key]) === val ? '' : 'none';
    });
  }

  function onSettingChange(e) {
    const el = e.target;
    const key = el.dataset.setting;
    if (!key) return;
    let v;
    if (el.type === 'checkbox') v = el.checked;
    else if (el.type === 'number' || key === 'clockSpeed') {
      if (el.value === '' || el.value === '-') return;
      v = Number(el.value);
      if (Number.isNaN(v)) return;
    } else v = el.value;

    if (key === 'clockSpeed') reanchorKeepingTime();
    settings[key] = v;
    if (key === 'clockMode' || key === 'customStart') anchorClock();
    if (key === 'rowsVisible') settings.rowsVisible = Math.max(3, Math.min(14, v));
    updateShowIf();
    save();
    if (simulate()) refreshEditorSoon();
    renderBoard();
    renderClock();
    setupMarquees();
  }

  function onColorChange(e) {
    const key = e.target.dataset.color;
    if (!key) return;
    settings.colors[key] = e.target.value;
    applyColors();
    save();
  }

  // ---------- Panel ----------
  function setPanel(open) {
    document.body.classList.toggle('panel-open', open);
    setTimeout(fitBoard, 260);
    fitBoard();
  }

  function toggleFullscreen() {
    if (document.fullscreenElement) document.exitFullscreen();
    else document.documentElement.requestFullscreen?.().catch(() => {});
  }

  let cursorTimer = null;
  function wakeCursor() {
    document.body.classList.remove('fs-hide-cursor');
    clearTimeout(cursorTimer);
    cursorTimer = setTimeout(() => {
      if (!document.body.classList.contains('panel-open')) document.body.classList.add('fs-hide-cursor');
    }, 3000);
  }

  function bindUI() {
    $('#panelToggle').addEventListener('click', () => setPanel(true));
    $('#btnClosePanel').addEventListener('click', () => setPanel(false));
    $('#btnFullscreen').addEventListener('click', toggleFullscreen);

    $$('.tab').forEach((tab) => tab.addEventListener('click', () => {
      $$('.tab').forEach((t) => t.classList.toggle('active', t === tab));
      $$('.tab-page').forEach((p) => p.classList.toggle('active', p.dataset.page === tab.dataset.tab));
    }));

    const list = $('#trainList');
    list.addEventListener('input', onTrainInput);
    list.addEventListener('change', onTrainInput);
    list.addEventListener('click', onTrainAction);
    list.addEventListener('focusout', () => setTimeout(() => {
      if (editorDirty && !list.contains(document.activeElement)) { editorDirty = false; renderEditor(); }
    }, 0));

    $('#btnAddTrain').addEventListener('click', () => {
      trains.push(makeTrain({ time: fmtHM(nextFreeTime()), dest: 'Nieuwe bestemming', platform: '1', type: 'IC' }));
      changedTrains();
      list.lastElementChild?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
    $('#btnRandomTrain').addEventListener('click', addRandom);
    $('#btnSort').addEventListener('click', () => { sortTrains(); changedTrains(); });
    $('#btnShiftNow').addEventListener('click', () => { shiftToNow(); sortTrains(); changedTrains(); });
    $('#btnClearTrains').addEventListener('click', () => {
      if (confirm('Alle treinen verwijderen?')) { trains = []; changedTrains(); }
    });

    document.addEventListener('input', (e) => {
      if (e.target.dataset.setting) onSettingChange(e);
      else if (e.target.dataset.color) onColorChange(e);
    });
    document.addEventListener('change', (e) => {
      if (e.target.dataset.setting) onSettingChange(e);
    });

    $('#btnRestartClock').addEventListener('click', () => { anchorClock(); renderClock(); });
    $('#btnResetColors').addEventListener('click', () => {
      settings.colors = { ...DEFAULT_COLORS };
      applyColors();
      syncSettingsUI();
      save();
    });

    $('#btnExport').addEventListener('click', () => {
      const blob = new Blob([JSON.stringify(exportData(), null, 2)], { type: 'application/json' });
      const a = document.createElement('a');
      a.href = URL.createObjectURL(blob);
      a.download = 'nmbs-vertrekbord.json';
      a.click();
      setTimeout(() => URL.revokeObjectURL(a.href), 1000);
    });
    $('#btnImport').addEventListener('click', () => $('#importFile').click());
    $('#importFile').addEventListener('change', async (e) => {
      const file = e.target.files[0];
      if (!file) return;
      try {
        applyData(JSON.parse(await file.text()));
        afterDataLoad();
      } catch (err) {
        alert('Kon bestand niet lezen: ' + err.message);
      }
      e.target.value = '';
    });
    $('#btnReset').addEventListener('click', () => {
      if (confirm('Alle instellingen en treinen terugzetten naar standaard?')) {
        resetAll();
        afterDataLoad();
      }
    });

    document.addEventListener('keydown', (e) => {
      const tag = e.target.tagName;
      if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA' || e.ctrlKey || e.metaKey || e.altKey) return;
      const key = e.key.toLowerCase();
      if (key === 'p' || key === 'h') setPanel(!document.body.classList.contains('panel-open'));
      else if (key === 'f') toggleFullscreen();
      else if (key === 'r') addRandom();
      else if (key === 'escape') setPanel(false);
    });

    document.addEventListener('mousemove', wakeCursor);
    window.addEventListener('resize', fitBoard);
    if (window.ResizeObserver) new ResizeObserver(fitBoard).observe($('#stage'));
  }

  function addRandom() {
    trains.push(randomTrain(nextFreeTime()));
    if (settings.autoSort) sortTrains();
    changedTrains();
  }

  function afterDataLoad() {
    anchorClock();
    applyColors();
    syncSettingsUI();
    lastLayoutKey = '';
    save();
    renderEditor();
    renderBoard();
    renderClock();
  }

  // ---------- Main loop ----------
  function tick() {
    renderClock();
    if (simulate()) {
      save();
      refreshEditorSoon();
    }
    renderBoard();
  }

  function init() {
    load();
    anchorClock();
    applyColors();
    syncSettingsUI();
    bindUI();
    renderEditor();
    renderBoard();
    renderClock();
    fitBoard();
    setPanel(location.hash !== '#display');
    if (document.fonts?.ready) document.fonts.ready.then(() => { lastBoardHTML = ''; renderBoard(); });
    setInterval(tick, 250);
  }

  document.addEventListener('DOMContentLoaded', init);
})();
