import {
  VeilClient,
  ConnectionState,
  ConnectionStats,
  ServerInfo,
  ServerStats,
  StreamParamsRequest,
  GamepadStatus,
  GamepadType,
  NetworkStatus,
  VideoCodecName,
  detectGamepadLayout,
  CompositorState,
  ViewerList,
  AudioPlayer,
} from '../sdk/src/index';

// ─── DOM ──────────────────────────────────────────────────────────────────────

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const canvas        = $<HTMLCanvasElement>('stream-canvas');
const canvasWrap    = canvas.parentElement as HTMLDivElement;
const vcursor       = $<HTMLImageElement>('vcursor');
const captureBadge  = $<HTMLDivElement>('captureBadge');
const fullscreenBtn = $<HTMLButtonElement>('fullscreenBtn');
const overlay       = $<HTMLDivElement>('overlay');
const overlayStatus = $<HTMLSpanElement>('overlay-status');
const panelToggle   = $<HTMLButtonElement>('panelToggle');

const connDot    = $<HTMLDivElement>('connDot');
const connState  = $<HTMLSpanElement>('connState');
const connNote   = $<HTMLSpanElement>('connNote');
const viewersEl      = $<HTMLDivElement>('viewers');
const viewersSummary = $('viewersSummary');
const viewerList     = $<HTMLDivElement>('viewerList');
const serverUrl  = $<HTMLInputElement>('serverUrl');
const connectBtn = $<HTMLButtonElement>('connectBtn');
const disconnBtn = $<HTMLButtonElement>('disconnBtn');

const infoGpu    = $('infoGpu');
const infoDriver = $('infoDriver');
const infoWrite  = $('infoWrite');
const infoCodec  = $('infoCodec');
const infoRc     = $('infoRc');
const infoBitrate = $('infoBitrate');
const infoNet    = $('infoNet');
const infoStream = $('infoStream');
const infoGop    = $('infoGop');

const liveNote      = $('liveNote');
const fpsCanvas     = $<HTMLCanvasElement>('fpsCanvas');
const bitrateCanvas = $<HTMLCanvasElement>('bitrateCanvas');
const latencyCanvas = $<HTMLCanvasElement>('latencyCanvas');
const valFps        = $('valFps');
const valBitrate    = $('valBitrate');
const valLatency    = $('valLatency');
const barEncode     = $('barEncode');
const barNet        = $('barNet');
const barDecode     = $('barDecode');
const legEncode     = $('legEncode');
const legNet        = $('legNet');
const legDecode     = $('legDecode');
const statDecoded   = $('statDecoded');
const statDropped   = $('statDropped');
const statQueue     = $('statQueue');
const statEncMax    = $('statEncMax');
const statKeys      = $('statKeys');
const statFrameKb   = $('statFrameKb');

const soundNote        = $('soundNote');
const soundEnable      = $<HTMLButtonElement>('soundEnable');
const soundMute        = $<HTMLButtonElement>('soundMute');
const soundVolume      = $<HTMLInputElement>('soundVolume');
const soundVolumeLabel = $<HTMLOutputElement>('soundVolumeLabel');
const soundOverlay     = $<HTMLButtonElement>('soundOverlay');
const soundBuffered    = $('soundBuffered');
const soundUnderruns   = $('soundUnderruns');
const soundTrims       = $('soundTrims');
const soundRate        = $('soundRate');
const soundBitrate     = $('soundBitrate');
const soundLatency     = $('soundLatency');

const settingsNote = $('settingsNote');
const setGpu       = $<HTMLSelectElement>('setGpu');
const switchGpuBtn = $<HTMLButtonElement>('switchGpuBtn');
const setCodec     = $<HTMLSelectElement>('setCodec');
const setPreset    = $<HTMLInputElement>('setPreset');
const presetLabel  = $<HTMLOutputElement>('presetLabel');
const setRes       = $<HTMLSelectElement>('setRes');
const setFps       = $<HTMLSelectElement>('setFps');
const setGop       = $<HTMLInputElement>('setGop');
const setInFlight  = $<HTMLSelectElement>('setInFlight');
const applyBtn     = $<HTMLButtonElement>('applyBtn');
const revertBtn    = $<HTMLButtonElement>('revertBtn');
const applyHint    = $('applyHint');

const logBody     = $<HTMLTableSectionElement>('logBody');
const logEmpty    = $('logEmpty');
const copyLogBtn  = $<HTMLButtonElement>('copyLogBtn');
const clearLogBtn = $<HTMLButtonElement>('clearLogBtn');

const padList  = $<HTMLDivElement>('padList');
const padEmpty = $<HTMLDivElement>('padEmpty');
const padNote  = $('padNote');

const compNote       = $('compNote');
const compDisplay    = $('compDisplay');
const compApp        = $('compApp');
const compSockets    = $('compSockets');
const compClients    = $('compClients');
const compWindows    = $<HTMLDivElement>('compWindows');
const compX11Wrap    = $<HTMLDetailsElement>('compX11Wrap');
const compX11Summary = $('compX11Summary');
const compX11        = $<HTMLDivElement>('compX11');
const toastEl        = $<HTMLDivElement>('toast');

// ─── Helpers ──────────────────────────────────────────────────────────────────

let toastTimer: ReturnType<typeof setTimeout> | null = null;
function toast(msg: string) {
  toastEl.textContent = msg;
  toastEl.classList.add('show');
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toastEl.classList.remove('show'), 3000);
}

const fmtMs = (v: number) => (v > 0 ? `${v < 10 ? v.toFixed(1) : Math.round(v)} ms` : '—');
const codecName = (c: string) => ({ h265: 'H.265', av1: 'AV1' } as Record<string, string>)[c] ?? 'H.264';

/** Mirrors the server's preset → rate control mapping (rate_control.rs). */
function presetDescription(preset: number): string {
  if (preset >= 7) return `Const QP ${40 - 2 * preset}`;
  return `VBR · ${['lowest', 'low', 'low+', 'medium', 'medium+', 'high', 'highest'][preset]}`;
}

function shortGpu(name: string): string {
  return name
    .replace(/^AMD Radeon /, '')
    .replace(/^NVIDIA GeForce /, '')
    .replace(/\s*\(.*\)$/, '');
}

function gpuVendorTag(name: string): string {
  if (/radeon|amd|radv/i.test(name)) return '<span class="tag amd">AMD</span>';
  if (/nvidia|geforce/i.test(name)) return '<span class="tag nvidia">NVIDIA</span>';
  return '';
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}

// ─── Graphs ───────────────────────────────────────────────────────────────────

const MAX_SAMPLES = 60;
const fpsHistory: number[]     = new Array(MAX_SAMPLES).fill(0);
const bitrateHistory: number[] = new Array(MAX_SAMPLES).fill(0);
const latencyHistory: number[] = new Array(MAX_SAMPLES).fill(0);

function push(arr: number[], v: number) {
  arr.push(v);
  if (arr.length > MAX_SAMPLES) arr.shift();
}

