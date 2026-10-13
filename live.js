/*
 * Live NMBS/SNCB data via the open iRail API (https://docs.irail.be).
 *
 * Endpoints used:
 *   /stations/   all Belgian (and some foreign) stations with their ids
 *   /liveboard/  real-time departures or arrivals of one station
 *   /vehicle/    all stops of one train (used to build the "via" line)
 *
 * iRail asks clients to stay under ~3 requests per second, so every request
 * goes through a small queue.
 */
(() => {
  'use strict';

  const API = 'https://api.irail.be/v1';
  const MIN_GAP_MS = 350;
  const STATION_CACHE_KEY = 'nmbs-irail-stations-';
  const STATION_CACHE_TTL = 7 * 24 * 3600 * 1000;
  const INTERNATIONAL_TYPES = ['ICE', 'EUR', 'EST', 'TGV', 'THA', 'ECD', 'ICD', 'EC', 'NJ'];

  // ---------- Request queue ----------
  let queue = Promise.resolve();
  let lastRequest = 0;

  function request(path, params) {
    const url = `${API}/${path}/?${new URLSearchParams({ format: 'json', ...params })}`;
    const run = async () => {
      const wait = lastRequest + MIN_GAP_MS - Date.now();
      if (wait > 0) await new Promise((r) => setTimeout(r, wait));
      lastRequest = Date.now();
      const res = await fetch(url, { headers: { Accept: 'application/json' } });
      if (!res.ok) {
        let msg = `HTTP ${res.status}`;
        try { msg = (await res.json()).message || msg; } catch (e) { /* ignore */ }
        throw new Error(msg);
      }
      return res.json();
    };
    const p = queue.then(run, run);
    queue = p.catch(() => {});
    return p;
  }

  // ---------- Helpers ----------
  const timeFmt = new Intl.DateTimeFormat('nl-BE', {
    timeZone: 'Europe/Brussels', hour: '2-digit', minute: '2-digit', hourCycle: 'h23',
  });
  const dateFmt = new Intl.DateTimeFormat('nl-BE', {
    timeZone: 'Europe/Brussels', day: '2-digit', month: '2-digit', year: '2-digit',
  });

  /** Unix seconds -> "HH:MM" in Belgian time. */
  const toHM = (unix) => timeFmt.format(new Date(Number(unix) * 1000));

  /** Unix seconds -> "ddmmyy" (iRail date format) in Belgian time. */
  function toIrailDate(unix) {
    const parts = Object.fromEntries(dateFmt.formatToParts(new Date(Number(unix) * 1000)).map((p) => [p.type, p.value]));
    return `${parts.day}${parts.month}${parts.year}`;
  }

  const asArray = (x) => (Array.isArray(x) ? x : x ? [x] : []);

  /** Belgian stations have ids BE.NMBS.0088xxxxx; anything else is abroad. */
  const isForeign = (stationId) => !!stationId && !/^BE\.NMBS\.0088/.test(stationId);

  // ---------- Stations ----------
  async function getStations(lang) {
    const key = STATION_CACHE_KEY + lang;
    try {
      const cached = JSON.parse(localStorage.getItem(key) || 'null');
      if (cached && Date.now() - cached.at < STATION_CACHE_TTL) return cached.list;
    } catch (e) { /* ignore */ }
    const data = await request('stations', { lang });
    const list = asArray(data.station)
      .map((s) => ({ id: s.id, name: s.name, standardname: s.standardname }))
      .sort((a, b) => a.name.localeCompare(b.name, 'nl'));
    try { localStorage.setItem(key, JSON.stringify({ at: Date.now(), list })); } catch (e) { /* quota */ }
    return list;
  }

  // ---------- Liveboard ----------
  /**
   * Returns { station, stationId, trains } where every train has the same
   * shape as the simulator's own trains plus `vehicle` and `ts` (unix time).
   */
  async function getLiveboard({ stationId, arrdep = 'departure', lang = 'nl' }) {
    const data = await request('liveboard', { id: stationId, arrdep, lang, alerts: 'false' });
    const block = arrdep === 'arrival' ? data.arrivals : data.departures;
    const items = asArray(block && (block.arrival || block.departure));
    const trains = items.map((d) => {
      const vi = d.vehicleinfo || {};
      const type = (vi.type || String(vi.shortname || '').split(' ')[0] || '').toUpperCase();
      const otherId = d.stationinfo && d.stationinfo.id;
      return {
        time: toHM(d.time),
        ts: Number(d.time),
        dest: d.station,
        destId: otherId || '',
        via: '',
        platform: d.platform && d.platform !== '?' ? String(d.platform) : '',
        platformChanged: !!(d.platforminfo && d.platforminfo.normal === '0'),
        type,
        icon: INTERNATIONAL_TYPES.includes(type) || isForeign(otherId) ? 'int' : '',
        delay: Math.round((Number(d.delay) || 0) / 60),
        status: d.canceled === '1' ? 'cancelled' : '',
        left: d.left === '1',
        vehicle: d.vehicle || vi.name || '',
      };
    });
    return {
      station: (data.stationinfo && data.stationinfo.name) || data.station || '',
      stationId: (data.stationinfo && data.stationinfo.id) || stationId,
      trains,
    };
  }

  // ---------- Via stations ----------
  const viaCache = new Map();

  /** Picks `max` stops spread evenly over the list (always keeping the first). */
  function spread(list, max) {
    if (list.length <= max) return list;
    if (max === 1) return [list[0]];
    const out = [];
    for (let i = 0; i < max; i++) out.push(list[Math.round(i * (list.length - 1) / (max - 1))]);
    return [...new Set(out)];
  }

  /**
   * Intermediate stops of a train between this station and its
   * destination (departures) or its origin and this station (arrivals).
   */
  async function getVia({ vehicle, ts, stationId, arrdep = 'departure', lang = 'nl', max = 3 }) {
    if (!vehicle || max <= 0) return '';
    const date = toIrailDate(ts);
    const key = `${vehicle}|${date}|${stationId}|${arrdep}|${lang}`;
    if (!viaCache.has(key)) {
      const p = request('vehicle', { id: vehicle, date, lang, alerts: 'false' }).then((data) => {
        const stops = asArray(data.stops && data.stops.stop);
        const idx = stops.findIndex((s) => s.stationinfo && s.stationinfo.id === stationId);
        if (idx < 0) return [];
        const between = arrdep === 'arrival' ? stops.slice(1, idx) : stops.slice(idx + 1, -1);
        return between.filter((s) => s.canceled !== '1').map((s) => s.station);
      });
      viaCache.set(key, p);
      p.catch(() => viaCache.delete(key));
      if (viaCache.size > 500) viaCache.delete(viaCache.keys().next().value);
    }
    const names = await viaCache.get(key);
    return spread(names, max).join(', ');
  }

  window.NMBSLive = { getStations, getLiveboard, getVia };
})();
