/** Inline HTML dashboard — served at GET / on the monitoring server. */
export function buildDashboardHtml() {
    return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Solana Arb Bot</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

    :root {
      --bg:       #0d1117;
      --surface:  #161b22;
      --border:   #30363d;
      --text:     #e6edf3;
      --muted:    #8b949e;
      --green:    #3fb950;
      --red:      #f85149;
      --yellow:   #d29922;
      --blue:     #58a6ff;
      --purple:   #bc8cff;
      --orange:   #ffa657;
    }

    body { background: var(--bg); color: var(--text); font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; font-size: 14px; line-height: 1.5; }

    header { display: flex; align-items: center; justify-content: space-between; padding: 14px 20px; border-bottom: 1px solid var(--border); }
    header h1 { font-size: 16px; font-weight: 600; letter-spacing: .3px; }
    header h1 span { color: var(--blue); }
    .status-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--green); display: inline-block; margin-right: 6px; animation: pulse 2s infinite; }
    .status-dot.halted { background: var(--red); animation: none; }
    @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.4} }
    #last-update { font-size: 11px; color: var(--muted); }

    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px; padding: 16px 20px; }

    .card { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 16px; }
    .card-title { font-size: 11px; font-weight: 600; color: var(--muted); text-transform: uppercase; letter-spacing: .8px; margin-bottom: 10px; }

    .big-num { font-size: 28px; font-weight: 700; }
    .big-num.green { color: var(--green); }
    .big-num.red   { color: var(--red); }
    .big-num.blue  { color: var(--blue); }

    .stat-row { display: flex; justify-content: space-between; padding: 4px 0; border-bottom: 1px solid var(--border); }
    .stat-row:last-child { border-bottom: none; }
    .stat-label { color: var(--muted); }
    .stat-value { font-weight: 500; }
    .stat-value.good { color: var(--green); }
    .stat-value.warn { color: var(--yellow); }
    .stat-value.bad  { color: var(--red); }

    /* Stage badge */
    .stage-badge { display: inline-block; padding: 2px 8px; border-radius: 12px; font-size: 11px; font-weight: 600; background: rgba(88,166,255,.15); color: var(--blue); border: 1px solid rgba(88,166,255,.3); }

    /* Progress bar */
    .progress-wrap { margin-top: 10px; }
    .progress-label { display: flex; justify-content: space-between; font-size: 11px; color: var(--muted); margin-bottom: 4px; }
    .progress-track { height: 6px; background: var(--border); border-radius: 3px; overflow: hidden; }
    .progress-fill  { height: 100%; background: linear-gradient(90deg, var(--blue), var(--purple)); border-radius: 3px; transition: width .5s ease; }

    /* Bar chart */
    .bar-chart { display: flex; flex-direction: column; gap: 5px; margin-top: 4px; }
    .bar-row { display: flex; align-items: center; gap: 8px; }
    .bar-key  { width: 130px; flex-shrink: 0; font-size: 11px; color: var(--muted); text-align: right; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .bar-track { flex: 1; height: 14px; background: var(--border); border-radius: 2px; overflow: hidden; position: relative; }
    .bar-fill  { height: 100%; border-radius: 2px; transition: width .4s ease; min-width: 2px; }
    .bar-fill.rejection { background: rgba(248,81,73,.6); }
    .bar-fill.spread    { background: rgba(63,185,80,.6); }
    .bar-count { font-size: 11px; color: var(--muted); width: 36px; text-align: right; flex-shrink: 0; }

    /* Trade log */
    .trade-log { display: flex; flex-direction: column; gap: 4px; max-height: 260px; overflow-y: auto; margin-top: 4px; }
    .trade-row { display: grid; grid-template-columns: 60px 110px 1fr 70px; gap: 6px; padding: 5px 8px; border-radius: 4px; background: var(--bg); font-size: 12px; align-items: center; }
    .trade-type { font-size: 10px; font-weight: 700; padding: 1px 5px; border-radius: 3px; text-align: center; }
    .trade-type.exec { background: rgba(63,185,80,.2); color: var(--green); }
    .trade-type.rej  { background: rgba(248,81,73,.15); color: var(--red); }
    .trade-pair { color: var(--text); font-weight: 500; }
    .trade-reason { color: var(--muted); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .trade-profit { text-align: right; font-weight: 600; }
    .trade-profit.pos { color: var(--green); }
    .trade-profit.neg { color: var(--red); }
    .trade-profit.zero { color: var(--muted); }

    /* Optimization box */
    .opt-box { background: rgba(88,166,255,.07); border: 1px solid rgba(88,166,255,.2); border-radius: 6px; padding: 10px 12px; margin-top: 6px; }
    .opt-title { font-size: 11px; font-weight: 700; color: var(--blue); margin-bottom: 6px; }
    .opt-item { font-size: 12px; color: var(--text); padding: 2px 0; }
    .opt-item::before { content: '→ '; color: var(--blue); }

    .full-width { grid-column: 1 / -1; }
    .two-wide   { grid-column: span 2; }

    .empty { color: var(--muted); font-style: italic; font-size: 12px; text-align: center; padding: 16px 0; }

    footer { text-align: center; padding: 12px; font-size: 11px; color: var(--muted); border-top: 1px solid var(--border); }
  </style>
</head>
<body>

<header>
  <h1><span>◎</span> Solana Arb Bot — Dashboard</h1>
  <div style="display:flex;align-items:center;gap:12px">
    <span><span class="status-dot" id="dot"></span><span id="status-text">connecting...</span></span>
    <span id="last-update">—</span>
  </div>
</header>

<div class="grid" id="grid">
  <div class="card"><div class="empty">Loading…</div></div>
</div>

<footer>Auto-refreshes every 5s &nbsp;·&nbsp; <a href="/analytics" style="color:var(--muted)">Raw JSON</a></footer>

<script>
const fmt = {
  usd: v => v === null || v === undefined ? '—' : (v >= 0 ? '+' : '') + '$' + Math.abs(v).toFixed(v >= 100 ? 0 : v >= 1 ? 2 : 4),
  pct: v => (v * 100).toFixed(1) + '%',
  num: v => v === null || v === undefined ? '—' : Number(v).toLocaleString(),
  dur: ms => {
    const m = Math.floor(ms / 60000);
    const h = Math.floor(m / 60);
    return h > 0 ? h + 'h ' + (m % 60) + 'm' : m + 'm';
  },
  age: ms => {
    const s = Math.floor(Date.now() - ms) / 1000;
    return s < 60 ? Math.floor(s) + 's ago' : Math.floor(s/60) + 'm ago';
  }
};

function humanRejection(key) {
  if (!key) return 'unknown';
  if (key.startsWith('spread_below_threshold:')) {
    const m = key.match(/spread_below_threshold:(\\d+)bps<(\\d+)bps/);
    if (m) return 'Spread too low (' + m[1] + ' bps, need ' + m[2] + ')';
  }
  if (key.startsWith('below_min_profit:')) {
    const m = key.match(/below_min_profit:\\$([\\d.-]+)/);
    if (m) return 'Below min profit (' + (parseFloat(m[1]) >= 0 ? '+' : '') + '$' + parseFloat(m[1]).toFixed(4) + ')';
  }
  if (key.startsWith('fees_eat_spread:')) return 'Fees exceed spread';
  if (key.startsWith('missing_quote:')) return 'No quote from ' + key.split(':')[1];
  if (key === 'token_quality') return 'Token quality check failed';
  if (key === 'warming_up') return 'Warming up (not enough history)';
  if (key.startsWith('low_route_divergence:')) return 'No route divergence (' + key.split(':')[1] + ' bps)';
  if (key.startsWith('edge_eroded')) return 'Edge eroded by fees/latency';
  if (key === 'stale_quote') return 'Quote expired before execution';
  if (key === 'dead_man_switch_halted') return 'Dead-man switch: too many failures';
  if (key.startsWith('fees_exceed_profit')) return 'Gas cost > profit';
  if (key.startsWith('below_global_min:')) return 'Below global min profit';
  if (key.startsWith('no_spread:')) return 'Raydium = Orca price (no spread)';
  return key.replace(/_/g, ' ');
}

function stageColor(stage) {
  if (!stage) return 'var(--muted)';
  if (stage.startsWith('stage1')) return 'var(--yellow)';
  if (stage.startsWith('stage2')) return 'var(--orange)';
  if (stage.startsWith('stage3')) return 'var(--blue)';
  return 'var(--green)';
}

function stageNext(stage, capital) {
  if (!stage) return null;
  if (stage.startsWith('stage1')) return { label: 'Stage 2', target: 3, current: capital };
  if (stage.startsWith('stage2')) return { label: 'Stage 3', target: 10, current: capital };
  if (stage.startsWith('stage3')) return { label: 'Stage 4', target: 30, current: capital };
  return null;
}

function barChart(items, maxVal, cls, limit = 12) {
  if (!items.length) return '<div class="empty">No data yet</div>';
  const rows = items.slice(0, limit).map(([k, v]) => {
    const pct = maxVal > 0 ? Math.min(100, v / maxVal * 100) : 0;
    return '<div class="bar-row">'
      + '<div class="bar-key" title="' + k + '">' + humanRejection(k) + '</div>'
      + '<div class="bar-track"><div class="bar-fill ' + cls + '" style="width:' + pct.toFixed(1) + '%"></div></div>'
      + '<div class="bar-count">' + v + '</div>'
      + '</div>';
  });
  return '<div class="bar-chart">' + rows.join('') + '</div>';
}

function spreadSummary(rejections) {
  // Extract spread values from rejection reasons
  const spreads = [];
  for (const [k] of rejections) {
    const m = k.match(/spread_below_threshold:(\\d+)bps/);
    if (m) spreads.push(parseInt(m[1]));
  }
  if (!spreads.length) return null;
  spreads.sort((a,b) => a - b);
  const max = Math.max(...spreads);
  const median = spreads[Math.floor(spreads.length/2)];
  const above35 = spreads.filter(s => s >= 35).length;
  const above100 = spreads.filter(s => s >= 100).length;
  return { max, median, above35, above100, total: spreads.length };
}

function buildOpts(analytics, stats, journal) {
  const suggestions = [];
  const rr = Object.entries(analytics.rejectionReasons || {}).sort((a,b) => b[1]-a[1]);
  const ss = spreadSummary(rr);

  // Spread analysis
  if (ss) {
    if (ss.max < 35) suggestions.push('Max observed spread is ' + ss.max + ' bps — core pairs have no edge at current capital. Need pump.fun token discoveries.');
    else if (ss.above35 === 0) suggestions.push('No spreads ≥35 bps seen yet. Pump token registry may need time to surface opportunities.');
    else suggestions.push(ss.above35 + ' of ' + ss.total + ' observations had spread ≥35 bps — threshold adjustment may unlock trades.');
  }

  // Execution rate
  if (analytics.totalExecutions === 0 && stats && stats.scans > 20) {
    suggestions.push('Zero executions after ' + stats.scans + ' scans. Strategy is in price-discovery mode — watching for pump pair opportunities.');
  }

  // Missing quotes
  const missingQuote = rr.find(([k]) => k.startsWith('missing_quote:'));
  if (missingQuote && missingQuote[1] > 5) {
    suggestions.push('Many "no quote" rejections — some pairs have no liquidity on one DEX. Pruning would improve scan efficiency.');
  }

  if (!suggestions.length) suggestions.push('Strategy running normally. Accumulating spread data for parameter optimization.');
  return suggestions;
}

async function load() {
  try {
    const [a, s, j] = await Promise.all([
      fetch('/analytics').then(r => r.json()),
      fetch('/scan-stats').then(r => r.json()),
      fetch('/journal?limit=30').then(r => r.json()),
    ]);

    document.getElementById('dot').className = 'status-dot' + (s.error ? ' halted' : '');
    document.getElementById('status-text').textContent = s.error ? 'halted' : 'running';
    document.getElementById('last-update').textContent = 'Updated ' + new Date().toLocaleTimeString();

    const pnl = a.sessionPnlUsd ?? 0;
    const capital = a.capitalSol;
    const stage = a.capitalStage;
    const next = stage ? stageNext(stage, capital) : null;
    const stageProg = next ? Math.min(100, ((next.current - (next.target === 3 ? 1 : next.target === 10 ? 3 : 10)) / (next.target - (next.target === 3 ? 1 : next.target === 10 ? 3 : 10))) * 100) : 100;
    const rr = Object.entries(a.rejectionReasons || {}).sort((a,b) => b[1]-a[1]);
    const maxRej = rr.length ? rr[0][1] : 1;
    const execs = (j.events || []).filter(e => e.type === 'execution' || e.type === 'rejection').reverse().slice(0, 20);
    const opts = buildOpts(a, s, j);

    document.getElementById('grid').innerHTML = /* html */ \`

    <!-- P&L Card -->
    <div class="card">
      <div class="card-title">Session P&amp;L</div>
      <div class="big-num \${pnl >= 0 ? 'green' : 'red'}">\${fmt.usd(pnl)}</div>
      <div style="margin-top:10px">
        <div class="stat-row"><span class="stat-label">Trades executed</span><span class="stat-value">\${fmt.num(a.totalExecutions)}</span></div>
        <div class="stat-row"><span class="stat-label">Rate</span><span class="stat-value">\${(a.tradesPerHour || 0).toFixed(1)}/hr</span></div>
        <div class="stat-row"><span class="stat-label">Avg per trade</span><span class="stat-value \${(a.avgRealizedProfitUsd||0) >= 0 ? 'good' : 'bad'}">\${fmt.usd(a.avgRealizedProfitUsd)}</span></div>
        <div class="stat-row"><span class="stat-label">Uptime</span><span class="stat-value">\${fmt.dur((a.uptimeMinutes||0)*60000)}</span></div>
      </div>
    </div>

    <!-- Scan Health -->
    <div class="card">
      <div class="card-title">Scan Health</div>
      <div class="big-num blue">\${fmt.num(s.scans || 0)}</div>
      <div style="font-size:11px;color:var(--muted);margin-bottom:8px">total scans</div>
      <div class="stat-row"><span class="stat-label">Pairs loaded</span><span class="stat-value">\${s.pairCount || '—'}</span></div>
      <div class="stat-row"><span class="stat-label">Actionable signals</span><span class="stat-value \${(s.actionable||0) > 0 ? 'good' : ''}">\${s.actionable || 0}</span></div>
      <div class="stat-row"><span class="stat-label">Rejected</span><span class="stat-value warn">\${s.rejected || 0}</span></div>
      <div class="stat-row"><span class="stat-label">Hit rate</span><span class="stat-value">\${fmt.pct(a.opportunityHitRate || 0)}</span></div>
    </div>

    <!-- Capital Stage -->
    <div class="card">
      <div class="card-title">Capital Stage</div>
      \${capital ? \`
        <div style="display:flex;align-items:baseline;gap:8px;margin-bottom:4px">
          <div class="big-num blue">\${(capital||0).toFixed(3)}</div>
          <div style="color:var(--muted)">SOL</div>
          <div class="stage-badge" style="margin-left:auto;color:\${stageColor(stage)};border-color:\${stageColor(stage)}30;background:\${stageColor(stage)}15">\${(stage||'').replace(':', ' ')}</div>
        </div>
        \${next ? \`
        <div class="progress-wrap">
          <div class="progress-label"><span>Progress to \${next.label}</span><span>\${next.current.toFixed(2)} / \${next.target} SOL</span></div>
          <div class="progress-track"><div class="progress-fill" style="width:\${Math.max(2,stageProg).toFixed(1)}%"></div></div>
        </div>\` : '<div style="margin-top:8px;font-size:12px;color:var(--green)">✓ Full strategy suite active</div>'}
        <div style="margin-top:10px">
          <div class="stat-row"><span class="stat-label">Min spread needed</span><span class="stat-value warn">\${stage?.startsWith('stage1') ? '100' : stage?.startsWith('stage2') ? '60' : stage?.startsWith('stage3') ? '35' : '25'} bps</span></div>
          <div class="stat-row"><span class="stat-label">Trade size</span><span class="stat-value">\${stage?.startsWith('stage1') ? '0.5' : stage?.startsWith('stage2') ? 'cap×50%' : 'cap×60-70%'} SOL</span></div>
        </div>
      \` : '<div class="empty">Capital tracker not connected</div>'}
    </div>

    <!-- Profit Distribution -->
    <div class="card">
      <div class="card-title">Profit Distribution</div>
      \${a.profitDistribution ? \`
        <div class="bar-chart">
          \${Object.entries(a.profitDistribution).map(([k,v]) => {
            const maxV = Math.max(...Object.values(a.profitDistribution));
            const pct = maxV > 0 ? (v / maxV * 100) : 0;
            const col = k === 'loss' ? 'var(--red)' : k === '$0-$0.10' ? 'var(--yellow)' : 'var(--green)';
            return '<div class="bar-row"><div class="bar-key">' + k + '</div><div class="bar-track"><div class="bar-fill" style="width:'+Math.max(v>0?2:0,pct).toFixed(1)+'%;background:'+col+'60"></div></div><div class="bar-count">' + v + '</div></div>';
          }).join('')}
        </div>
      \` : '<div class="empty">No executions yet</div>'}
    </div>

    <!-- Rejection Breakdown (full width) -->
    <div class="card two-wide">
      <div class="card-title">Why Trades Are Being Rejected</div>
      \${rr.length ? barChart(rr, maxRej, 'rejection') : '<div class="empty">No rejections logged yet</div>'}
    </div>

    <!-- Optimization Hints -->
    <div class="card">
      <div class="card-title">Strategy Insights</div>
      <div class="opt-box">
        <div class="opt-title">Current Analysis</div>
        \${opts.map(o => '<div class="opt-item">' + o + '</div>').join('')}
      </div>
    </div>

    <!-- Recent Activity -->
    <div class="card two-wide">
      <div class="card-title">Recent Activity</div>
      \${execs.length ? '<div class="trade-log">' + execs.map(e => {
        const isExec = e.type === 'execution';
        const profit = e.realizedProfitUsd ?? e.expectedProfitUsd ?? null;
        const meta = e.routeMetadata || {};
        const spread = meta.spreadBps;
        const reason = isExec ? (spread !== undefined ? spread + ' bps spread' : 'executed') : humanRejection(e.rejectionReason);
        const profitClass = profit === null ? 'zero' : profit > 0 ? 'pos' : 'neg';
        return '<div class="trade-row">'
          + '<div class="trade-type ' + (isExec ? 'exec' : 'rej') + '">' + (isExec ? 'TRADE' : 'SKIP') + '</div>'
          + '<div class="trade-pair">' + (e.pairLabel || '—') + '</div>'
          + '<div class="trade-reason">' + reason + '</div>'
          + '<div class="trade-profit ' + profitClass + '">' + (profit !== null ? fmt.usd(profit) : '—') + '</div>'
          + '</div>';
      }).join('') + '</div>'
      : '<div class="empty">Waiting for first scan results…</div>'}
    </div>

    \`;
  } catch(err) {
    console.error(err);
    document.getElementById('status-text').textContent = 'error';
  }
}

load();
setInterval(load, 5000);
</script>
</body>
</html>`;
}
