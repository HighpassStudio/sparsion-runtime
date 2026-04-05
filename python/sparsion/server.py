"""Sparsion Web Dashboard — FastAPI server with Chart.js frontend.

Usage:
    sparsion serve              # localhost:8765
    sparsion serve --port 9000
"""

from __future__ import annotations

import os

try:
    from fastapi import FastAPI, Query
    from fastapi.responses import HTMLResponse, JSONResponse
    import uvicorn
except ImportError:
    raise ImportError(
        "fastapi and uvicorn are required for the web dashboard. "
        "Install with: pip install fastapi uvicorn"
    )

from sparsion import Runtime

DEFAULT_DB = os.path.expanduser("~/.sparsion/memory.db")

app = FastAPI(title="Sparsion Dashboard")

_runtime: Runtime | None = None


def get_runtime() -> Runtime:
    global _runtime
    if _runtime is None:
        db = os.environ.get("SPARSION_DB", DEFAULT_DB)
        policy = os.environ.get("SPARSION_POLICY", None)
        _runtime = Runtime(db, policy=policy)
    return _runtime


@app.get("/", response_class=HTMLResponse)
def dashboard():
    return DASHBOARD_HTML


@app.get("/api/inspect")
def api_inspect():
    return get_runtime().inspect()


@app.get("/api/query")
def api_query(
    text: str = Query(None),
    tier: str = Query(None),
    source: str = Query(None),
    limit: int = Query(50),
):
    return get_runtime().query(
        text=text, tier=tier, source=source, limit=limit,
    )


@app.get("/api/sweep")
def api_sweep():
    return get_runtime().sweep()


@app.get("/api/snapshots")
def api_snapshots(limit: int = Query(100)):
    return get_runtime().get_snapshots(limit)


@app.get("/api/distribution")
def api_distribution():
    return get_runtime().get_salience_distribution()


def start_server(db_path: str = None, policy: str = None, port: int = 8765):
    if db_path:
        os.environ["SPARSION_DB"] = db_path
    if policy:
        os.environ["SPARSION_POLICY"] = policy
    print(f"Sparsion Dashboard: http://localhost:{port}")
    uvicorn.run(app, host="0.0.0.0", port=port, log_level="warning")