function drawSparkline(cnv: HTMLCanvasElement, data: number[], lineHex: string, fillRgba: string) {
  const dpr  = Math.min(window.devicePixelRatio || 1, 2);
  const rect = cnv.getBoundingClientRect();
  const w    = rect.width  || 100;
  const h    = rect.height || 30;

  if (cnv.width !== Math.round(w * dpr) || cnv.height !== Math.round(h * dpr)) {
    cnv.width  = Math.round(w * dpr);
    cnv.height = Math.round(h * dpr);
  }

  const ctx = cnv.getContext('2d')!;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);

  if (data.filter((v) => v > 0).length < 2) return;

  const max = Math.max(...data) * 1.2;
  const len = data.length;
  const pad = 2;
  const px = (i: number) => (i / (len - 1)) * w;
  const py = (v: number) => pad + (1 - v / max) * (h - pad * 2);

  ctx.beginPath();
  ctx.moveTo(px(0), h);
  for (let i = 0; i < len; i++) ctx.lineTo(px(i), py(data[i]));
  ctx.lineTo(px(len - 1), h);
  ctx.closePath();
  ctx.fillStyle = fillRgba;
  ctx.fill();

  ctx.beginPath();
  for (let i = 0; i < len; i++) {
    if (i === 0) ctx.moveTo(px(i), py(data[i]));
    else         ctx.lineTo(px(i), py(data[i]));
  }
  ctx.strokeStyle = lineHex;
  ctx.lineWidth   = 1.5;
  ctx.lineJoin    = 'round';
  ctx.stroke();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
}

function drawGraphs() {
  drawSparkline(fpsCanvas,     fpsHistory,     '#4ade80', 'rgba(74,222,128,0.13)');
  drawSparkline(bitrateCanvas, bitrateHistory, '#60a5fa', 'rgba(96,165,250,0.13)');
  drawSparkline(latencyCanvas, latencyHistory, '#fb923c', 'rgba(251,146,60,0.13)');
}

function resetLive() {
  fpsHistory.fill(0);
  bitrateHistory.fill(0);
  latencyHistory.fill(0);
  for (const el of [valFps, valBitrate, valLatency, legEncode, legNet, legDecode,
                    statDecoded, statDropped, statQueue, statEncMax, statKeys, statFrameKb]) {
    el.textContent = '—';
  }
  for (const el of [barEncode, barNet, barDecode]) el.style.width = '0';
  drawGraphs();
}

// ─── Panel toggle ─────────────────────────────────────────────────────────────

function togglePanel() {
  document.body.classList.toggle('panel-collapsed');
  drawGraphs();
}

panelToggle.addEventListener('click', togglePanel);

// Tab toggles the panel — but not while typing in one of its fields.
document.addEventListener('keydown', (e) => {
  const t = e.target as HTMLElement;
  if (e.key === 'Tab' && !t.closest('input, select, textarea')) {
    e.preventDefault();
    e.stopPropagation();
    togglePanel();
  }
}, { capture: true });

// ─── Session log ──────────────────────────────────────────────────────────────

/** Samples taken right after a change are skipped (encoder warm-up, resize). */
const SETTLE_MS = 2000;

interface Segment {
  key: string;
  label: string;
  detail: string;
  start: number;
  samples: number;
  fps: number;
  mbps: number;
  encodeMs: number;
  encodeSamples: number;
  decodeMs: number;
  rttMs: number;
  droppedAtStart: number;
  dropped: number;
}

const segments: Segment[] = [];
let current: Segment | null = null;

function segmentKey(info: ServerInfo): string {
  return [info.gpu, info.codec, info.width, info.height, info.framerate,
          info.preset, info.gop_size, info.in_flight_frames].join('|');
}

function startSegment(info: ServerInfo, droppedNow: number) {
  const key = segmentKey(info);
  if (current?.key === key) return;
  current = {
    key,
    label: `${shortGpu(info.gpu)} · ${codecName(info.codec)}`,
    detail: `${info.width}×${info.height}@${info.framerate} · q${info.preset} · gop ${info.gop_size} · if ${info.in_flight_frames}`,
    start: performance.now(),
    samples: 0, fps: 0, mbps: 0,
    encodeMs: 0, encodeSamples: 0, decodeMs: 0, rttMs: 0,
    droppedAtStart: droppedNow, dropped: 0,
  };
  segments.push(current);
  renderLog();
}

function settled(seg: Segment) {
  return performance.now() - seg.start >= SETTLE_MS;
}

function avg(total: number, n: number) {
  return n > 0 ? total / n : 0;
}

function renderLog() {
  logEmpty.style.display = segments.length ? 'none' : '';
  // Highlight the best value per column among rows with data.
  const rows = segments.filter((s) => s.samples > 0);
  const best = {
    fps: Math.max(...rows.map((s) => avg(s.fps, s.samples))),
    enc: Math.min(...rows.map((s) => avg(s.encodeMs, s.encodeSamples) || Infinity)),
    dec: Math.min(...rows.map((s) => avg(s.decodeMs, s.samples) || Infinity)),
  };

  logBody.innerHTML = '';
  for (const seg of [...segments].reverse()) {
    const tr = document.createElement('tr');
    if (seg === current) tr.className = 'live';
    const secs = Math.round((performance.now() - seg.start) / 1000);
    const has = seg.samples > 0;
    const fps = avg(seg.fps, seg.samples);
    const enc = avg(seg.encodeMs, seg.encodeSamples);
    const dec = avg(seg.decodeMs, seg.samples);
    const cls = (isBest: boolean) => (isBest && rows.length > 1 ? ' class="best"' : '');
    tr.innerHTML = `
      <td class="cfg">${escapeHtml(seg.label)}<small>${escapeHtml(seg.detail)}</small></td>
      <td>${seg === current ? secs : Math.round(seg.samples)}s</td>
      <td${cls(has && fps === best.fps)}>${has ? fps.toFixed(0) : '…'}</td>
      <td>${has ? avg(seg.mbps, seg.samples).toFixed(1) : '…'}</td>
      <td${cls(has && enc === best.enc)}>${has && enc ? enc.toFixed(1) : '…'}</td>
      <td${cls(has && dec === best.dec)}>${has ? dec.toFixed(1) : '…'}</td>
      <td>${has ? avg(seg.rttMs, seg.samples).toFixed(0) : '…'}</td>
      <td>${seg.dropped}</td>`;
    logBody.appendChild(tr);
  }
}

function logAsMarkdown(): string {
  const lines = [
    '| Config | Settings | Seconds | FPS | Mbps | Encode ms | Decode ms | RTT ms | Dropped |',
    '|---|---|---|---|---|---|---|---|---|',
  ];
  for (const s of segments) {
    lines.push(`| ${s.label} | ${s.detail} | ${s.samples} | ${avg(s.fps, s.samples).toFixed(1)} | ` +
      `${avg(s.mbps, s.samples).toFixed(2)} | ${avg(s.encodeMs, s.encodeSamples).toFixed(2)} | ` +
      `${avg(s.decodeMs, s.samples).toFixed(2)} | ${avg(s.rttMs, s.samples).toFixed(1)} | ${s.dropped} |`);
  }
  return lines.join('\n');
}

copyLogBtn.addEventListener('click', async () => {
  try {
    await navigator.clipboard.writeText(logAsMarkdown());
    toast('Session log copied as Markdown');
  } catch {
    toast('Clipboard unavailable — see the console');
    console.log(logAsMarkdown());
  }
});

clearLogBtn.addEventListener('click', () => {
  segments.length = 0;
  if (current) {
    current.start = performance.now();
    current.samples = current.fps = current.mbps = 0;
    current.encodeMs = current.encodeSamples = current.decodeMs = current.rttMs = 0;
    current.dropped = 0;
    current.droppedAtStart = lastStats?.video.framesDropped ?? 0;
    segments.push(current);
  }
  renderLog();
});

// ─── Live stats ───────────────────────────────────────────────────────────────

let lastStats: ConnectionStats | null = null;
let lastServerStats: ServerStats | null = null;

function updateLatency() {
  const enc = lastServerStats?.encode_ms_avg ?? 0;
  const net = (lastStats?.rttMs ?? 0) / 2;
  const dec = lastStats?.decodeLatencyMs ?? 0;
  const total = enc + net + dec;
  push(latencyHistory, total);
  valLatency.innerHTML = total > 0 ? `${total.toFixed(1)}<small> ms</small>` : '—';
  legEncode.textContent = fmtMs(enc);
  legNet.textContent    = fmtMs(net);
  legDecode.textContent = fmtMs(dec);
  // Scale the bar so 50 ms fills it.
  const pct = (v: number) => `${Math.min(100, (v / 50) * 100)}%`;
  barEncode.style.width = pct(enc);
  barNet.style.width    = pct(net);
  barDecode.style.width = pct(dec);
}

function onStats(stats: ConnectionStats) {
  lastStats = stats;
  onAudioStats(stats);
  const mbps = stats.video.bitrateBps / 1_000_000;
  push(fpsHistory, stats.video.fps);
  push(bitrateHistory, mbps);

  valFps.textContent     = String(stats.video.fps);
  valBitrate.textContent = mbps.toFixed(1);
  statDecoded.textContent = stats.video.framesDecoded.toLocaleString();
  statDropped.textContent = String(stats.video.framesDropped);
  statQueue.textContent   = String(stats.video.decodeQueueSize);
  updateLatency();
  drawGraphs();

  if (current) {
    current.dropped = stats.video.framesDropped - current.droppedAtStart;
    if (settled(current) && stats.video.fps > 0) {
      current.samples += 1;
      current.fps += stats.video.fps;
      current.mbps += mbps;
      current.decodeMs += stats.decodeLatencyMs;
      current.rttMs += stats.rttMs;
    }
    renderLog();
  }
  liveNote.textContent = current && !settled(current) ? 'settling…' : '';
}

const mbps = (bps: number) => `${(bps / 1e6).toFixed(bps < 1e7 ? 1 : 0)} Mbps`;

function onServerStats(s: ServerStats) {
  lastServerStats = s;
  if (s.target_bitrate === null) {
    infoBitrate.textContent = 'Constant QP (not adapted)';
    infoBitrate.className = '';
  } else if (s.preset_bitrate !== null && s.target_bitrate < s.preset_bitrate) {
    infoBitrate.textContent = `${mbps(s.target_bitrate)} of ${mbps(s.preset_bitrate)} · limited by the network`;
    infoBitrate.className = 'warn';
  } else {
    infoBitrate.textContent = `${mbps(s.target_bitrate)} (preset)`;
    infoBitrate.className = '';
  }
  const perSec = 1000 / Math.max(1, s.window_ms);
  statEncMax.textContent  = fmtMs(s.encode_ms_max);
  statKeys.textContent    = (s.keyframes * perSec).toFixed(1);
  statFrameKb.textContent = s.frames > 0 ? `${(s.bytes / s.frames / 1024).toFixed(1)} KB` : '—';
  if (current && settled(current) && s.frames > 0) {
    current.encodeMs += s.encode_ms_avg;
    current.encodeSamples += 1;
  }
}

// ─── Server info & settings form ──────────────────────────────────────────────

let serverInfo: ServerInfo | null = null;
/** The latest network/adaptation status, which says whether Auto is on. */
let network: NetworkStatus | null = null;

function onNetwork(n: NetworkStatus) {
  const autoChanged = network?.auto_codec !== n.auto_codec;
  network = n;
  infoNet.textContent = `RTT ${n.rtt_ms.toFixed(0)} ms · queue ${n.queue_ms.toFixed(0)} ms · ` +
    `loss ${(n.loss * 100).toFixed(1)}% · ${mbps(n.delivered_bps)}` + (n.congested ? ' · congested' : '');
  infoNet.className = n.congested ? 'warn' : '';
  if (serverInfo) {
    infoCodec.textContent = `${codecName(serverInfo.codec)} ${serverInfo.codec_profile}` +
      (n.auto_codec ? ' · Auto' : '');
    infoCodec.title = (n.auto_codec && n.codec_reason) || '';
  }
  if (autoChanged && serverInfo) fillForm(serverInfo);
}

function onServerInfo(info: ServerInfo) {
  serverInfo = info;
  pendingApply = false;

  infoGpu.innerHTML      = `${escapeHtml(info.gpu)}${gpuVendorTag(info.gpu)}`;
  infoGpu.title          = info.gpu;
  infoDriver.textContent = info.gpu_driver;
  infoWrite.textContent  = info.write_path === 'direct'
    ? 'Direct (storage writes)'
    : 'Copy (per-plane images)';
  infoCodec.textContent  = `${codecName(info.codec)} ${info.codec_profile}`;
  infoRc.textContent     = info.rate_control;
  infoRc.title           = info.rate_control;
  infoStream.textContent = `${info.width}×${info.height} @ ${info.framerate} fps · in flight ${info.in_flight_frames}`;
  infoGop.textContent    = `${info.gop_size} frames · ${info.temporal_layers} temporal layer${info.temporal_layers === 1 ? '' : 's'}`;

  // GPU list.
  setGpu.innerHTML = '';
  for (const name of info.gpus) {
    const opt = document.createElement('option');
    opt.value = name;
    opt.textContent = name === info.gpu ? `${name} (current)` : name;
    setGpu.appendChild(opt);
  }
  setGpu.value = info.gpu;

  fillForm(info);
  startSegment(info, lastStats?.video.framesDropped ?? 0);
  settingsNote.textContent = '';
  if (expectingRestart) {
    expectingRestart = false;
    toast(`Now running on ${info.gpu}`);
  }
}

function ensureOption(sel: HTMLSelectElement, value: string, label: string) {
  if (![...sel.options].some((o) => o.value === value)) {
    const opt = document.createElement('option');
    opt.value = value;
    opt.textContent = label;
    sel.appendChild(opt);
  }
  sel.value = value;
}

function fillForm(info: ServerInfo) {
  // Only offer codecs this GPU can encode (older servers don't say).
  for (const opt of setCodec.options) {
    opt.disabled = opt.value !== 'auto' && !!info.codecs && !info.codecs.includes(opt.value as VideoCodecName);
    opt.title = opt.disabled ? `${info.gpu} can't encode this` : '';
  }
  setCodec.value  = network?.auto_codec ? 'auto' : info.codec;
  setPreset.value = String(info.preset);
  ensureOption(setRes, `${info.width}x${info.height}`, `${info.width}×${info.height}`);
  ensureOption(setFps, String(info.framerate), `${info.framerate} fps`);
  setGop.value = String(info.gop_size);
  ensureOption(setInFlight, String(info.in_flight_frames), String(info.in_flight_frames));
  updateFormState();
}