DASHBOARD_HTML = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Sparsion Dashboard</title>
<script src="https://cdn.jsdelivr.net/npm/chart.js@4"></script>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #0d1117; color: #c9d1d9; }
.header { padding: 20px 30px; border-bottom: 1px solid #21262d; display: flex; justify-content: space-between; align-items: center; }
.header h1 { font-size: 20px; font-weight: 600; }
.header .stats { font-size: 14px; color: #8b949e; }
.top-row { display: flex; gap: 16px; padding: 20px 30px 0; }
.top-row .card { flex: 1; }
.bottom-section { padding: 16px 30px 20px; display: flex; flex-direction: column; gap: 16px; }
.card { background: #161b22; border: 1px solid #21262d; border-radius: 8px; padding: 20px; }
.card h2 { font-size: 14px; color: #8b949e; margin-bottom: 12px; text-transform: uppercase; letter-spacing: 0.5px; }
.chart-container { position: relative; height: 220px; }
.btn { background: #21262d; color: #c9d1d9; border: 1px solid #30363d; padding: 6px 14px; border-radius: 6px; cursor: pointer; font-size: 13px; }
.btn:hover { background: #30363d; }
.btn.active { background: #1f6feb; border-color: #1f6feb; }
.controls { display: flex; gap: 8px; margin-bottom: 12px; flex-wrap: wrap; }
input[type="text"] { background: #0d1117; color: #c9d1d9; border: 1px solid #30363d; padding: 6px 12px; border-radius: 6px; font-size: 13px; width: 200px; }
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th { text-align: left; padding: 8px 12px; border-bottom: 1px solid #21262d; color: #8b949e; font-weight: 500; }
td { padding: 8px 12px; border-bottom: 1px solid #161b22; }
tr:hover td { background: #1c2128; }
.tier-hot { color: #f85149; }
.tier-warm { color: #d29922; }
.tier-cold { color: #58a6ff; }
.tier-forgotten { color: #484f58; }
.overridden { text-decoration: line-through; color: #484f58; }
.salience { font-family: monospace; }
.age { color: #8b949e; font-size: 12px; }
</style>
</head>
<body>

<div class="header">
  <h1>Sparsion Dashboard</h1>
  <div class="stats" id="stats">Loading...</div>
</div>

<div class="top-row">
  <div class="card">
    <h2>Tier Distribution</h2>
    <div class="chart-container"><canvas id="tierChart"></canvas></div>
  </div>
  <div class="card">
    <h2>Salience Distribution</h2>
    <div class="chart-container"><canvas id="salienceChart"></canvas></div>
  </div>
</div>

<div class="bottom-section">
  <div class="card">
    <h2>Tier Timeline</h2>
    <div class="chart-container" style="height: 180px;"><canvas id="timelineChart"></canvas></div>
  </div>

  <div class="card">
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
      <h2 style="margin: 0;">Memories</h2>
      <button class="btn" onclick="runSweep()">Run Sweep</button>
    </div>
    <div class="controls">
      <input type="text" id="searchInput" placeholder="Search memories..." oninput="loadMemories()">
      <button class="btn active" data-tier="" onclick="setTier(this, '')">All</button>
      <button class="btn" data-tier="hot" onclick="setTier(this, 'hot')">Hot</button>
      <button class="btn" data-tier="warm" onclick="setTier(this, 'warm')">Warm</button>
      <button class="btn" data-tier="cold" onclick="setTier(this, 'cold')">Cold</button>
    </div>
    <div style="max-height: 400px; overflow-y: auto;">
    <table>
      <thead>
        <tr>
          <th style="width: 60px;">Tier</th>
          <th style="width: 80px;">Salience</th>
          <th>Content</th>
          <th style="width: 70px;">Source</th>
          <th style="width: 50px;">Occ</th>
          <th style="width: 80px;">Age</th>
        </tr>
      </thead>
      <tbody id="memoryTable"></tbody>
    </table>
    </div>
  </div>
</div>

<script>
let currentTier = '';
let tierChart, salienceChart, timelineChart;

const COLORS = {
  hot: '#f85149',
  warm: '#d29922',
  cold: '#58a6ff',
  forgotten: '#484f58'
};

async function api(path) {
  const r = await fetch(path);
  return r.json();
}

function formatAge(ts) {
  const ms = Date.now() - new Date(ts).getTime();
  const mins = Math.floor(ms / 60000);
  if (mins < 60) return mins + 'm';
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return hrs + 'h';
  const days = Math.floor(hrs / 24);
  if (days < 7) return days + 'd';
  return Math.floor(days / 7) + 'w';
}

async function loadInspect() {
  const d = await api('/api/inspect');
  document.getElementById('stats').textContent =
    `${d.total_events} events | ${d.hot} hot | ${d.warm} warm | ${d.cold} cold | ${d.forgotten} forgotten`;

  const data = [d.hot, d.warm, d.cold, d.forgotten];
  if (tierChart) {
    tierChart.data.datasets[0].data = data;
    tierChart.update();
  } else {
    tierChart = new Chart(document.getElementById('tierChart'), {
      type: 'doughnut',
      data: {
        labels: ['Hot', 'Warm', 'Cold', 'Forgotten'],
        datasets: [{
          data: data,
          backgroundColor: [COLORS.hot, COLORS.warm, COLORS.cold, COLORS.forgotten],
          borderWidth: 0
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: { legend: { position: 'right', labels: { color: '#8b949e', padding: 12 } } }
      }
    });
  }
}

async function loadDistribution() {
  const values = await api('/api/distribution');
  if (!values.length) return;

  // Bucket into histogram
  const maxVal = Math.max(...values, 1);
  const bucketCount = 20;
  const bucketSize = maxVal / bucketCount;
  const buckets = new Array(bucketCount).fill(0);
  const labels = [];

  for (let i = 0; i < bucketCount; i++) {
    labels.push((i * bucketSize).toFixed(1));
  }

  for (const v of values) {
    const idx = Math.min(Math.floor(v / bucketSize), bucketCount - 1);
    buckets[idx]++;
  }

  if (salienceChart) {
    salienceChart.data.labels = labels;
    salienceChart.data.datasets[0].data = buckets;
    salienceChart.update();
  } else {
    salienceChart = new Chart(document.getElementById('salienceChart'), {
      type: 'bar',
      data: {
        labels: labels,
        datasets: [{
          label: 'Count',
          data: buckets,
          backgroundColor: '#58a6ff44',
          borderColor: '#58a6ff',
          borderWidth: 1
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: {
          x: { title: { display: true, text: 'Salience', color: '#8b949e' }, ticks: { color: '#8b949e' }, grid: { color: '#21262d' } },
          y: { title: { display: true, text: 'Count', color: '#8b949e' }, ticks: { color: '#8b949e' }, grid: { color: '#21262d' } }
        },
        plugins: { legend: { display: false } }
      }
    });
  }
}

async function loadTimeline() {
  const snaps = await api('/api/snapshots');
  if (!snaps.length) return;

  const labels = snaps.map(s => {
    const d = new Date(s.timestamp);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  });

  const datasets = [
    { label: 'Hot', data: snaps.map(s => s.hot), borderColor: COLORS.hot, fill: false, tension: 0.3 },
    { label: 'Warm', data: snaps.map(s => s.warm), borderColor: COLORS.warm, fill: false, tension: 0.3 },
    { label: 'Cold', data: snaps.map(s => s.cold), borderColor: COLORS.cold, fill: false, tension: 0.3 },
    { label: 'Forgotten', data: snaps.map(s => s.forgotten), borderColor: COLORS.forgotten, fill: false, tension: 0.3 },
  ];

  if (timelineChart) {
    timelineChart.data.labels = labels;
    timelineChart.data.datasets = datasets;
    timelineChart.update();
  } else {
    timelineChart = new Chart(document.getElementById('timelineChart'), {
      type: 'line',
      data: { labels, datasets },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: {
          x: { ticks: { color: '#8b949e' }, grid: { color: '#21262d' } },
          y: { ticks: { color: '#8b949e' }, grid: { color: '#21262d' } }
        },
        plugins: { legend: { labels: { color: '#8b949e' } } }
      }
    });
  }
}

async function loadMemories() {
  const text = document.getElementById('searchInput').value || null;
  const params = new URLSearchParams();
  if (text) params.set('text', text);
  if (currentTier) params.set('tier', currentTier);
  params.set('limit', '100');

  const memories = await api('/api/query?' + params);
  const tbody = document.getElementById('memoryTable');
  tbody.innerHTML = memories.map(m => {
    const tierClass = 'tier-' + m.tier.toLowerCase();
    const overridden = m.is_overridden ? ' overridden' : '';
    const content = m.content.length > 120 ? m.content.slice(0, 117) + '...' : m.content;
    return `<tr>
      <td class="${tierClass}">${m.tier}</td>
      <td class="salience">${m.salience.toFixed(2)}</td>
      <td class="${overridden}">${escapeHtml(content)}</td>
      <td>${escapeHtml(m.source)}</td>
      <td>${m.occurrence_count}</td>
      <td class="age">${formatAge(m.timestamp)}</td>
    </tr>`;
  }).join('');
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function setTier(btn, tier) {
  currentTier = tier;
  document.querySelectorAll('.controls .btn').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  loadMemories();
}

async function runSweep() {
  const r = await api('/api/sweep');
  alert(`Sweep: ${r.total_processed} processed, ${r.demoted} demoted, ${r.forgotten} forgotten, ${r.promoted} promoted`);
  refresh();
}

function refresh() {
  loadInspect();
  loadDistribution();
  loadTimeline();
  loadMemories();
}

refresh();
setInterval(refresh, 30000);
</script>
</body>
</html>"""