/** The form's settings, as far as they differ from the server's. */
function formChanges(): StreamParamsRequest {
  if (!serverInfo) return {};
  const req: StreamParamsRequest = {};
  const [w, h] = setRes.value.split('x').map(Number);
  const auto = network?.auto_codec ?? false;
  if (setCodec.value === 'auto') {
    if (!auto) req.auto_codec = true;
  } else if (auto || setCodec.value !== serverInfo.codec) {
    req.codec = setCodec.value as VideoCodecName;
    req.auto_codec = false;
  }
  if (Number(setPreset.value) !== serverInfo.preset) req.preset = Number(setPreset.value);
  if (w !== serverInfo.width || h !== serverInfo.height) { req.width = w; req.height = h; }
  if (Number(setFps.value) !== serverInfo.framerate) req.framerate = Number(setFps.value);
  const gop = Number(setGop.value);
  if (gop > 0 && gop !== serverInfo.gop_size) req.gop_size = gop;
  if (Number(setInFlight.value) !== serverInfo.in_flight_frames) req.in_flight_frames = Number(setInFlight.value);
  return req;
}

let pendingApply = false;

function updateFormState() {
  presetLabel.textContent = presetDescription(Number(setPreset.value));
  const connected = client?.getState() === ConnectionState.Connected && !!serverInfo;
  const changes = formChanges();
  const changed = Object.keys(changes).length > 0;

  setCodec.classList.toggle('changed', 'codec' in changes || 'auto_codec' in changes);
  setPreset.classList.toggle('changed', 'preset' in changes);
  setRes.classList.toggle('changed', 'width' in changes);
  setFps.classList.toggle('changed', 'framerate' in changes);
  setGop.classList.toggle('changed', 'gop_size' in changes);
  setInFlight.classList.toggle('changed', 'in_flight_frames' in changes);

  applyBtn.disabled  = !connected || !changed || pendingApply;
  revertBtn.disabled = !changed;
  applyHint.textContent = pendingApply
    ? 'Applying…'
    : 'width' in changes || 'framerate' in changes
      ? 'Resizes the app and rebuilds the encoder'
      : 'Rebuilds the encoder';

  const gpuChanged = !!serverInfo && setGpu.value !== serverInfo.gpu;
  switchGpuBtn.disabled = !connected || !gpuChanged || expectingRestart;
  setGpu.classList.toggle('changed', gpuChanged);
}

for (const el of [setCodec, setPreset, setRes, setFps, setGop, setInFlight, setGpu]) {
  el.addEventListener('input', updateFormState);
}

applyBtn.addEventListener('click', () => {
  const changes = formChanges();
  if (!client || Object.keys(changes).length === 0) return;
  pendingApply = true;
  settingsNote.textContent = 'applying…';
  client.setStreamParams(changes);
  updateFormState();
  // If the server rejects the change (e.g. unsupported codec), it re-sends
  // the old info, which clears this. Otherwise give up waiting eventually.
  setTimeout(() => {
    if (pendingApply) {
      pendingApply = false;
      settingsNote.textContent = '';
      toast('No response to the settings change');
      updateFormState();
    }
  }, 8000);
});

revertBtn.addEventListener('click', () => {
  if (serverInfo) {
    fillForm(serverInfo);
    setGpu.value = serverInfo.gpu;
    updateFormState();
  }
});

// ─── GPU switching (server restart) ───────────────────────────────────────────

let expectingRestart = false;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

switchGpuBtn.addEventListener('click', () => {
  if (!client || !serverInfo) return;
  const gpu = setGpu.value;
  expectingRestart = true;
  settingsNote.textContent = 'restarting…';
  toast(`Restarting on ${gpu}…`);
  client.switchGpu(gpu);
  updateFormState();
});

/** After a GPU switch the server restarts; keep trying until it's back. */
function scheduleReconnect(attempt: number) {
  if (reconnectTimer) clearTimeout(reconnectTimer);
  if (attempt > 30) {
    expectingRestart = false;
    settingsNote.textContent = '';
    toast('Server did not come back after the restart');
    return;
  }
  connNote.textContent = `server restarting… (${attempt})`;
  reconnectTimer = setTimeout(async () => {
    const ok = await connect();
    if (!ok && expectingRestart) scheduleReconnect(attempt + 1);
  }, attempt === 1 ? 1500 : 1000);
}

// ─── Gamepads ─────────────────────────────────────────────────────────────────

/** Button labels for the W3C "standard" mapping, by index. */
const STANDARD_BUTTONS = ['A', 'B', 'X', 'Y', 'LB', 'RB', 'LT', 'RT', 'View', 'Menu',
                          'LS', 'RS', '↑', '↓', '←', '→', 'Home'];

/** The server's view of each of our gamepads, by browser index. */
const padStatus = new Map<number, GamepadStatus>();
/** Pads other viewers have plugged in. */
let otherPads = 0;
/** Our viewer ID, from the viewer list; pad status is matched against it. */
let myViewerId: number | null = null;
/** When each pad's rumble indicator should go out. */
const padRumbleUntil = new Map<number, number>();
/** Event counts from the previous status, for a per-second rate. */
const padPrevEvents = new Map<number, { events: number; at: number }>();
const padRate = new Map<number, number>();

function onGamepadStatus(list: GamepadStatus[]) {
  const now = performance.now();
  padStatus.clear();
  otherPads = 0;
  for (const s of list) {
    if (s.viewer !== myViewerId) {
      otherPads++;
      continue;
    }
    padStatus.set(s.index, s);
    const prev = padPrevEvents.get(s.index);
    if (prev && now > prev.at) {
      padRate.set(s.index, ((s.events - prev.events) * 1000) / (now - prev.at));
    }
    padPrevEvents.set(s.index, { events: s.events, at: now });
  }
  renderPadInfo();
}

/** "Xbox Wireless Controller (STANDARD GAMEPAD Vendor: 045e Product: 0b13)" → parts. */
function parsePadId(id: string): { name: string; usb: string } {
  const m = id.match(/^(.*?)\s*\((?:.*?)Vendor:\s*([0-9a-f]{4})\s*Product:\s*([0-9a-f]{4})\)\s*$/i);
  if (m) return { name: m[1] || id, usb: `${m[2]}:${m[3]}` };
  // Firefox-style "045e-0b13-Xbox Wireless Controller".
  const f = id.match(/^([0-9a-f]{4})-([0-9a-f]{4})-(.*)$/i);
  if (f) return { name: f[3], usb: `${f[1]}:${f[2]}` };
  return { name: id, usb: '' };
}

interface PadCard {
  root: HTMLDivElement;
  server: HTMLSpanElement;
  browser: HTMLSpanElement;
  type: HTMLSelectElement;
  notice: HTMLDivElement;
  left: HTMLDivElement;
  right: HTMLDivElement;
  lt: HTMLDivElement;
  rt: HTMLDivElement;
  chips: HTMLSpanElement[];
}

const padCards = new Map<number, PadCard>();

/** The emulation choice for a controller model (by browser pad ID). */
function loadPadType(padId: string): GamepadType {
  try {
    const v = localStorage.getItem(`veil.padType.${padId}`);
    if (v === 'xbox' || v === 'playstation') return v;
  } catch { /* storage unavailable */ }
  return 'auto';
}

function savePadType(padId: string, type: GamepadType) {
  try {
    if (type === 'auto') localStorage.removeItem(`veil.padType.${padId}`);
    else localStorage.setItem(`veil.padType.${padId}`, type);
  } catch { /* storage unavailable */ }
}

function makeCard(pad: Gamepad): PadCard {
  const { name, usb } = parsePadId(pad.id);
  const root = document.createElement('div');
  root.className = 'pad-card';
  root.innerHTML = `
    <div class="pad-head">
      <span class="pad-index">#${pad.index}</span>
      <span class="pad-name" title="${escapeHtml(pad.id)}">${escapeHtml(name)}</span>
      <span class="pad-rumble" title="The game is rumbling this pad">Rumble</span>
    </div>
    <div class="pad-row"><span class="k">Browser</span><span class="v" data-browser></span></div>
    <div class="pad-row"><span class="k">Emulate</span>
      <select data-type title="Which controller games see. Changing it replugs the pad.">
        <option value="auto">Auto (${detectGamepadLayout(pad.id) === 'playstation' ? 'DualSense' : 'Xbox 360'})</option>
        <option value="xbox">Xbox 360</option>
        <option value="playstation">DualSense</option>
      </select>
    </div>
    <div class="pad-row"><span class="k">Server</span><span class="v" data-server></span></div>
    <div class="pad-row pad-notice" data-notice style="display:none"></div>
    <div class="pad-viz">
      <div class="stick" data-left><div class="knob"></div></div>
      <div class="stick" data-right><div class="knob"></div></div>
      <div class="triggers">
        <div class="trigger">LT <div class="bar"><div class="fill" data-lt></div></div></div>
        <div class="trigger">RT <div class="bar"><div class="fill" data-rt></div></div></div>
      </div>
      <div class="buttons" data-buttons></div>
    </div>`;
  const q = <T extends HTMLElement>(sel: string) => root.querySelector(sel) as T;
  const buttons = q<HTMLDivElement>('[data-buttons]');
  const standard = pad.mapping === 'standard';
  const chips = pad.buttons.map((_, i) => {
    const chip = document.createElement('span');
    chip.className = 'btn-chip';
    chip.textContent = standard ? (STANDARD_BUTTONS[i] ?? String(i)) : String(i);
    buttons.appendChild(chip);
    return chip;
  });
  const card: PadCard = {
    root,
    browser: q('[data-browser]'),
    server: q('[data-server]'),
    type: q('[data-type]'),
    notice: q('[data-notice]'),
    left: q('[data-left]'),
    right: q('[data-right]'),
    lt: q('[data-lt]'),
    rt: q('[data-rt]'),
    chips,
  };
  card.type.value = loadPadType(pad.id);
  client?.setGamepadType(pad.index, card.type.value as GamepadType);
  card.type.addEventListener('change', () => {
    const type = card.type.value as GamepadType;
    savePadType(pad.id, type);
    client?.setGamepadType(pad.index, type);
  });
  card.browser.textContent = [
    usb && `USB ${usb}`,
    standard ? 'standard mapping' : 'non-standard mapping (buttons may be mislabelled)',
    `${pad.buttons.length} buttons, ${pad.axes.length} axes`,
  ].filter(Boolean).join(' · ');
  return card;
}

/** Update the server line of each card (on status messages and connect/disconnect). */
function renderPadInfo() {
  const connected = client?.getState() === ConnectionState.Connected;
  for (const [index, card] of padCards) {
    const s = padStatus.get(index);
    if (!connected) {
      card.server.className = 'v';
      card.server.textContent = 'Not connected';
    } else if (!s) {
      card.server.className = 'v pad-wait';
      card.server.textContent = 'Waiting for the virtual device…';
    } else if (s.error) {
      card.server.className = 'v pad-err';
      card.server.textContent = `Virtual device failed: ${s.error}`;
    } else {
      const rate = padRate.get(index);
      card.server.className = 'v pad-ok';
      card.server.textContent = `${s.emulates} at ${s.device ?? 'unknown node'} · ` +
        `${s.events.toLocaleString()} events` + (rate !== undefined ? ` (${rate.toFixed(0)}/s)` : '');
    }
    card.notice.textContent = (connected && s?.notice) || '';
    card.notice.style.display = card.notice.textContent ? '' : 'none';
  }
  const failed = [...padStatus.values()].filter((s) => s.error).length;
  padNote.textContent = [
    failed && `${failed} failed on server`,
    otherPads && `${otherPads} from other viewers`,
  ].filter(Boolean).join(' · ');
  padNote.style.color = failed ? 'var(--bad)' : '';
}

function placeKnob(stick: HTMLDivElement, x: number, y: number) {
  const knob = stick.firstElementChild as HTMLDivElement;
  // 44px stick, 12px knob → 16px of travel from centre.
  knob.style.transform = `translate(${(x || 0) * 16}px, ${(y || 0) * 16}px)`;
}

/** Poll the browser's gamepads and redraw the live state. */
function renderPads() {
  const pads = [...(navigator.getGamepads?.() ?? [])].filter((p): p is Gamepad => !!p);
  const seen = new Set(pads.map((p) => p.index));

  for (const [index, card] of padCards) {
    if (!seen.has(index)) {
      card.root.remove();
      padCards.delete(index);
    }
  }
  for (const pad of pads) {
    let card = padCards.get(pad.index);
    if (!card) {
      card = makeCard(pad);
      padCards.set(pad.index, card);
      padList.appendChild(card.root);
      renderPadInfo();
    }
    const ax = pad.axes;
    placeKnob(card.left, ax[0] ?? 0, ax[1] ?? 0);
    placeKnob(card.right, ax[2] ?? 0, ax[3] ?? 0);
    card.left.classList.toggle('pressed', !!pad.buttons[10]?.pressed);
    card.right.classList.toggle('pressed', !!pad.buttons[11]?.pressed);
    card.lt.style.width = `${(pad.buttons[6]?.value ?? 0) * 100}%`;
    card.rt.style.width = `${(pad.buttons[7]?.value ?? 0) * 100}%`;
    pad.buttons.forEach((b, i) => card!.chips[i]?.classList.toggle('on', b.pressed));
    card.root.classList.toggle('rumbling', (padRumbleUntil.get(pad.index) ?? 0) > performance.now());
  }
  padEmpty.style.display = pads.length ? 'none' : '';
  requestAnimationFrame(renderPads);
}

window.addEventListener('gamepadconnected', () => renderPadInfo());

// ─── Sound ────────────────────────────────────────────────────────────────────

/** One player for every connection, so sound survives reconnects. */
const audioPlayer = new AudioPlayer();
let soundMuted = false;
let lastAudioPackets = 0;

function loadVolume(): number {
  try {
    const raw = localStorage.getItem('veil.volume');
    const v = Number(raw);
    return raw !== null && Number.isFinite(v) ? v : 100;
  } catch {
    return 100;
  }
}

function saveVolume(v: number) {
  try { localStorage.setItem('veil.volume', String(v)); } catch { /* storage unavailable */ }
}

/** Why sound last failed to start, if it did (e.g. the browser lacks WebCodecs audio). */
let soundError: string | null = null;

async function enableSound() {
  try {
    await audioPlayer.enable();
    soundError = null;
  } catch (err) {
    soundError = err instanceof Error ? err.message : String(err);
    toast(`Couldn't start sound: ${soundError}`);
  }
  updateSoundUi();
}

function updateSoundUi() {
  const on = audioPlayer.enabled;
  soundEnable.style.display = on ? 'none' : '';
  soundMute.style.display = on ? '' : 'none';
  soundMute.textContent = soundMuted ? 'Unmute' : 'Mute';
  // The stylesheet hides it by default, so show it explicitly.
  soundOverlay.style.display = !on && client?.getState() === ConnectionState.Connected ? 'block' : 'none';
  soundNote.textContent = on ? (soundMuted ? 'muted' : 'playing') : soundError ? 'failed to start' : 'off';
  soundNote.title = !on && soundError ? soundError : '';
}

function onAudioStats(stats: ConnectionStats) {
  const p = stats.audio.player;
  const rate = stats.audio.framesDecoded - lastAudioPackets;
  lastAudioPackets = stats.audio.framesDecoded;
  soundRate.textContent = String(rate);
  soundBitrate.textContent = stats.audio.bitrateBps > 0 ? `${Math.round(stats.audio.bitrateBps / 1000)} kbps` : '—';
  soundBuffered.textContent = p.enabled ? `${Math.round(p.bufferedMs)} ms` : '—';
  soundUnderruns.textContent = p.enabled ? String(p.underruns) : '—';
  soundTrims.textContent = p.enabled ? String(p.trims) : '—';
  soundLatency.textContent = p.enabled && p.outputLatencyMs > 0 ? `${Math.round(p.outputLatencyMs)} ms` : '—';
  updateSoundUi();
}

soundEnable.addEventListener('click', () => { void enableSound(); });
soundOverlay.addEventListener('click', (e) => { e.stopPropagation(); void enableSound(); });
soundMute.addEventListener('click', () => {
  soundMuted = !soundMuted;
  audioPlayer.setMuted(soundMuted);
  updateSoundUi();
});
soundVolume.addEventListener('input', () => {
  const v = Number(soundVolume.value);
  soundVolumeLabel.textContent = `${v}%`;
  audioPlayer.setVolume(v / 100);
  saveVolume(v);
});
{
  const v = loadVolume();
  soundVolume.value = String(v);
  soundVolumeLabel.textContent = `${v}%`;
  audioPlayer.setVolume(v / 100);
}

// ─── Viewers ──────────────────────────────────────────────────────────────────

function fmtDuration(secs: number): string {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

function onViewers(list: ViewerList) {
  myViewerId = list.you;
  const n = list.viewers.length;
  const controlling = list.viewers.filter((v) => v.active).length;
  const remote = list.viewers.filter((v) => v.network === 'internet').length;
  viewersEl.style.display = '';
  viewersSummary.innerHTML = `<b>${n}</b> viewer${n === 1 ? '' : 's'} connected` +
    (remote ? ` · ${remote} from the internet` : '') +
    (controlling ? ` · <b>${controlling}</b> controlling` : '');
  viewerList.innerHTML = list.viewers.map((v) => {
    const you = v.id === list.you;
    const cls = ['viewer', you && 'you', v.active && 'active'].filter(Boolean).join(' ');
    const title = `${v.network === 'lan' ? 'Local network' : 'Internet'}, connected ${fmtDuration(v.connected_secs)}` +
      (v.active ? ', sending input' : '');
    return `<span class="${cls}" title="${title}">${you ? 'You' : `#${v.id}`} ` +
      `<small>${v.network === 'lan' ? 'LAN' : 'net'} · ${fmtDuration(v.connected_secs)}</small></span>`;
  }).join('');
  panelToggle.textContent = `Panel ⇥ · 👁 ${n}`;
}

function resetViewers() {
  viewersEl.style.display = 'none';
  viewerList.innerHTML = '';
  panelToggle.textContent = 'Panel ⇥';
}

// ─── Compositor ───────────────────────────────────────────────────────────────

function onCompositorState(st: CompositorState) {
  compDisplay.textContent = `${st.width}×${st.height} @ ${st.framerate} fps`;
  const appState = st.app_exit
    ? `<span style="color:var(--warn)">exited (${escapeHtml(st.app_exit)})</span>`
    : '<span style="color:var(--good)">running</span>';
  compApp.innerHTML = `${escapeHtml(st.app_command)} · pid ${st.app_pid ?? '?'} · ${appState}`;
  compApp.title = st.app_exit
    ? 'The launched process exited. Launchers like Steam do this and leave their children running.'
    : st.app_command;
  compSockets.textContent = `WAYLAND_DISPLAY=${st.wayland_socket}` +
    (st.x11_display ? ` · DISPLAY=${st.x11_display}` : ' · no Xwayland');
  const mapped = st.surfaces.filter((s) => s.mapped).length;
  compClients.textContent = `${st.clients} Wayland client${st.clients === 1 ? '' : 's'} · ` +
    `${mapped}/${st.surfaces.length} windows mapped · ${st.other_surfaces} other surfaces`;

  // A window behind a fullscreen one (e.g. Steam behind a game) is normal,
  // so that's a note, not an issue.
  const isNote = (s: CompositorState['surfaces'][number]) => s.mapped && !s.composited;
  const problems = st.surfaces.filter((s) => s.problem && !isNote(s)).length +
                   st.x11_windows.filter((w) => w.problem).length;
  compNote.textContent = problems ? `${problems} issue${problems === 1 ? '' : 's'}` : '';
  compNote.style.color = problems ? 'var(--warn)' : '';

  compWindows.innerHTML = '';
  if (st.surfaces.length === 0) {
    compWindows.innerHTML = '<div class="comp-empty">No windows yet — the app hasn\'t created one.</div>';
  }
  for (const s of st.surfaces) {
    const card = document.createElement('div');
    const notMapped = !s.mapped;
    card.className = 'win-card' + (notMapped ? ' bad' : s.problem && !isNote(s) ? ' problem' : '');
    const name = s.title || s.app_id || `surface ${s.key}`;
    const state = s.composited
      ? '<span class="chip good">IN VIDEO</span>'
      : s.mapped ? '<span class="chip warn">HIDDEN</span>' : '<span class="chip bad">NOT MAPPED</span>';
    const kind = s.kind === 'x11' ? '<span class="chip x11">X11</span>' : '<span class="chip wl">WAYLAND</span>';
    const flags = [s.active && 'ACTIVE', s.fullscreen && 'FULLSCREEN']
      .filter(Boolean).map((f) => `<span class="chip">${f}</span>`).join('');
    const meta = [
      s.stack_index !== null ? `stack <b>#${s.stack_index}</b>` : null,
      s.configured ? `configured <b>${s.configured[0]}×${s.configured[1]}</b>` : 'not configured',
      s.buffer ? `buffer <b>${s.buffer.width}×${s.buffer.height}</b> ${s.buffer.kind}` +
                 (s.buffer.kind === 'shm' ? ' (CPU copy)' : '') : 'no buffer',
      `<b>${s.commits_per_sec.toFixed(0)}</b> commits/s`,
      s.app_id && s.title ? `app_id <b>${escapeHtml(s.app_id)}</b>` : null,
      s.x11_window !== null ? `X11 0x${s.x11_window.toString(16)}` : null,
    ].filter(Boolean).join(' · ');
    card.innerHTML = `
      <div class="win-head">
        <span class="win-title" title="${escapeHtml(name)}">${escapeHtml(name)}</span>
        ${kind}${flags}${state}
      </div>
      <div class="win-meta">${meta}</div>
      ${s.problem ? `<div class="win-problem"${isNote(s) ? ' style="color:var(--dim)"' : ''}>${escapeHtml(s.problem)}</div>` : ''}`;
    if (s.window_id !== null && !s.active) {
      const btn = document.createElement('button');
      btn.className = 'btn btn-ghost win-raise';
      btn.textContent = 'Raise';
      btn.title = 'Bring this window to the top and focus it';
      const wid = s.window_id;
      btn.addEventListener('click', () => client?.focusWindow(wid));
      card.querySelector('.win-head')!.appendChild(btn);
    }
    compWindows.appendChild(card);
  }

  compX11Wrap.style.display = st.x11_display ? '' : 'none';
  const x11Problems = st.x11_windows.filter((w) => w.problem).length;
  compX11Summary.textContent = `X11 windows (${st.x11_windows.length}` +
    (x11Problems ? `, ${x11Problems} with issues` : '') + ')';
  if (x11Problems) compX11Wrap.open = true;
  compX11.innerHTML = st.x11_windows.map((w) => {
    const flags = [w.mapped ? 'mapped' : 'unmapped', w.override_redirect && 'override-redirect',
                   w.fullscreen && 'fullscreen', w.has_surface ? 'linked' : 'no surface']
      .filter(Boolean).join(', ');
    const [x, y, ww, hh] = w.geometry;
    return `<div class="x11-row"><b>0x${w.id.toString(16)}</b> ${escapeHtml(w.title || w.class || '(untitled)')}` +
      `${w.class && w.title ? ` <span style="color:var(--dim)">(${escapeHtml(w.class)})</span>` : ''}` +
      `<br><span style="color:var(--dim)">${ww}×${hh}+${x}+${y} · ${flags}</span>` +
      `${w.problem ? `<br><span class="p">${escapeHtml(w.problem)}</span>` : ''}</div>`;
  }).join('');
}

function resetCompositor() {
  for (const el of [compDisplay, compApp, compSockets, compClients]) el.textContent = '—';
  compNote.textContent = '';
  compWindows.innerHTML = '';
  compX11.innerHTML = '';
}

// ─── Mouse capture ────────────────────────────────────────────────────────────

/** Fallback arrow for named cursors, which we can't draw from a CSS name. */
const ARROW_CURSOR = 'data:image/svg+xml,' + encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="24" viewBox="0 0 20 24">' +
  '<path d="M1 1v19l5-5 3.5 7.5 3-1.4L9 13.7h7z" fill="#fff" stroke="#000" stroke-width="1.2" stroke-linejoin="round"/></svg>');

let cursorImage = ARROW_CURSOR;
let cursorHotspot = { x: 1, y: 1 };
let cursorHidden = false;
let captureState = { captured: false, relative: false, appWantsLock: false };
let badgeTimer: ReturnType<typeof setTimeout> | null = null;

/** macOS shows Control+Option, not Ctrl+Alt (the same keys). */
const IS_MAC = /mac/i.test(
  (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData?.platform
  || navigator.userAgent);

/** The Ctrl+Alt+`key` shortcut, as this OS writes it. */
function combo(key: string): string {
  return IS_MAC
    ? `<kbd title="Control+Option+${key}">⌃</kbd><kbd>⌥</kbd><kbd>${key}</kbd>`
    : `<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>${key}</kbd>`;
}

/** How to release the mouse, and what Esc does, right now. */
function releaseHint(): string {
  if (!document.fullscreenElement) {
    return `${combo('M')} to release · ${combo('F')} for fullscreen, where Esc goes to the app`;
  }
  return client?.escapeGoesToApp
    ? `${combo('M')} to release · Esc goes to the app (hold Esc to leave fullscreen)`
    : `${combo('M')} or Esc to release (this browser won't pass Esc to the app)`;
}

function showBadge(html: string, opts: { sticky?: boolean; want?: boolean } = {}) {
  captureBadge.innerHTML = html;
  captureBadge.classList.toggle('want', !!opts.want);
  captureBadge.classList.add('show');
  if (badgeTimer) clearTimeout(badgeTimer);
  badgeTimer = opts.sticky ? null : setTimeout(() => captureBadge.classList.remove('show'), 3500);
}

function onMouseCapture(state: { captured: boolean; relative: boolean; appWantsLock: boolean }) {
  const was = captureState;
  captureState = state;
  if (state.captured) {
    if (!was.captured || was.relative !== state.relative) {
      showBadge(state.relative
        ? `<b>Mouse captured</b> · relative (app has locked the pointer) — ${releaseHint()}`
        : `<b>Mouse captured</b> · absolute — ${releaseHint()}`);
    }
  } else if (state.appWantsLock) {
    showBadge(`<b>The app wants the mouse</b> — click the stream or press ${combo('M')}`,
              { sticky: true, want: true });
  } else if (was.captured || was.appWantsLock) {
    showBadge(`Mouse released — click the stream or press ${combo('M')} to capture`);
  }
  canvasWrap.classList.toggle('captured', state.captured);
}

/** Draw the app's cursor at the virtual position while captured and absolute. */
function renderCursor() {
  const show = !!client && captureState.captured && !captureState.relative && !cursorHidden;
  vcursor.style.display = show ? 'block' : 'none';
  if (show && client) {
    const pos = client.getCursorPosition();
    const rect = canvas.getBoundingClientRect();
    const wrap = canvasWrap.getBoundingClientRect();
    const sx = rect.width / canvas.width;
    const sy = rect.height / canvas.height;
    const left = rect.left - wrap.left + pos.x * sx - cursorHotspot.x;
    const top  = rect.top  - wrap.top  + pos.y * sy - cursorHotspot.y;
    vcursor.style.transform = `translate(${left}px, ${top}px)`;
  }
  requestAnimationFrame(renderCursor);
}

document.addEventListener('pointerlockerror', () => {
  showBadge('<b>Couldn’t capture the mouse</b> — the browser refused (is this window focused?)', { want: true });
});

/** Fullscreen the stream, where Esc goes to the app (if the browser allows). */
async function toggleFullscreen() {
  try {
    if (document.fullscreenElement) {
      await (client ? client.exitFullscreen() : document.exitFullscreen());
    } else if (client) {
      const escLocked = await client.enterFullscreen(canvasWrap);
      showBadge(escLocked
        ? `<b>Fullscreen</b> — Esc goes to the app · click the stream to capture the mouse · hold Esc to leave`
        : `<b>Fullscreen</b> — this browser won't pass Esc to the app, so Esc releases the mouse`);
    } else {
      await canvasWrap.requestFullscreen();
    }
  } catch (err) {
    showBadge(`<b>Couldn't go fullscreen</b> — ${err instanceof Error ? err.message : err}`, { want: true });
  }
}

fullscreenBtn.title = `Fullscreen (${IS_MAC ? '⌃⌥F' : 'Ctrl+Alt+F'}): Esc goes to the app, and holding Esc leaves`;
fullscreenBtn.addEventListener('click', () => void toggleFullscreen());
document.addEventListener('fullscreenchange', () => {
  const full = document.fullscreenElement !== null;
  fullscreenBtn.textContent = full ? '⛶ Exit fullscreen' : '⛶ Fullscreen';
  // Left with a long Esc press: let go of the keyboard too.
  if (!full) void client?.exitFullscreen();
});

// Ctrl+Alt+M toggles capture and Ctrl+Alt+F fullscreen (Control+Option on a
// Mac). Handled before the SDK's key handler, so the keys aren't sent to the
// app.
document.addEventListener('keydown', (e) => {
  if (!e.ctrlKey || !e.altKey || e.shiftKey || e.metaKey) return;
  if (e.code === 'KeyM') {
    e.preventDefault();
    e.stopPropagation();
    if (!client) return;
    if (client.isMouseCaptured()) client.releaseMouse();
    else client.captureMouse();
  } else if (e.code === 'KeyF') {
    e.preventDefault();
    e.stopPropagation();
    void toggleFullscreen();
  }
}, { capture: true });

// ─── Connection ───────────────────────────────────────────────────────────────

const STATE_LABEL: Record<ConnectionState, string> = {
  [ConnectionState.Disconnected]: 'Disconnected',
  [ConnectionState.Connecting]:   'Connecting…',
  [ConnectionState.Connected]:    'Streaming',
  [ConnectionState.Failed]:       'Failed',
  [ConnectionState.Closed]:       'Closed',
};

let client: VeilClient | null = null;

function setStatus(state: ConnectionState) {
  const connected = state === ConnectionState.Connected;
  const idle = state === ConnectionState.Disconnected
            || state === ConnectionState.Closed
            || state === ConnectionState.Failed;

  overlay.classList.toggle('hidden', connected);
  overlayStatus.textContent = expectingRestart ? 'Restarting on another GPU…' : STATE_LABEL[state];
  connDot.className = `dot ${state}`;
  connState.textContent = STATE_LABEL[state];
  if (!expectingRestart) connNote.textContent = '';

  connectBtn.disabled = !idle;
  disconnBtn.disabled = idle;

  if (idle) {
    resetViewers();
    lastAudioPackets = 0;
    captureState = { captured: false, relative: false, appWantsLock: false };
    captureBadge.classList.remove('show');
    resetLive();
    current = null;
    renderLog();
    padStatus.clear();
    padRumbleUntil.clear();
    myViewerId = null;
    network = null;
    infoNet.textContent = '—';
    infoNet.className = '';
    padPrevEvents.clear();
    padRate.clear();
  }
  renderPadInfo();
  updateSoundUi();
  if (connected) {
    connNote.textContent = (() => { try { return new URL(serverUrl.value).host; } catch { return ''; } })();
    canvas.focus();
  }
  updateFormState();
}

/** Connect to the server; resolves to whether it worked. */
async function connect(): Promise<boolean> {
  if (client) return true;
  const url = serverUrl.value.trim();

  const c = new VeilClient({
    serverUrl:       url,
    videoCodec:      'h264',
    width:           1920,
    height:          1080,
    framerate:       60,
    captureKeyboard: true,
    captureMouse:    true,
    captureGamepad:  true,
    // Click the stream to capture the mouse.
    pointerLock:     true,
    audioPlayer,
    debug:           false,
  });
  client = c;
  for (const [index, card] of padCards) c.setGamepadType(index, card.type.value as GamepadType);

  c.on('statechange', (state) => {
    if (client !== c) return;
    setStatus(state);
    if (state === ConnectionState.Closed || state === ConnectionState.Failed) {
      client = null;
      if (expectingRestart) scheduleReconnect(1);
    }
  });
  c.on('stats',       (s) => client === c && onStats(s));
  c.on('serverinfo',  (i) => client === c && onServerInfo(i));
  c.on('serverstats', (s) => client === c && onServerStats(s));
  c.on('gamepadstatus', (s) => client === c && onGamepadStatus(s));
  c.on('gamepadrumble', (r) => {
    if (client !== c) return;
    const off = r.strong === 0 && r.weak === 0;
    padRumbleUntil.set(r.index, off ? 0 : performance.now() + r.durationMs);
  });
  c.on('error',       (err) => console.error('[veil]', err.message));

  c.on('mousecapture', (s) => client === c && onMouseCapture(s));

  c.on('cursor', (cursor) => {
    cursorHidden = false;
    if (cursor.image) {
      canvas.style.cursor = `url(${cursor.image}) ${cursor.hotspotX} ${cursor.hotspotY}, auto`;
      cursorImage = cursor.image;
      cursorHotspot = { x: cursor.hotspotX, y: cursor.hotspotY };
    } else if (cursor.icon) {
      canvas.style.cursor = cursor.icon;
      cursorImage = ARROW_CURSOR;
      cursorHotspot = { x: 1, y: 1 };
    } else {
      canvas.style.cursor = 'none';
      cursorHidden = true;
    }
    if (vcursor.src !== cursorImage) vcursor.src = cursorImage;
  });

  c.on('compositorstate', (st) => client === c && onCompositorState(st));
  c.on('viewers', (list) => client === c && onViewers(list));
  c.on('network', (n) => client === c && onNetwork(n));

  setStatus(ConnectionState.Connecting);
  try {
    await c.connect(canvas);
    return true;
  } catch (err) {
    console.error('[veil] connect failed:', err);
    if (client === c) client = null;
    setStatus(ConnectionState.Failed);
    // Show the browser's own reason (e.g. no WebTransport, handshake failed).
    const reason = err instanceof Error ? err.message : String(err);
    overlayStatus.textContent = `Couldn't connect: ${reason}`;
    connNote.textContent = reason;
    connNote.title = reason;
    return false;
  }
}

async function disconnect() {
  expectingRestart = false;
  if (reconnectTimer) clearTimeout(reconnectTimer);
  const c = client;
  client = null;
  await c?.disconnect();
  canvas.style.cursor = '';
  resetCompositor();
  setStatus(ConnectionState.Disconnected);
  canvas.getContext('2d')?.clearRect(0, 0, canvas.width, canvas.height);
}

connectBtn.addEventListener('click', () => { void connect(); });
disconnBtn.addEventListener('click', () => { void disconnect(); });
serverUrl.addEventListener('keydown', (e) => { if (e.key === 'Enter') void connect(); });
window.addEventListener('resize', drawGraphs);

// ─── Init ─────────────────────────────────────────────────────────────────────

setStatus(ConnectionState.Disconnected);
renderLog();
requestAnimationFrame(renderPads);
vcursor.src = cursorImage;
requestAnimationFrame(renderCursor);
setTimeout(() => { void connect(); }, 300);
