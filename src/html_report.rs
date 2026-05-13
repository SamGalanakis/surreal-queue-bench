use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Serialize;

use super::Args;

pub(super) fn html_output_path(args: &Args) -> Option<PathBuf> {
    if let Some(path) = &args.export {
        return Some(path.clone());
    }
    if args.no_html {
        return None;
    }
    args.output.as_ref().map(|path| {
        let mut html_path = path.clone();
        html_path.set_extension("html");
        html_path
    })
}

pub(super) fn write_html_report<T: Serialize>(args: &Args, report: &T) -> Result<()> {
    let Some(path) = html_output_path(args) else {
        return Ok(());
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let html = render_html_report(report)?;
    fs::write(&path, html).with_context(|| format!("write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn render_html_report<T: Serialize>(report: &T) -> Result<String> {
    Ok(HTML_TEMPLATE.replace("__REPORT_JSON__", &encode_report_data(report)?))
}

fn encode_report_data<T: Serialize>(report: &T) -> Result<String> {
    Ok(serde_json::to_string(report)
        .context("serialize HTML report data")?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}

const HTML_TEMPLATE: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>SurrealDB Queue Run</title>
<link rel="icon" href="data:,">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,400;9..144,500;9..144,700;9..144,900&family=Newsreader:opsz,wght@6..72,400;6..72,500;6..72,600&family=JetBrains+Mono:wght@400;500;700&display=swap" rel="stylesheet">
<style>
:root {
  --paper:        #f1eadb;
  --paper-shade:  #e8dec9;
  --paper-deep:   #ddd1b6;
  --ink:          #1a1d24;
  --ink-soft:     #3b3f49;
  --ink-faint:    #6b6e77;
  --rule:         #c5b89b;
  --red:          #b3322a;
  --red-deep:     #872019;
  --blue:         #2a4a73;
  --blue-deep:    #1a3354;
  --amber:        #a67517;
  --amber-soft:   #d4a843;
  --olive:        #5b6233;
  --ok:           #5b6233;

  --mono:    'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace;
  --serif:   'Newsreader', 'Spectral', Georgia, serif;
  --display: 'Fraunces', 'Spectral', Georgia, serif;
}

* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; }
body {
  font-family: var(--serif);
  color: var(--ink);
  background: var(--paper);
  background-image:
    radial-gradient(circle at 20% 10%, rgba(0,0,0,0.025) 0, transparent 50%),
    radial-gradient(circle at 80% 60%, rgba(0,0,0,0.02) 0, transparent 50%),
    url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='40' height='40'><circle cx='1' cy='1' r='0.6' fill='%23c5b89b' opacity='0.35'/></svg>");
  line-height: 1.55;
  font-size: 16px;
  min-height: 100vh;
}

.page {
  max-width: 1280px;
  margin: 0 auto;
  padding: 28px 28px 80px;
}

/* COVER */
header.cover {
  border-top: 6px solid var(--ink);
  border-bottom: 1px solid var(--ink);
  padding: 18px 0 26px;
  position: relative;
}
.top-strip {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--ink-soft);
  padding-bottom: 14px;
  border-bottom: 1px dotted var(--rule);
  margin-bottom: 20px;
  flex-wrap: wrap;
  gap: 12px;
}
.top-strip .lhs { color: var(--red); font-weight: 700; }
.top-strip .rhs span { margin-left: 16px; }

h1.title {
  font-family: var(--display);
  font-weight: 900;
  font-size: clamp(40px, 7vw, 84px);
  line-height: 0.95;
  letter-spacing: -0.03em;
  margin: 0 0 12px;
  font-variation-settings: "opsz" 144, "SOFT" 0;
}
h1.title em {
  font-style: italic;
  color: var(--red);
  font-variation-settings: "opsz" 144, "SOFT" 50;
}
.subhead {
  font-family: var(--serif);
  font-size: 19px;
  font-weight: 400;
  line-height: 1.45;
  color: var(--ink-soft);
  max-width: 920px;
  margin: 0 0 20px;
}
.run-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.pill {
  display: inline-flex;
  align-items: center;
  padding: 4px 10px;
  border: 1px solid var(--ink);
  background: var(--paper-shade);
  color: var(--ink);
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.04em;
}

/* KPI STRIP */
.kpis {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0;
  border: 1px solid var(--ink);
  background: var(--paper-shade);
  margin: 22px 0 0;
}
.kpi {
  padding: 14px 16px 16px;
  border-right: 1px solid var(--ink);
}
.kpi:last-child { border-right: none; }
.kpi .label {
  font-family: var(--mono);
  font-size: 10px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--ink-faint);
  display: block;
  margin-bottom: 8px;
}
.kpi .value {
  font-family: var(--display);
  font-weight: 700;
  font-size: 30px;
  color: var(--ink);
  line-height: 1;
  margin: 0;
  font-variation-settings: "opsz" 60;
}
.kpi .hint {
  margin-top: 8px;
  font-family: var(--mono);
  font-size: 10px;
  color: var(--ink-faint);
  letter-spacing: 0.04em;
}

/* TOC */
nav.toc {
  position: sticky;
  top: 0;
  background: var(--paper);
  z-index: 10;
  padding: 14px 0 0;
  margin: 32px 0 0;
  border-bottom: 2px solid var(--ink);
  display: flex;
  gap: 0;
  overflow-x: auto;
}
nav.toc a {
  background: transparent;
  border: 1px solid var(--ink);
  border-bottom: none;
  border-right: none;
  padding: 11px 16px 9px;
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--ink-soft);
  text-decoration: none;
  white-space: nowrap;
  transition: color 120ms, background 120ms;
}
nav.toc a:last-of-type { border-right: 1px solid var(--ink); }
nav.toc a:hover { color: var(--red); background: var(--paper-shade); }
nav.toc a.active {
  background: var(--ink);
  color: var(--paper);
  font-weight: 700;
}
nav.toc a span.idx {
  color: var(--red);
  margin-right: 8px;
  font-weight: 700;
}
nav.toc a.active span.idx { color: var(--amber-soft); }

/* SECTIONS */
section.section {
  margin-top: 32px;
  scroll-margin-top: 72px;
}
.section .marker {
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.2em;
  color: var(--red);
  text-transform: uppercase;
  font-weight: 700;
  display: block;
  margin-bottom: 6px;
}
.section h2 {
  font-family: var(--display);
  font-weight: 700;
  font-size: 30px;
  line-height: 1.04;
  letter-spacing: -0.012em;
  margin: 0 0 4px;
  color: var(--ink);
  font-variation-settings: "opsz" 96;
}
.section .note {
  font-family: var(--serif);
  color: var(--ink-soft);
  font-size: 15px;
  margin: 4px 0 18px;
  line-height: 1.5;
  max-width: 80ch;
}
.section .note code { font-family: var(--mono); font-size: 0.9em; color: var(--red-deep); background: var(--paper-shade); border: 1px solid var(--rule); padding: 1px 6px; }

/* FRAMES */
.frame {
  border: 1px solid var(--ink);
  background: var(--paper);
  margin: 14px 0 22px;
  position: relative;
}
.frame .label {
  position: absolute;
  top: -10px;
  left: 16px;
  background: var(--paper);
  padding: 0 8px;
  font-family: var(--mono);
  font-size: 10px;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  color: var(--red);
  font-weight: 700;
}
.frame .body { padding: 22px 22px 20px; }

/* Two-col */
.cols-2 {
  display: grid;
  grid-template-columns: 7fr 5fr;
  gap: 18px;
}
@media (max-width: 900px) { .cols-2 { grid-template-columns: 1fr; } }

/* BARS */
.bar-list { display: grid; gap: 12px; }
.bar-row {
  display: grid;
  grid-template-columns: 160px 1fr 130px;
  gap: 14px;
  align-items: center;
}
.bar-name {
  font-family: var(--mono);
  font-size: 12px;
  color: var(--ink-soft);
  letter-spacing: 0.02em;
}
.bar-track {
  height: 16px;
  background: var(--paper-shade);
  border: 1px solid var(--ink);
  position: relative;
  overflow: hidden;
}
.bar-fill {
  display: block;
  height: 100%;
  width: var(--w, 0%);
  background: var(--red);
}
.bar-fill.alt { background: var(--blue); }
.bar-fill.bad { background: var(--amber); }
.bar-val {
  text-align: right;
  font-family: var(--mono);
  font-size: 12px;
  font-weight: 700;
  color: var(--ink);
  letter-spacing: 0.02em;
}

/* RANGE / PERCENTILE TRACKS */
.range-table { display: grid; gap: 0; }
.range-row {
  display: grid;
  grid-template-columns: 170px minmax(0, 1fr) 320px;
  gap: 16px;
  align-items: center;
  padding: 14px 0;
  border-top: 1px dotted var(--rule);
}
.range-row:first-child { border-top: none; padding-top: 0; }
.range-label {
  font-family: var(--mono);
  font-size: 13px;
  color: var(--ink);
  font-weight: 700;
}
.range-sub {
  display: block;
  margin-top: 3px;
  color: var(--ink-faint);
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  font-weight: 400;
}
.range-track {
  position: relative;
  height: 26px;
  background: var(--paper-shade);
  border: 1px solid var(--ink);
}
.range-fill {
  position: absolute;
  inset: 0 auto 0 0;
  width: var(--w, 0%);
  background: var(--red);
  opacity: 0.18;
}
.range-track .tick {
  position: absolute;
  top: -4px;
  bottom: -4px;
  width: 2px;
  transform: translateX(-1px);
  background: var(--ink);
}
.range-track .tick::after {
  content: attr(data-label);
  position: absolute;
  top: -16px;
  left: 50%;
  transform: translateX(-50%);
  font-family: var(--mono);
  font-size: 9px;
  color: var(--ink);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  font-weight: 700;
}
.range-track .tick.p95 { background: var(--amber); }
.range-track .tick.p95::after { color: var(--amber); }
.range-track .tick.p99 { background: var(--red); }
.range-track .tick.p99::after { color: var(--red); }
.range-stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 5px;
  margin-top: 8px;
}
.stat-chip {
  border: 1px solid var(--ink);
  padding: 5px 8px 6px;
  background: var(--paper-shade);
}
.stat-chip span {
  display: block;
  font-family: var(--mono);
  font-size: 9px;
  letter-spacing: 0.15em;
  text-transform: uppercase;
  color: var(--ink-faint);
}
.stat-chip strong {
  display: block;
  margin-top: 2px;
  font-family: var(--mono);
  font-size: 13px;
  font-weight: 700;
  color: var(--ink);
  letter-spacing: 0.02em;
}

/* SPARKLINE */
.sparkline {
  width: 100%;
  height: 70px;
  border: 1px solid var(--ink);
  background: var(--paper-shade);
}
.sparkline svg { display: block; width: 100%; height: 70px; }
.sparkline polyline {
  fill: none;
  stroke: var(--red);
  stroke-width: 1.8;
  vector-effect: non-scaling-stroke;
}
.sparkline path.grid {
  stroke: var(--rule);
  stroke-width: 1;
  stroke-dasharray: 2 3;
}
.sparkline text {
  fill: var(--ink-faint);
  font-family: var(--mono);
  font-size: 10px;
  letter-spacing: 0.08em;
}

/* TABLES */
table.spec {
  width: 100%;
  border-collapse: collapse;
  font-family: var(--mono);
  font-size: 13px;
  border: 1px solid var(--ink);
}
table.spec th, table.spec td {
  text-align: left;
  padding: 9px 14px;
  border-bottom: 1px solid var(--rule);
  vertical-align: top;
}
table.spec th {
  background: var(--ink);
  color: var(--paper);
  font-size: 11px;
  letter-spacing: 0.15em;
  text-transform: uppercase;
  font-weight: 700;
}
table.spec tr:last-child td { border-bottom: none; }
table.spec tr:nth-child(even) td { background: var(--paper-shade); }
table.spec code {
  color: var(--red-deep);
  font-family: var(--mono);
}
table.spec td.num {
  font-variant-numeric: tabular-nums;
  text-align: right;
}
table.spec th.num { text-align: right; }

/* ISSUES */
.issue-list { display: grid; gap: 10px; }
.issue {
  border: 1px solid var(--ink);
  border-left: 4px solid var(--olive);
  padding: 12px 14px;
  background: var(--paper);
}
.issue.bad  { border-left-color: var(--red); }
.issue.warn { border-left-color: var(--amber); }
.issue strong {
  display: block;
  font-family: var(--display);
  font-size: 16px;
  font-weight: 700;
  color: var(--ink);
}
.issue.bad strong  { color: var(--red); }
.issue.warn strong { color: var(--amber); }
.issue span {
  display: block;
  margin-top: 4px;
  color: var(--ink-soft);
  font-size: 13px;
  line-height: 1.45;
}

/* DETAILS / EXPLAIN */
details {
  border: 1px solid var(--ink);
  background: var(--paper);
  margin-bottom: 10px;
}
summary {
  cursor: pointer;
  padding: 10px 14px;
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  font-weight: 700;
  color: var(--ink);
  background: var(--paper-shade);
  border-bottom: 1px solid var(--ink);
  list-style: none;
}
summary::-webkit-details-marker { display: none; }
summary::before {
  content: "+ ";
  color: var(--red);
  margin-right: 6px;
}
details[open] summary::before { content: "- "; }
pre {
  margin: 0;
  padding: 14px 18px;
  overflow: auto;
  max-height: 440px;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.6;
  color: var(--ink);
  background: var(--paper);
  white-space: pre-wrap;
  word-break: break-word;
}

/* TOOLTIP */
.tip {
  position: fixed;
  pointer-events: none;
  z-index: 200;
  background: var(--ink);
  color: var(--paper);
  border: 1px solid var(--ink);
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.04em;
  padding: 8px 12px 9px;
  max-width: 360px;
  opacity: 0;
  transition: opacity 80ms;
  line-height: 1.5;
}
.tip.show { opacity: 1; }
.tip strong {
  color: var(--amber-soft);
  display: block;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  font-size: 10px;
  margin-bottom: 4px;
}
.tip .row { display: flex; justify-content: space-between; gap: 16px; }
.tip .row span:first-child { color: var(--ink-faint); }
.tip .row span:last-child { color: var(--paper); font-weight: 700; }

/* CHART LEGEND */
.chart-legend {
  display: flex;
  gap: 18px;
  font-family: var(--mono);
  font-size: 11px;
  color: var(--ink-soft);
  letter-spacing: 0.04em;
  margin: 0 0 14px;
  flex-wrap: wrap;
  align-items: center;
}
.chart-legend .swatch {
  display: inline-flex;
  align-items: center;
  gap: 7px;
}
.chart-legend i.sw {
  display: inline-block;
  width: 14px;
  height: 12px;
  border: 1px solid var(--ink);
  background: var(--paper);
}
.chart-legend i.sw.red { background: var(--red); border-color: var(--red-deep); }
.chart-legend i.sw.blue { background: var(--blue); border-color: var(--blue-deep); }
.chart-legend i.sw.range { background: var(--red); opacity: 0.18; }
.chart-legend i.sw.tick { width: 2px; height: 16px; border: none; background: var(--ink); }
.chart-legend i.sw.tick.amber { background: var(--amber); }
.chart-legend i.sw.tick.red { background: var(--red); }

/* SPARKLINE INTERACTIVE BITS */
.sparkline svg .crosshair {
  stroke: var(--ink);
  stroke-width: 1;
  stroke-dasharray: 2 2;
}
.sparkline svg .crosspoint {
  fill: var(--red);
  stroke: var(--ink);
  stroke-width: 1;
}
.sparkline svg .hover-overlay {
  cursor: crosshair;
}
.sparkline svg:focus { outline: 2px solid var(--amber); outline-offset: -2px; }

/* FOCUS rings consistent with the engineering look */
.range-track:focus,
.bar-track:focus,
.range-track .tick:focus {
  outline: 2px solid var(--amber);
  outline-offset: 2px;
}
.range-track { cursor: crosshair; }
.bar-track   { cursor: crosshair; }
.range-track .tick { cursor: pointer; }

/* CHART MODAL */
.chart-modal {
  position: fixed;
  inset: 0;
  background: var(--ink);
  z-index: 1000;
  display: none;
  flex-direction: column;
  padding: 20px;
}
.chart-modal.open { display: flex; }
.chart-modal .titlebar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 4px 14px;
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--paper);
  gap: 12px;
  flex-wrap: wrap;
}
.chart-modal .titlebar .left {
  color: var(--amber-soft);
  font-weight: 700;
  display: flex;
  gap: 14px;
  align-items: baseline;
  flex-wrap: wrap;
}
.chart-modal .titlebar .left small { color: var(--paper); font-weight: 400; opacity: 0.7; letter-spacing: 0.08em; }
.chart-modal .titlebar .controls { display: flex; gap: 6px; }
.chart-modal .titlebar button {
  background: var(--paper);
  border: 1px solid var(--paper);
  color: var(--ink);
  font-family: var(--mono);
  font-size: 12px;
  font-weight: 700;
  padding: 6px 14px;
  letter-spacing: 0.1em;
  cursor: pointer;
}
.chart-modal .titlebar button:hover { background: var(--amber-soft); border-color: var(--amber-soft); }
.chart-modal .titlebar button.close { background: var(--red); color: var(--paper); border-color: var(--red); }
.chart-modal .titlebar button.close:hover { background: var(--red-deep); border-color: var(--red-deep); color: var(--paper); }
.chart-modal .stage {
  flex: 1;
  background: var(--paper);
  border: 1px solid var(--paper);
  overflow: auto;
  padding: 26px 28px;
}

/* BIG CHART */
.big-chart { font-family: var(--mono); }
.big-chart .meta {
  display: flex;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 11px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--ink-faint);
  padding-bottom: 10px;
  border-bottom: 1px dotted var(--rule);
  margin-bottom: 16px;
}
.big-chart svg.canvas { display: block; width: 100%; height: auto; }
.big-chart .axis-line { stroke: var(--ink); stroke-width: 1; }
.big-chart .gridline { stroke: var(--rule); stroke-width: 1; stroke-dasharray: 2 4; }
.big-chart .axis-label { fill: var(--ink-soft); font-size: 13px; font-family: var(--mono); }
.big-chart .axis-title {
  fill: var(--ink);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  font-family: var(--mono);
}
.big-chart .data-area {
  fill: var(--red);
  opacity: 0.10;
}
.big-chart .data-line {
  fill: none;
  stroke: var(--red);
  stroke-width: 1.6;
  vector-effect: non-scaling-stroke;
}
.big-chart .data-point {
  fill: var(--red);
  stroke: var(--paper);
  stroke-width: 1;
}
.big-chart .data-point:hover { fill: var(--ink); }
.big-chart .percentile {
  stroke: var(--ink);
  stroke-width: 1.4;
  stroke-dasharray: 5 4;
  vector-effect: non-scaling-stroke;
}
.big-chart .percentile.p95 { stroke: var(--amber); }
.big-chart .percentile.p99 { stroke: var(--red); stroke-dasharray: 7 3; }
.big-chart .percentile-label {
  font-family: var(--mono);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}
.big-chart .percentile-label.p50 { fill: var(--ink); }
.big-chart .percentile-label.p95 { fill: var(--amber); }
.big-chart .percentile-label.p99 { fill: var(--red); }
.big-chart .summary {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
  gap: 0;
  margin-top: 22px;
  border: 1px solid var(--ink);
  background: var(--paper-shade);
}
.big-chart .summary .stat {
  padding: 10px 14px;
  border-right: 1px solid var(--ink);
}
.big-chart .summary .stat:last-child { border-right: none; }
.big-chart .summary .stat .lbl {
  font-size: 10px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--ink-faint);
}
.big-chart .summary .stat .val {
  font-family: var(--display);
  font-size: 22px;
  font-weight: 700;
  margin-top: 4px;
  color: var(--ink);
  letter-spacing: -0.005em;
}
.big-chart .cursor-line { stroke: var(--ink); stroke-width: 1; stroke-dasharray: 2 3; pointer-events: none; }
.big-chart .cursor-point { fill: var(--ink); stroke: var(--paper); stroke-width: 1; pointer-events: none; }
.big-chart .hover-overlay { fill: transparent; cursor: crosshair; }

/* "expand to chart" affordance */
.range-row .expand-chart {
  display: inline-block;
  font-family: var(--mono);
  font-size: 10px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--ink-faint);
  background: transparent;
  border: 1px solid var(--rule);
  padding: 3px 8px 2px;
  cursor: pointer;
  margin-left: 6px;
  transition: color 120ms, background 120ms, border-color 120ms;
}
.range-row .expand-chart:hover, .range-row .expand-chart:focus {
  color: var(--paper);
  background: var(--ink);
  border-color: var(--ink);
  outline: none;
}
.sparkline svg, .range-track { cursor: zoom-in; }

/* === MATRIX REPORT === */

/* Top banner for invalid runs */
.top-banner {
  border: 1px solid var(--red);
  background: color-mix(in oklch, var(--red) 14%, var(--paper));
  color: var(--red-deep);
  padding: 10px 16px;
  font-family: var(--mono);
  font-size: 12px;
  letter-spacing: 0.04em;
  margin: 22px 0 0;
  display: flex;
  gap: 14px;
  align-items: center;
  flex-wrap: wrap;
}
.top-banner strong {
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  font-size: 11px;
}

/* KPI hint clamp to 2 lines */
.kpi .hint {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* REGIME SWITCHER */
.regime-bar {
  display: flex;
  margin: 22px 0 0;
  border: 1px solid var(--ink);
  background: var(--paper-shade);
  align-items: stretch;
  flex-wrap: wrap;
}
.regime-bar .label {
  padding: 12px 18px 11px;
  font-family: var(--mono);
  font-size: 10px;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  font-weight: 700;
  color: var(--ink-faint);
  background: var(--paper-shade);
  border-right: 1px solid var(--ink);
  display: inline-flex;
  align-items: center;
}
.regime-bar button {
  flex: 1 1 0;
  background: transparent;
  border: none;
  border-right: 1px solid var(--rule);
  padding: 12px 18px 11px;
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  font-weight: 700;
  color: var(--ink-soft);
  cursor: pointer;
  transition: color 100ms, background 100ms;
}
.regime-bar button:last-child { border-right: none; }
.regime-bar button:hover { background: color-mix(in oklch, var(--amber) 8%, var(--paper)); color: var(--red); }
.regime-bar button.active { background: var(--ink); color: var(--paper); }
.regime-bar button .count {
  display: inline-block;
  margin-left: 8px;
  color: var(--ink-faint);
  font-weight: 400;
  letter-spacing: 0.04em;
}
.regime-bar button.active .count { color: var(--amber-soft); }

/* SCATTER PIVOT */
.scatter-controls {
  display: flex;
  gap: 18px;
  align-items: center;
  flex-wrap: wrap;
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--ink-soft);
  margin-bottom: 16px;
}
.scatter-controls label {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.scatter-controls label span { color: var(--ink-faint); }
.scatter-controls select {
  font-family: var(--mono);
  font-size: 12px;
  letter-spacing: 0.04em;
  text-transform: none;
  background: var(--paper);
  border: 1px solid var(--ink);
  padding: 5px 30px 5px 10px;
  color: var(--ink);
  cursor: pointer;
  font-weight: 700;
  appearance: none;
  background-image: linear-gradient(45deg, transparent 50%, var(--ink) 50%), linear-gradient(135deg, var(--ink) 50%, transparent 50%);
  background-position: calc(100% - 16px) 50%, calc(100% - 10px) 50%;
  background-size: 6px 6px;
  background-repeat: no-repeat;
}
.scatter-controls select:focus { outline: 2px solid var(--amber); outline-offset: 2px; }

.scatter-svg-wrap {
  background: var(--paper-shade);
  border: 1px solid var(--ink);
  padding: 4px;
  position: relative;
}
.scatter-svg { display: block; width: 100%; height: auto; }
.scatter-svg .axis-line { stroke: var(--ink); stroke-width: 1.2; }
.scatter-svg .gridline { stroke: var(--rule); stroke-width: 1; stroke-dasharray: 2 4; }
.scatter-svg .axis-label { fill: var(--ink-soft); font-size: 13px; font-family: var(--mono); }
.scatter-svg .axis-title {
  fill: var(--ink);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  font-family: var(--mono);
}
.scatter-svg .point {
  stroke: var(--ink);
  stroke-width: 1;
  cursor: pointer;
  transition: stroke-width 120ms;
}
.scatter-svg .point:hover { stroke-width: 2.4; }
.scatter-svg .point.focused { stroke-width: 3; }
.scatter-svg .point.dim { opacity: 0.18; }
.scatter-svg .point.invalid {
  fill: var(--paper);
}

.scatter-legend {
  display: flex;
  gap: 4px 14px;
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.04em;
  flex-wrap: wrap;
  margin-top: 12px;
  align-items: center;
}
.scatter-legend .lbl {
  color: var(--ink-faint);
  letter-spacing: 0.16em;
  text-transform: uppercase;
  font-size: 10px;
  margin-right: 6px;
}
.scatter-legend .item {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  cursor: pointer;
  padding: 3px 6px;
  border-radius: 2px;
  background: transparent;
  border: 1px solid transparent;
  color: inherit;
  font: inherit;
  letter-spacing: inherit;
  line-height: 1;
}
.scatter-legend .item:hover { background: var(--paper-shade); border-color: var(--rule); }
.scatter-legend .item:focus-visible { outline: 2px solid var(--amber); outline-offset: 2px; }
.scatter-legend .item.muted { opacity: 0.32; }
.scatter-legend .item.muted .swatch { background: transparent !important; }
.scatter-legend .item.ghost {
  text-transform: uppercase;
  font-size: 10px;
  letter-spacing: 0.16em;
  color: var(--red);
}
.scatter-legend .item.ghost:hover { background: var(--paper-shade); color: var(--red-deep); }
.scatter-legend .muted-count {
  margin-left: auto;
  color: var(--ink-faint);
  letter-spacing: 0.12em;
  text-transform: uppercase;
  font-size: 10px;
}
.scatter-legend .swatch {
  width: 12px;
  height: 12px;
  border: 1px solid var(--ink);
  border-radius: 50%;
}

/* RUN TABLE */
.run-table-controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 12px;
}
.col-toggle {
  border: 1px solid var(--ink);
  background: var(--paper-shade);
}
.col-toggle > summary {
  padding: 6px 14px;
  font-family: var(--mono);
  font-size: 10px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  font-weight: 700;
  color: var(--ink);
  cursor: pointer;
  list-style: none;
}
.col-toggle > summary::-webkit-details-marker { display: none; }
.col-toggle > summary::before { content: "+ "; color: var(--red); }
.col-toggle[open] > summary::before { content: "− "; }
.col-toggle[open] > summary { background: var(--ink); color: var(--paper); }
.col-toggle[open] > summary::before { color: var(--amber-soft); }
.col-toggle .col-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
  gap: 0;
  border-top: 1px solid var(--ink);
}
.col-toggle .col-grid label {
  font-family: var(--mono);
  font-size: 11px;
  padding: 8px 12px;
  border-right: 1px solid var(--rule);
  border-bottom: 1px solid var(--rule);
  display: flex;
  gap: 8px;
  align-items: center;
  background: var(--paper);
  cursor: pointer;
  letter-spacing: 0.02em;
}
.col-toggle .col-grid label:hover { background: color-mix(in oklch, var(--amber) 8%, var(--paper)); }
.col-toggle input[type="checkbox"] { accent-color: var(--red); }
.run-table-count {
  font-family: var(--mono);
  font-size: 11px;
  color: var(--ink-faint);
  letter-spacing: 0.06em;
}
.run-table-count em { color: var(--ink); font-style: normal; font-weight: 700; }
.run-table-hint {
  font-family: var(--mono);
  font-size: 10px;
  color: var(--ink-faint);
  letter-spacing: 0.1em;
  text-transform: uppercase;
  margin: 10px 0 0;
}

table.run-table {
  width: 100%;
  border-collapse: collapse;
  font-family: var(--mono);
  font-size: 12px;
  border: 1px solid var(--ink);
}
table.run-table th, table.run-table td {
  text-align: left;
  padding: 8px 12px;
  border-bottom: 1px solid var(--rule);
  border-right: 1px solid var(--rule);
  vertical-align: top;
}
table.run-table th:last-child, table.run-table td:last-child { border-right: none; }
table.run-table th {
  background: var(--ink);
  color: var(--paper);
  font-size: 10px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  font-weight: 700;
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
  position: sticky;
  top: 0;
}
table.run-table th:hover { background: color-mix(in oklch, var(--ink) 80%, var(--red)); }
table.run-table th.num, table.run-table td.num {
  text-align: right;
  font-variant-numeric: tabular-nums;
}
table.run-table th .sort-ind {
  color: var(--amber-soft);
  margin-left: 6px;
  font-weight: 700;
}
table.run-table tbody tr.row {
  cursor: pointer;
  transition: background 80ms;
}
table.run-table tbody tr.row:hover td { background: color-mix(in oklch, var(--amber) 8%, var(--paper)); }
table.run-table tbody tr.row.selected td { background: color-mix(in oklch, var(--amber) 22%, var(--paper)); }
table.run-table tbody tr.row.expanded td { background: var(--paper-shade); }
table.run-table .run-id {
  color: var(--red-deep);
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
table.run-table .run-id .swatch-dot {
  display: inline-block;
  width: 9px;
  height: 9px;
  border: 1px solid var(--ink);
  border-radius: 50%;
  flex-shrink: 0;
}
table.run-table .status-badge {
  display: inline-block;
  font-size: 9px;
  letter-spacing: 0.18em;
  padding: 1px 7px;
  border: 1px solid var(--rule);
  background: var(--paper);
  color: var(--ink-faint);
  font-weight: 700;
  text-transform: uppercase;
}
table.run-table .status-badge.invalid {
  background: var(--red);
  color: var(--paper);
  border-color: var(--red-deep);
}
table.run-table .shape-cell {
  font-size: 11px;
  color: var(--ink-soft);
  letter-spacing: 0.02em;
  white-space: nowrap;
}
table.run-table .num strong { font-weight: 700; color: var(--ink); }

table.run-table tr.expand-row > td {
  padding: 26px 24px 20px;
  background: var(--paper-shade);
  border-top: 1px dotted var(--ink);
}
.expand-grid {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.expand-grid .range-row {
  border-top: 1px dotted var(--rule);
  padding: 12px 0 10px;
  grid-template-columns: 170px minmax(0, 1fr) 280px;
  gap: 16px;
  align-items: center;
}
.expand-grid .range-row:first-child { border-top: none; padding-top: 4px; }
.expand-grid .range-row .range-label { font-size: 12px; }
.expand-grid .range-row .expand-chart { display: none; }
.expand-grid .range-row .sparkline { height: 52px; }
.expand-grid .range-row .sparkline svg { height: 52px; }
/* Hide the P50/P95/P99 top labels inside expand rows so they don't crowd the row above —
   the AVG/P50/P95/P99 chips below already carry the values. */
.expand-grid .range-track .tick::after { display: none; }
.expand-grid .range-stats { margin-top: 6px; }

/* OVERLAY ACTION BAR */
.overlay-bar {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%) translateY(140%);
  background: var(--ink);
  color: var(--paper);
  padding: 12px 16px 12px 20px;
  font-family: var(--mono);
  font-size: 12px;
  letter-spacing: 0.06em;
  display: flex;
  gap: 14px;
  align-items: center;
  z-index: 900;
  border: 1px solid var(--ink);
  box-shadow: 0 8px 28px rgba(0,0,0,0.32);
  transition: transform 240ms cubic-bezier(0.16, 1, 0.3, 1);
}
.overlay-bar.show { transform: translateX(-50%) translateY(0); }
.overlay-bar .count {
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--amber-soft);
  font-weight: 700;
}
.overlay-bar button {
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  font-weight: 700;
  padding: 6px 14px;
  cursor: pointer;
  border: 1px solid var(--paper);
}
.overlay-bar button.primary { background: var(--amber-soft); border-color: var(--amber-soft); color: var(--ink); }
.overlay-bar button.primary:hover { background: var(--paper); border-color: var(--paper); }
.overlay-bar button.ghost { background: transparent; color: var(--paper); }
.overlay-bar button.ghost:hover { background: var(--paper); color: var(--ink); }
.overlay-bar select {
  font-family: var(--mono);
  font-size: 11px;
  background: var(--paper);
  border: 1px solid var(--paper);
  padding: 5px 10px;
  color: var(--ink);
  letter-spacing: 0.04em;
  font-weight: 700;
}

/* Multi-line overlay chart variants */
.big-chart .data-line.overlay { fill: none; stroke-width: 1.6; vector-effect: non-scaling-stroke; opacity: 0.85; }
.big-chart .overlay-legend {
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
  font-family: var(--mono);
  font-size: 11px;
  margin-top: 18px;
  letter-spacing: 0.04em;
}
.big-chart .overlay-legend .item { display: inline-flex; gap: 8px; align-items: center; }
.big-chart .overlay-legend .swatch { width: 14px; height: 4px; }

/* FOOTER */
.foot {
  margin-top: 44px;
  padding-top: 16px;
  border-top: 2px solid var(--ink);
  font-family: var(--mono);
  font-size: 10px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--ink-faint);
  display: flex;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
}

@media (max-width: 780px) {
  .kpis { grid-template-columns: repeat(2, 1fr); }
  .kpi:nth-child(2) { border-right: none; }
  .kpi:nth-child(-n+2) { border-bottom: 1px solid var(--ink); }
  .bar-row { grid-template-columns: 1fr; }
  .bar-val { text-align: left; }
  .range-row { grid-template-columns: 1fr; }
}
</style>
</head>
<body>
<script id="report-data" type="application/json">__REPORT_JSON__</script>
<div class="tip" id="tip" role="tooltip" aria-hidden="true"></div>
<div class="chart-modal" id="chartModal" role="dialog" aria-modal="true" aria-label="Chart detail">
  <div class="titlebar">
    <span class="left">
      <strong id="chartTitle">CHART</strong>
      <small id="chartSubtitle"></small>
    </span>
    <div class="controls">
      <button type="button" data-act="close" class="close">&times;&nbsp;CLOSE</button>
    </div>
  </div>
  <div class="stage" id="chartStage"></div>
</div>
<div class="page">
  <header class="cover">
    <div class="top-strip">
      <span class="lhs" id="rev">FIG. RUN / REV --</span>
      <span class="rhs" id="meta-strip"></span>
    </div>
    <h1 class="title" id="title">queue<em>·</em>run</h1>
    <p class="subhead" id="subtitle"></p>
    <div class="run-meta" id="run-meta"></div>
    <div class="kpis" id="kpis"></div>
  </header>

  <nav class="toc" id="toc">
    <a href="#throughput"><span class="idx">01</span>Throughput</a>
    <a href="#timings"><span class="idx">02</span>Timings</a>
    <a href="#latency"><span class="idx">03</span>Latency</a>
    <a href="#health"><span class="idx">04</span>Health</a>
    <a href="#counters"><span class="idx">05</span>Counters</a>
    <a href="#config"><span class="idx">06</span>Config</a>
    <a href="#explain"><span class="idx">07</span>Plans</a>
  </nav>

  <main>
    <section class="section" id="throughput">
      <span class="marker">§ 01 — Throughput</span>
      <h2>Rates &amp; phase windows</h2>
      <div class="cols-2">
        <div class="frame">
          <span class="label">FIG. 1 — Rates</span>
          <div class="body">
            <div class="chart-legend">
              <span class="swatch"><i class="sw red"></i>produced</span>
              <span class="swatch"><i class="sw blue"></i>acked</span>
              <span class="swatch" style="margin-left:auto;color:var(--ink-faint)">hover / focus for exact value</span>
            </div>
            <div class="bar-list" id="rate-bars"></div>
          </div>
        </div>
        <div class="frame">
          <span class="label">FIG. 2 — Phase windows (seconds)</span>
          <div class="body">
            <div class="chart-legend">
              <span class="swatch"><i class="sw red"></i>producer window</span>
              <span class="swatch"><i class="sw blue"></i>total elapsed</span>
            </div>
            <div class="bar-list" id="phase-bars"></div>
          </div>
        </div>
      </div>
    </section>

    <section class="section" id="timings">
      <span class="marker">§ 02 — Operation timings</span>
      <h2>Per-op percentile distributions</h2>
      <p class="note">p50, p95, and p99 marked over each op's full ms range; the same samples drawn as a sparkline on the right.</p>
      <div class="frame">
        <span class="label">FIG. 3 — Per-op latency</span>
        <div class="body">
          <div class="chart-legend">
            <span class="swatch"><i class="sw range"></i>0 &rarr; max range</span>
            <span class="swatch"><i class="sw tick"></i>p50</span>
            <span class="swatch"><i class="sw tick amber"></i>p95</span>
            <span class="swatch"><i class="sw tick red"></i>p99</span>
            <span class="swatch" style="margin-left:auto;color:var(--ink-faint)">hover sparkline / track for sample values</span>
          </div>
          <div class="range-table" id="timing-ranges"></div>
        </div>
      </div>
    </section>

    <section class="section" id="latency">
      <span class="marker">§ 03 — End-to-end latency</span>
      <h2>Claimed-job latency · sent &rarr; ack</h2>
      <div class="frame">
        <span class="label">FIG. 4 — Job latency</span>
        <div class="body">
          <div class="chart-legend">
            <span class="swatch"><i class="sw range"></i>0 &rarr; max range</span>
            <span class="swatch"><i class="sw tick"></i>p50</span>
            <span class="swatch"><i class="sw tick amber"></i>p95</span>
            <span class="swatch"><i class="sw tick red"></i>p99</span>
          </div>
          <div class="range-table" id="latency-range"></div>
        </div>
      </div>
    </section>

    <section class="section" id="health">
      <span class="marker">§ 04 — Health</span>
      <h2>Drain &amp; error checks</h2>
      <div class="frame">
        <span class="label">FIG. 5 — Run health</span>
        <div class="body">
          <div class="issue-list" id="issues"></div>
        </div>
      </div>
    </section>

    <section class="section" id="counters">
      <span class="marker">§ 05 — Counters</span>
      <h2>Calls, errors, empty polls</h2>
      <div class="frame">
        <span class="label">FIG. 6 — Counter table</span>
        <div class="body">
          <div id="counter-table"></div>
        </div>
      </div>
    </section>

    <section class="section" id="config">
      <span class="marker">§ 06 — Config</span>
      <h2>Inputs that make the run comparable</h2>
      <div class="frame">
        <span class="label">FIG. 7 — Configuration</span>
        <div class="body">
          <div id="config-table"></div>
        </div>
      </div>
    </section>

    <section class="section" id="explain">
      <span class="marker">§ 07 — Query plans</span>
      <h2>EXPLAIN ANALYZE captures</h2>
      <p class="note">Hot read paths should show <code>RecordIdScan</code> &mdash; never table scans or secondary-index seeks.</p>
      <div id="explain-blocks"></div>
    </section>

    <div class="foot">
      <span id="footer-l">SURREAL // QUEUE // BENCH</span>
      <span id="footer-r"></span>
    </div>
  </main>
</div>

<script>
(function() {
  var report = JSON.parse(document.getElementById("report-data").textContent);
  var cfg = report.config || {};
  var sum = report.summary || {};
  var timings = report.timings || {};
  var latency = report.latency || {};
  var explain = report.explain || {};

  var opLabels = {
    producer_query:  "Producer insert",
    select_query:    "Receive claim",
    ack_query:       "Ack complete",
    register_worker: "Register worker",
    heartbeat:       "Heartbeat",
    bucket_refresh:  "Bucket refresh"
  };

  function $(id) { return document.getElementById(id); }
  function esc(v) {
    return String(v == null ? "" : v).replace(/[&<>"']/g, function(c) {
      return ({ "&":"&amp;", "<":"&lt;", ">":"&gt;", '"':"&quot;", "'":"&#39;" })[c];
    });
  }
  function finite(v) { return typeof v === "number" && Number.isFinite(v); }
  function fmtInt(v)  { return finite(v) ? Math.round(v).toLocaleString() : "0"; }
  function fmtRate(v) { return finite(v) ? Math.round(v).toLocaleString() + "/s" : "0/s"; }
  function fmtMs(v) {
    if (!finite(v)) return "-";
    if (v >= 1000) return (v / 1000).toFixed(2) + "s";
    if (v >= 10)   return v.toFixed(1) + "ms";
    return v.toFixed(3) + "ms";
  }
  function fmtSecs(v) { return finite(v) ? v.toFixed(v >= 10 ? 2 : 3) + "s" : "0.000s"; }
  function pct(v, m) {
    if (!finite(v) || !finite(m) || m <= 0) return 0;
    return Math.max(0, Math.min(100, (v / m) * 100));
  }

  function bar(label, value, max, alt, unit) {
    var safeUnit = unit ? " " + unit : "";
    return '<div class="bar-row">'
      + '<div class="bar-name">' + esc(label) + '</div>'
      + '<div class="bar-track" tabindex="0"'
      +   ' data-label="' + esc(label) + '"'
      +   ' data-value="' + (finite(value) ? value : 0) + '"'
      +   ' data-max="' + (finite(max) ? max : 0) + '"'
      +   ' data-unit="' + esc(safeUnit) + '">'
      +   '<span class="bar-fill ' + (alt ? "alt" : "") + '" style="--w:' + pct(value, max).toFixed(2) + '%"></span>'
      + '</div>'
      + '<div class="bar-val">' + esc(finite(value) ? value.toLocaleString(undefined, { maximumFractionDigits: 2 }) : "-") + safeUnit + '</div>'
      + '</div>';
  }

  function statChip(label, value) {
    return '<div class="stat-chip"><span>' + esc(label) + '</span><strong>' + esc(value) + '</strong></div>';
  }

  function rangeRow(label, timing, sublabel) {
    timing = timing || {};
    var count = Number(timing.count) || 0;
    var max = Number(timing.max_ms) || 0;
    var min = Number(timing.min_ms) || 0;
    var p50 = Number(timing.p50_ms) || 0;
    var p95 = Number(timing.p95_ms) || 0;
    var p99 = Number(timing.p99_ms) || 0;
    var avg = Number(timing.avg_ms) || Number(timing.mean_ms) || 0;
    var samples = Array.isArray(timing.samples_ms) ? timing.samples_ms : [];
    var samplesAttr = samples.length ? ' data-samples="' + samples.join(",") + '"' : '';
    return '<div class="range-row"'
      + ' data-op="' + esc(label) + '"'
      + ' data-max="' + max + '"'
      + ' data-min="' + min + '"'
      + ' data-p50="' + p50 + '"'
      + ' data-p95="' + p95 + '"'
      + ' data-p99="' + p99 + '"'
      + ' data-avg="' + avg + '"'
      + ' data-count="' + count + '"'
      + samplesAttr + '>'
      + '<div class="range-label">' + esc(label)
      +   '<span class="range-sub">' + esc(sublabel || (fmtInt(count) + " samples")) + '</span>'
      +   '<button type="button" class="expand-chart" data-expand>&#x2922;&nbsp;EXPAND</button>'
      + '</div>'
      + '<div>'
      +   '<div class="range-track" tabindex="0" role="img" aria-label="' + esc(label) + ' percentile distribution">'
      +     '<span class="range-fill" style="--w:' + pct(p95 || avg || max, max).toFixed(2) + '%"></span>'
      +     '<span class="tick" data-label="p50" data-value="' + p50 + '" style="left:' + pct(p50, max).toFixed(2) + '%"></span>'
      +     '<span class="tick p95" data-label="p95" data-value="' + p95 + '" style="left:' + pct(p95, max).toFixed(2) + '%"></span>'
      +     '<span class="tick p99" data-label="p99" data-value="' + p99 + '" style="left:' + pct(p99, max).toFixed(2) + '%"></span>'
      +   '</div>'
      +   '<div class="range-stats">'
      +     statChip("avg", fmtMs(avg))
      +     statChip("p50", fmtMs(p50))
      +     statChip("p95", fmtMs(p95))
      +     statChip("p99", fmtMs(p99))
      +   '</div>'
      + '</div>'
      + '<div class="sparkline">' + sparkline(samples) + '</div>'
      + '</div>';
  }

  function sparkline(values) {
    var nums = (values || []).filter(finite);
    if (nums.length < 2) {
      return '<svg viewBox="0 0 420 70" role="img"><text x="14" y="40" opacity="0.6">no samples</text></svg>';
    }
    var w = 420, h = 70;
    var min = nums[0], max = nums[0];
    for (var i = 1; i < nums.length; i++) {
      if (nums[i] < min) min = nums[i];
      if (nums[i] > max) max = nums[i];
    }
    if (max === min) max = min + 1;
    var pts = [];
    for (var j = 0; j < nums.length; j++) {
      var x = nums.length === 1 ? 0 : (j / (nums.length - 1)) * w;
      var y = h - ((nums[j] - min) / (max - min)) * (h - 14) - 7;
      pts.push(x.toFixed(1) + "," + y.toFixed(1));
    }
    return '<svg viewBox="0 0 ' + w + ' ' + h + '" preserveAspectRatio="none" role="img">'
      + '<path class="grid" d="M0 ' + (h / 2).toFixed(1) + 'H' + w + '"></path>'
      + '<polyline points="' + pts.join(" ") + '"></polyline>'
      + '</svg>';
  }

  function renderTable(headers, rows) {
    var thead = '<thead><tr>' + headers.map(function(h) {
      var cls = typeof h === "object" && h.num ? ' class="num"' : '';
      var label = typeof h === "object" ? h.label : h;
      return '<th' + cls + '>' + esc(label) + '</th>';
    }).join("") + '</tr></thead>';
    var tbody = '<tbody>' + rows.map(function(row) {
      return '<tr>' + row.map(function(cell) {
        if (cell && typeof cell === "object") {
          return '<td class="num">' + cell.html + '</td>';
        }
        return '<td>' + cell + '</td>';
      }).join("") + '</tr>';
    }).join("") + '</tbody>';
    return '<table class="spec">' + thead + tbody + '</table>';
  }

  function configRows() {
    return [
      ["mode", cfg.mode],
      ["queue_model", cfg.queue_model],
      ["num_total_jobs", cfg.num_total_jobs],
      ["producers", cfg.producers],
      ["consumers", cfg.consumers],
      ["producer_batch_size", cfg.producer_batch_size],
      ["receive_batch_size", cfg.receive_batch_size],
      ["buckets", cfg.buckets],
      ["payload_bytes", cfg.payload_bytes],
      ["lease_secs", cfg.lease_secs],
      ["job_ms", cfg.job_ms],
      ["fixed_workdown", cfg.fixed_workdown],
      ["separate_clients", cfg.separate_clients],
      ["endpoint", cfg.endpoint],
      ["namespace", cfg.namespace],
      ["database", cfg.database]
    ];
  }

  function renderIssues() {
    var issues = [];
    var errors = (sum.producer_errors||0) + (sum.receive_errors||0) + (sum.ack_errors||0);
    if (errors > 0) issues.push(["bad", "Errors recorded", errors + " producer/receive/ack errors observed."]);
    if (!sum.drained) issues.push(["bad", "Run did not drain", fmtInt(sum.final_backlog) + " jobs remain in backlog."]);
    if ((sum.produced_total||0) !== (sum.acked_total||0)) issues.push(["warn", "Produced and acked differ", fmtInt(sum.produced_total) + " produced vs " + fmtInt(sum.acked_total) + " acked."]);
    if ((sum.empty_polls||0) > (sum.receive_calls_completed||0) * 0.5) issues.push(["warn", "High empty-poll ratio", fmtInt(sum.empty_polls) + " empty polls for " + fmtInt(sum.receive_calls_completed) + " receive calls."]);
    if (!issues.length) issues.push(["", "Clean run", "Drained with matching produced and acked counts and no recorded operation errors."]);
    $("issues").innerHTML = issues.map(function(it) {
      return '<div class="issue ' + it[0] + '"><strong>' + esc(it[1]) + '</strong><span>' + esc(it[2]) + '</span></div>';
    }).join("");
  }

  function renderExplain() {
    var blocks = [
      ["receive_ready_range", explain.receive_ready_range],
      ["recover_lease_range", explain.recover_lease_range],
      ["bucket_owner_lookup", explain.bucket_owner_lookup]
    ].filter(function(p) { return p[1]; });
    var errors = Array.isArray(explain.errors) ? explain.errors : [];
    var html = blocks.map(function(b, i) {
      return '<details ' + (i === 0 ? "open" : "") + '><summary>' + esc(b[0]) + '</summary><pre>' + esc(b[1]) + '</pre></details>';
    }).join("");
    if (errors.length) {
      html += '<details open><summary>explain_errors</summary><pre>' + esc(errors.join("\n")) + '</pre></details>';
    }
    if (!html) {
      html = '<div class="frame"><div class="body" style="font-family:var(--serif);color:var(--ink-faint)">No query plans were captured for this run.</div></div>';
    }
    $("explain-blocks").innerHTML = html;
  }

  function renderCounters() {
    var rows = [
      ["Producer batches", sum.producer_batches_started, sum.producer_batches_completed, sum.producer_batches_in_flight, sum.producer_errors],
      ["Receive calls",    sum.receive_calls_started,    sum.receive_calls_completed,    sum.receive_calls_in_flight,    sum.receive_errors],
      ["Ack batches",      sum.ack_batches_started,      sum.ack_batches_completed,      sum.ack_batches_in_flight,      sum.ack_errors],
      ["Empty polls",      sum.empty_polls,              null, null, null],
      ["Bucket refreshes", sum.bucket_refreshes,         null, null, null]
    ].map(function(r) {
      function cell(v) { return v == null ? { html: "&mdash;" } : { html: esc(fmtInt(Number(v) || 0)) }; }
      return [
        '<code>' + esc(r[0]) + '</code>',
        cell(r[1]), cell(r[2]), cell(r[3]), cell(r[4])
      ];
    });
    $("counter-table").innerHTML = renderTable([
      "Counter",
      { label: "Started", num: true },
      { label: "Completed", num: true },
      { label: "In flight", num: true },
      { label: "Errors", num: true }
    ], rows);
  }

  // ---- TOOLTIP + INTERACTIVITY ----

  var tip = document.getElementById("tip");
  var tipHideTimer = null;
  function showTip(html, x, y) {
    if (tipHideTimer) { clearTimeout(tipHideTimer); tipHideTimer = null; }
    tip.innerHTML = html;
    tip.setAttribute("aria-hidden", "false");
    tip.classList.add("show");
    var rect = tip.getBoundingClientRect();
    var w = rect.width, h = rect.height;
    var vw = window.innerWidth, vh = window.innerHeight;
    var left = x + 14;
    var top  = y + 16;
    if (left + w + 8 > vw) left = x - w - 14;
    if (top + h + 8 > vh)  top  = y - h - 16;
    if (left < 8) left = 8;
    if (top  < 8) top  = 8;
    tip.style.left = left + "px";
    tip.style.top  = top + "px";
  }
  function hideTip() {
    tipHideTimer = setTimeout(function() {
      tip.classList.remove("show");
      tip.setAttribute("aria-hidden", "true");
    }, 60);
  }
  function pointer(e) {
    if (e.touches && e.touches.length) return { x: e.touches[0].clientX, y: e.touches[0].clientY };
    if (typeof e.clientX === "number") return { x: e.clientX, y: e.clientY };
    return null;
  }

  function setupBar(track) {
    var value = parseFloat(track.dataset.value);
    var max   = parseFloat(track.dataset.max);
    var label = track.dataset.label || "";
    var unit  = (track.dataset.unit || "").trim();
    var frac  = max > 0 ? (value / max) * 100 : 0;
    var ariaUnit = unit ? " " + unit : "";
    track.setAttribute("aria-label",
      label + ": " + (finite(value) ? value.toLocaleString(undefined, { maximumFractionDigits: 4 }) : "-") + ariaUnit +
      (max > 0 ? " (" + frac.toFixed(1) + "% of max " + max.toLocaleString(undefined, { maximumFractionDigits: 4 }) + ariaUnit + ")" : ""));
    var show = function(e) {
      var p = pointer(e);
      if (!p) {
        var r = track.getBoundingClientRect();
        p = { x: r.right, y: r.top + r.height / 2 };
      }
      var html = '<strong>' + esc(label) + '</strong>'
        + '<div class="row"><span>value</span><span>'
        +   esc(value.toLocaleString(undefined, { maximumFractionDigits: 4 })) + esc(ariaUnit) + '</span></div>'
        + '<div class="row"><span>max</span><span>'
        +   esc(max.toLocaleString(undefined, { maximumFractionDigits: 4 })) + esc(ariaUnit) + '</span></div>'
        + '<div class="row"><span>fraction</span><span>' + frac.toFixed(2) + '%</span></div>';
      showTip(html, p.x, p.y);
    };
    track.addEventListener("mouseenter", show);
    track.addEventListener("mousemove", show);
    track.addEventListener("focus", show);
    track.addEventListener("mouseleave", hideTip);
    track.addEventListener("blur", hideTip);
  }

  function setupRangeRow(row) {
    var max = parseFloat(row.dataset.max) || 0;
    var avg = parseFloat(row.dataset.avg) || 0;
    var p50 = parseFloat(row.dataset.p50) || 0;
    var p95 = parseFloat(row.dataset.p95) || 0;
    var p99 = parseFloat(row.dataset.p99) || 0;
    var min = parseFloat(row.dataset.min) || 0;
    var count = parseInt(row.dataset.count, 10) || 0;
    var op  = row.dataset.op || "";
    var track = row.querySelector(".range-track");
    var ticks = row.querySelectorAll(".range-track .tick");

    // Per-tick focus + tooltip with the real percentile value
    ticks.forEach(function(t) {
      var lbl = t.dataset.label || "";
      var v   = parseFloat(t.dataset.value) || 0;
      t.setAttribute("tabindex", "0");
      t.setAttribute("role", "button");
      t.setAttribute("aria-label", op + " " + lbl + " = " + fmtMs(v));
      var show = function(e) {
        var p = pointer(e);
        if (!p) {
          var r = t.getBoundingClientRect();
          p = { x: r.left + r.width / 2, y: r.top };
        }
        var html = '<strong>' + esc(op) + ' &middot; ' + esc(lbl) + '</strong>'
          + '<div class="row"><span>value</span><span>' + esc(fmtMs(v)) + '</span></div>'
          + '<div class="row"><span>fraction</span><span>' + (max > 0 ? ((v / max) * 100).toFixed(1) + "%" : "-") + '</span></div>';
        showTip(html, p.x, p.y);
        if (e && e.stopPropagation) e.stopPropagation();
      };
      t.addEventListener("mouseenter", show);
      t.addEventListener("mousemove", show);
      t.addEventListener("focus", show);
      t.addEventListener("mouseleave", hideTip);
      t.addEventListener("blur", hideTip);
    });

    // Hovering the track shows the linear value at cursor position
    if (track) {
      var trackShow = function(e) {
        if (max <= 0) return;
        var r = track.getBoundingClientRect();
        var p = pointer(e) || { x: r.left + r.width / 2, y: r.top };
        var frac = Math.max(0, Math.min(1, (p.x - r.left) / r.width));
        var ms = frac * max;
        var html = '<strong>' + esc(op) + ' &middot; position</strong>'
          + '<div class="row"><span>at cursor</span><span>&asymp; ' + esc(fmtMs(ms)) + '</span></div>'
          + '<div class="row"><span>min</span><span>' + esc(fmtMs(min)) + '</span></div>'
          + '<div class="row"><span>avg</span><span>' + esc(fmtMs(avg)) + '</span></div>'
          + '<div class="row"><span>max</span><span>' + esc(fmtMs(max)) + '</span></div>'
          + '<div class="row"><span>samples</span><span>' + fmtInt(count) + '</span></div>';
        showTip(html, p.x, p.y);
      };
      track.addEventListener("mousemove", trackShow);
      track.addEventListener("mouseleave", hideTip);
      track.addEventListener("focus", function() {
        var r = track.getBoundingClientRect();
        trackShow({ clientX: r.left + r.width / 2, clientY: r.top });
      });
      track.addEventListener("blur", hideTip);
      track.addEventListener("keydown", function(e) {
        if (e.key === "Escape") track.blur();
      });
    }

    // Sparkline interactivity
    var svg = row.querySelector(".sparkline svg");
    var samples = (row.dataset.samples || "").split(",").filter(Boolean).map(parseFloat).filter(finite);
    if (svg && samples.length > 1) setupSparkline(svg, samples, op);
  }

  function setupSparkline(svg, samples, op) {
    var ns = "http://www.w3.org/2000/svg";
    var w = 420, h = 70;
    var min = samples[0], max = samples[0];
    for (var i = 1; i < samples.length; i++) {
      if (samples[i] < min) min = samples[i];
      if (samples[i] > max) max = samples[i];
    }
    if (max === min) max = min + 1;
    function yFor(v) { return h - ((v - min) / (max - min)) * (h - 14) - 7; }

    var overlay = document.createElementNS(ns, "rect");
    overlay.setAttribute("x", "0"); overlay.setAttribute("y", "0");
    overlay.setAttribute("width", String(w)); overlay.setAttribute("height", String(h));
    overlay.setAttribute("fill", "transparent");
    overlay.setAttribute("class", "hover-overlay");
    overlay.setAttribute("pointer-events", "all");

    var line = document.createElementNS(ns, "line");
    line.setAttribute("class", "crosshair");
    line.setAttribute("y1", "0"); line.setAttribute("y2", String(h));
    line.style.display = "none";

    var dot = document.createElementNS(ns, "circle");
    dot.setAttribute("class", "crosspoint");
    dot.setAttribute("r", "3.5");
    dot.style.display = "none";

    svg.appendChild(overlay);
    svg.appendChild(line);
    svg.appendChild(dot);
    svg.setAttribute("tabindex", "0");
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", op + " sparkline: " + samples.length + " samples, min " + fmtMs(min) + ", max " + fmtMs(max));

    function setHover(idx, clientX, clientY) {
      idx = Math.max(0, Math.min(samples.length - 1, idx));
      var v = samples[idx];
      var x = samples.length === 1 ? 0 : (idx / (samples.length - 1)) * w;
      var y = yFor(v);
      line.setAttribute("x1", String(x));
      line.setAttribute("x2", String(x));
      line.style.display = "";
      dot.setAttribute("cx", String(x));
      dot.setAttribute("cy", String(y));
      dot.style.display = "";
      var html = '<strong>' + esc(op) + ' &middot; sample</strong>'
        + '<div class="row"><span>value</span><span>' + esc(fmtMs(v)) + '</span></div>'
        + '<div class="row"><span>index</span><span>' + (idx + 1) + ' / ' + samples.length + '</span></div>'
        + '<div class="row"><span>min</span><span>' + esc(fmtMs(min)) + '</span></div>'
        + '<div class="row"><span>max</span><span>' + esc(fmtMs(max)) + '</span></div>';
      showTip(html, clientX, clientY);
    }
    function hide() {
      line.style.display = "none";
      dot.style.display = "none";
      hideTip();
    }
    function onPoint(e) {
      var p = pointer(e);
      if (!p) return;
      var r = svg.getBoundingClientRect();
      var pctX = Math.max(0, Math.min(1, (p.x - r.left) / r.width));
      var idx = Math.round(pctX * (samples.length - 1));
      setHover(idx, p.x, p.y);
      if (e.cancelable) e.preventDefault();
    }
    overlay.addEventListener("mousemove", onPoint);
    overlay.addEventListener("mouseleave", hide);
    overlay.addEventListener("touchstart", onPoint, { passive: false });
    overlay.addEventListener("touchmove",  onPoint, { passive: false });
    overlay.addEventListener("touchend",   hide);

    // keyboard
    var kbIdx = Math.floor(samples.length / 2);
    svg.addEventListener("focus", function() {
      var r = svg.getBoundingClientRect();
      var x = r.left + (kbIdx / Math.max(1, samples.length - 1)) * r.width;
      setHover(kbIdx, x, r.top);
    });
    svg.addEventListener("blur", hide);
    svg.addEventListener("keydown", function(e) {
      if (e.key === "ArrowLeft")  { kbIdx = Math.max(0, kbIdx - 1); e.preventDefault(); }
      else if (e.key === "ArrowRight") { kbIdx = Math.min(samples.length - 1, kbIdx + 1); e.preventDefault(); }
      else if (e.key === "Home")  { kbIdx = 0; e.preventDefault(); }
      else if (e.key === "End")   { kbIdx = samples.length - 1; e.preventDefault(); }
      else if (e.key === "Escape") { svg.blur(); return; }
      else return;
      var r = svg.getBoundingClientRect();
      var x = r.left + (kbIdx / Math.max(1, samples.length - 1)) * r.width;
      setHover(kbIdx, x, r.top);
    });
  }

  // ---- BIG CHART MODAL ----

  var chartModal = document.getElementById("chartModal");
  var chartStage = document.getElementById("chartStage");
  var chartTitleEl = document.getElementById("chartTitle");
  var chartSubtitleEl = document.getElementById("chartSubtitle");

  function openChart(rowData) {
    chartTitleEl.textContent = rowData.op || "chart";
    chartSubtitleEl.textContent = fmtInt(rowData.count) + " samples · min " + fmtMs(rowData.min) + " · avg " + fmtMs(rowData.avg) + " · max " + fmtMs(rowData.max);
    chartStage.innerHTML = "";
    chartStage.appendChild(buildBigChart(rowData));
    chartModal.classList.add("open");
    hideTip();
    var closer = chartModal.querySelector("button.close");
    if (closer) closer.focus();
  }
  function closeChart() { chartModal.classList.remove("open"); }

  chartModal.addEventListener("click", function(e) {
    if (e.target === chartModal) closeChart();
    var btn = e.target.closest && e.target.closest("button[data-act='close']");
    if (btn) closeChart();
  });
  document.addEventListener("keydown", function(e) {
    if (e.key === "Escape" && chartModal.classList.contains("open")) closeChart();
  });

  function buildBigChart(data) {
    var ns = "http://www.w3.org/2000/svg";
    var wrap = document.createElement("div");
    wrap.className = "big-chart";

    var samples = (data.samples && data.samples.length) ? data.samples : [];
    var hasSamples = samples.length > 1;

    // META row
    var meta = document.createElement("div");
    meta.className = "meta";
    meta.innerHTML = '<span>' + esc(data.op) + '</span>'
      + '<span>' + (hasSamples ? samples.length + " sampled points" : "no sample trace · percentile guides only") + '</span>';
    wrap.appendChild(meta);

    // viewbox dims
    var W = 1400, H = 520;
    var padL = 90, padR = 40, padT = 40, padB = 70;
    var iw = W - padL - padR;
    var ih = H - padT - padB;

    // y scale
    var yMin = 0;
    var yMax = data.max || 0;
    if (hasSamples) {
      for (var i = 0; i < samples.length; i++) if (samples[i] > yMax) yMax = samples[i];
    }
    if (data.p99 > yMax) yMax = data.p99;
    if (yMax <= 0) yMax = 1;

    function xFor(i) { return padL + (samples.length <= 1 ? 0 : (i / (samples.length - 1)) * iw); }
    function yFor(v) { return padT + ih - ((v - yMin) / (yMax - yMin)) * ih; }

    var svg = document.createElementNS(ns, "svg");
    svg.setAttribute("class", "canvas");
    svg.setAttribute("viewBox", "0 0 " + W + " " + H);
    svg.setAttribute("preserveAspectRatio", "xMidYMid meet");
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", data.op + " latency chart");

    // y gridlines + labels
    var yTicks = 6;
    for (var ti = 0; ti <= yTicks; ti++) {
      var yv = yMin + ((yMax - yMin) * ti) / yTicks;
      var yPx = yFor(yv);
      var gl = document.createElementNS(ns, "line");
      gl.setAttribute("class", "gridline");
      gl.setAttribute("x1", padL); gl.setAttribute("x2", padL + iw);
      gl.setAttribute("y1", yPx); gl.setAttribute("y2", yPx);
      svg.appendChild(gl);
      var lbl = document.createElementNS(ns, "text");
      lbl.setAttribute("class", "axis-label");
      lbl.setAttribute("x", padL - 12);
      lbl.setAttribute("y", yPx + 4);
      lbl.setAttribute("text-anchor", "end");
      lbl.textContent = fmtMs(yv);
      svg.appendChild(lbl);
    }

    // percentile lines
    ["p50", "p95", "p99"].forEach(function(k) {
      var v = data[k];
      if (!v || v < yMin || v > yMax) return;
      var y = yFor(v);
      var l = document.createElementNS(ns, "line");
      l.setAttribute("class", "percentile " + k);
      l.setAttribute("x1", padL); l.setAttribute("x2", padL + iw);
      l.setAttribute("y1", y); l.setAttribute("y2", y);
      svg.appendChild(l);
      var t = document.createElementNS(ns, "text");
      t.setAttribute("class", "percentile-label " + k);
      t.setAttribute("x", padL + iw - 6);
      t.setAttribute("y", y - 6);
      t.setAttribute("text-anchor", "end");
      t.textContent = k.toUpperCase() + " = " + fmtMs(v);
      svg.appendChild(t);
    });

    // data area + line
    if (hasSamples) {
      var pathPts = [];
      var areaPts = [padL + ",0"]; // placeholder, we'll rewrite below
      areaPts = [];
      for (var pi = 0; pi < samples.length; pi++) {
        var px = xFor(pi);
        var py = yFor(samples[pi]);
        pathPts.push(px + "," + py);
        areaPts.push(px + "," + py);
      }
      var area = document.createElementNS(ns, "polygon");
      area.setAttribute("class", "data-area");
      area.setAttribute("points", areaPts.concat([
        (padL + iw) + "," + (padT + ih),
        padL + "," + (padT + ih)
      ]).join(" "));
      svg.appendChild(area);

      var line = document.createElementNS(ns, "polyline");
      line.setAttribute("class", "data-line");
      line.setAttribute("points", pathPts.join(" "));
      svg.appendChild(line);

      // points if sparse enough
      if (samples.length <= 60) {
        for (var pj = 0; pj < samples.length; pj++) {
          var c = document.createElementNS(ns, "circle");
          c.setAttribute("class", "data-point");
          c.setAttribute("cx", xFor(pj));
          c.setAttribute("cy", yFor(samples[pj]));
          c.setAttribute("r", "3.5");
          svg.appendChild(c);
        }
      }
    } else {
      var note = document.createElementNS(ns, "text");
      note.setAttribute("class", "axis-label");
      note.setAttribute("x", padL + iw / 2);
      note.setAttribute("y", padT + ih / 2);
      note.setAttribute("text-anchor", "middle");
      note.setAttribute("font-style", "italic");
      note.textContent = "no per-sample trace captured · percentile lines above";
      svg.appendChild(note);
    }

    // axes
    var xax = document.createElementNS(ns, "line");
    xax.setAttribute("class", "axis-line");
    xax.setAttribute("x1", padL); xax.setAttribute("x2", padL + iw);
    xax.setAttribute("y1", padT + ih); xax.setAttribute("y2", padT + ih);
    svg.appendChild(xax);
    var yax = document.createElementNS(ns, "line");
    yax.setAttribute("class", "axis-line");
    yax.setAttribute("x1", padL); yax.setAttribute("x2", padL);
    yax.setAttribute("y1", padT); yax.setAttribute("y2", padT + ih);
    svg.appendChild(yax);

    // x-axis labels
    if (hasSamples) {
      [0, 0.25, 0.5, 0.75, 1].forEach(function(f) {
        var idx = Math.round(f * (samples.length - 1));
        var px = xFor(idx);
        var t = document.createElementNS(ns, "text");
        t.setAttribute("class", "axis-label");
        t.setAttribute("x", px); t.setAttribute("y", padT + ih + 22);
        t.setAttribute("text-anchor", "middle");
        t.textContent = "#" + (idx + 1);
        svg.appendChild(t);
      });
    }

    // axis titles
    var xt = document.createElementNS(ns, "text");
    xt.setAttribute("class", "axis-title");
    xt.setAttribute("x", padL + iw / 2);
    xt.setAttribute("y", H - 22);
    xt.setAttribute("text-anchor", "middle");
    xt.textContent = hasSamples ? "SAMPLE INDEX" : "TIME";
    svg.appendChild(xt);

    var yt = document.createElementNS(ns, "text");
    yt.setAttribute("class", "axis-title");
    yt.setAttribute("x", -(padT + ih / 2));
    yt.setAttribute("y", 22);
    yt.setAttribute("text-anchor", "middle");
    yt.setAttribute("transform", "rotate(-90)");
    yt.textContent = "LATENCY (MS)";
    svg.appendChild(yt);

    // cursor crosshair + overlay
    if (hasSamples) {
      var cursorLine = document.createElementNS(ns, "line");
      cursorLine.setAttribute("class", "cursor-line");
      cursorLine.setAttribute("y1", padT); cursorLine.setAttribute("y2", padT + ih);
      cursorLine.style.display = "none";
      svg.appendChild(cursorLine);
      var cursorDot = document.createElementNS(ns, "circle");
      cursorDot.setAttribute("class", "cursor-point");
      cursorDot.setAttribute("r", "5");
      cursorDot.style.display = "none";
      svg.appendChild(cursorDot);
      var overlay = document.createElementNS(ns, "rect");
      overlay.setAttribute("class", "hover-overlay");
      overlay.setAttribute("x", padL); overlay.setAttribute("y", padT);
      overlay.setAttribute("width", iw); overlay.setAttribute("height", ih);
      svg.appendChild(overlay);

      function onMove(e) {
        var rect = svg.getBoundingClientRect();
        var sx = (e.touches ? e.touches[0].clientX : e.clientX) - rect.left;
        var scale = rect.width / W;
        var localX = sx / scale;
        var fracX = Math.max(0, Math.min(1, (localX - padL) / iw));
        var idx = Math.round(fracX * (samples.length - 1));
        var v = samples[idx];
        var cx = xFor(idx);
        var cy = yFor(v);
        cursorLine.setAttribute("x1", cx); cursorLine.setAttribute("x2", cx);
        cursorLine.style.display = "";
        cursorDot.setAttribute("cx", cx); cursorDot.setAttribute("cy", cy);
        cursorDot.style.display = "";
        var html = '<strong>' + esc(data.op) + ' &middot; sample</strong>'
          + '<div class="row"><span>value</span><span>' + esc(fmtMs(v)) + '</span></div>'
          + '<div class="row"><span>index</span><span>' + (idx + 1) + ' / ' + samples.length + '</span></div>';
        showTip(html, e.clientX || rect.left + sx, e.clientY || rect.top + cy * scale);
        if (e.cancelable) e.preventDefault();
      }
      function onLeave() {
        cursorLine.style.display = "none";
        cursorDot.style.display = "none";
        hideTip();
      }
      overlay.addEventListener("mousemove", onMove);
      overlay.addEventListener("mouseleave", onLeave);
      overlay.addEventListener("touchmove", onMove, { passive: false });
      overlay.addEventListener("touchend", onLeave);
    }

    wrap.appendChild(svg);

    // summary stat strip
    var summary = document.createElement("div");
    summary.className = "summary";
    var statDefs = [
      ["count", fmtInt(data.count)],
      ["min", fmtMs(data.min)],
      ["avg", fmtMs(data.avg)],
      ["p50", fmtMs(data.p50)],
      ["p95", fmtMs(data.p95)],
      ["p99", fmtMs(data.p99)],
      ["max", fmtMs(data.max)]
    ];
    summary.innerHTML = statDefs.map(function(d) {
      return '<div class="stat"><div class="lbl">' + esc(d[0]) + '</div><div class="val">' + esc(d[1]) + '</div></div>';
    }).join("");
    wrap.appendChild(summary);

    return wrap;
  }

  function rowDataFrom(row) {
    return {
      op: row.dataset.op || "",
      samples: (row.dataset.samples || "").split(",").filter(Boolean).map(parseFloat).filter(finite),
      count: parseInt(row.dataset.count, 10) || 0,
      min: parseFloat(row.dataset.min) || 0,
      avg: parseFloat(row.dataset.avg) || 0,
      p50: parseFloat(row.dataset.p50) || 0,
      p95: parseFloat(row.dataset.p95) || 0,
      p99: parseFloat(row.dataset.p99) || 0,
      max: parseFloat(row.dataset.max) || 0
    };
  }

  function initInteractivity() {
    document.querySelectorAll(".bar-track").forEach(setupBar);
    document.querySelectorAll(".range-row").forEach(function(row) {
      setupRangeRow(row);
      var open = function(e) {
        openChart(rowDataFrom(row));
        if (e && e.preventDefault) e.preventDefault();
        if (e && e.stopPropagation) e.stopPropagation();
      };
      var spark = row.querySelector(".sparkline svg");
      if (spark) spark.addEventListener("click", open);
      var track = row.querySelector(".range-track");
      if (track) track.addEventListener("click", open);
      var btn = row.querySelector("[data-expand]");
      if (btn) btn.addEventListener("click", open);
      // keyboard: Enter on a focusable element inside the row opens the chart
      [spark, track, btn].forEach(function(el) {
        if (!el) return;
        el.addEventListener("keydown", function(e) {
          if (e.key === "Enter") open(e);
        });
      });
    });
  }

  function initToc() {
    var toc = $("toc");
    var links = Array.prototype.slice.call(toc.querySelectorAll("a"));
    var sections = links.map(function(link) {
      return { link: link, el: document.getElementById(link.getAttribute("href").slice(1)) };
    }).filter(function(s) { return s.el; });
    if (typeof IntersectionObserver === "undefined") return;
    var io = new IntersectionObserver(function(entries) {
      entries.forEach(function(e) {
        if (!e.isIntersecting) return;
        links.forEach(function(l) { l.classList.remove("active"); });
        var m = sections.find(function(s) { return s.el === e.target; });
        if (m) m.link.classList.add("active");
      });
    }, { rootMargin: "-15% 0px -75% 0px" });
    sections.forEach(function(s) { io.observe(s.el); });
  }

  function avgOp(op) {
    return op && finite(op.avg_ms) ? op.avg_ms : 0;
  }

  function median(values) {
    var xs = values.filter(finite).sort(function(a, b) { return a - b; });
    if (!xs.length) return 0;
    return xs[Math.floor(xs.length / 2)];
  }

  function matrixRuns(data) {
    return (data.runs || []).map(function(row) {
      var runReport = row.report || {};
      var c = runReport.config || {};
      var s = runReport.summary || {};
      var t = runReport.timings || {};
      var latencyReport = runReport.latency || {};
      var mode = c.mode || "";
      var isInsertOnly = mode === "insert_only";
      var producedTotal = s.produced_total || 0;
      var ackedTotal = s.acked_total || 0;
      var valid;
      if (isInsertOnly) {
        valid = (s.producer_errors || 0) === 0
          && producedTotal > 0;
      } else {
        valid = producedTotal > 0
          && ackedTotal >= producedTotal
          && (s.producer_errors || 0) === 0
          && (s.receive_errors || 0) === 0
          && (s.ack_errors || 0) === 0;
      }
      var duplicateAcks = Math.max(0, ackedTotal - producedTotal);
      return {
        run_index: row.run_index || 0,
        repeat: row.repeat || 0,
        report: runReport,
        mode: c.mode || "",
        tasks: c.num_total_jobs || 0,
        producers: c.producers || 0,
        consumers: c.consumers || 0,
        buckets: c.buckets || 0,
        producer_batch: c.producer_batch_size || 0,
        receive_batch: c.receive_batch_size || 0,
        job_ms: c.job_ms || 0,
        produced_total: s.produced_total || 0,
        acked_total: s.acked_total || 0,
        produced_per_sec: s.produced_per_sec || 0,
        acked_per_sec: s.acked_per_sec || 0,
        elapsed_secs: s.elapsed_secs || 0,
        empty_polls: s.empty_polls || 0,
        receive_calls: s.receive_calls_completed || 0,
        ack_batches: s.ack_batches_completed || 0,
        producer_avg_ms: avgOp(t.producer_query),
        select_avg_ms: avgOp(t.select_query),
        ack_avg_ms: avgOp(t.ack_query),
        latency_p95_ms: latencyReport.p95_ms,
        valid: valid,
        final_backlog: s.final_backlog || 0
      };
    });
  }

  function setSection(id, marker, title, note, label) {
    var section = document.getElementById(id);
    if (!section) return;
    var markerEl = section.querySelector(".marker");
    var titleEl = section.querySelector("h2");
    var noteEl = section.querySelector(".note");
    var labelEl = section.querySelector(".frame .label");
    if (markerEl) markerEl.textContent = marker;
    if (titleEl) titleEl.textContent = title;
    if (noteEl) noteEl.textContent = note || "";
    if (labelEl) labelEl.textContent = label || "";
    if (labelEl && !label) labelEl.style.display = "none";
    else if (labelEl) labelEl.style.display = "";
  }

  // =============== MATRIX REPORT ===============

  var MATRIX_PALETTE = [
    "#b3322a", "#2a4a73", "#a67517", "#5b6233",
    "#872019", "#1a3354", "#4d4a4f", "#d4a843"
  ];
  var MATRIX_PARAMS = [
    { key: "tasks",          label: "tasks",      axis: "TASKS" },
    { key: "producers",      label: "producers",  axis: "PRODUCERS" },
    { key: "consumers",      label: "workers",    axis: "WORKERS" },
    { key: "buckets",        label: "buckets",    axis: "BUCKETS" },
    { key: "producer_batch", label: "prod batch", axis: "PRODUCER BATCH" },
    { key: "receive_batch",  label: "recv batch", axis: "RECEIVE BATCH" },
    { key: "job_ms",         label: "job ms",     axis: "JOB MS" },
    { key: "mode",           label: "mode",       axis: "MODE", categorical: true }
  ];
  var MATRIX_METRICS = [
    { key: "acked_per_sec",    label: "ack/s",       axis: "ACK / SEC",          fmt: fmtRate },
    { key: "produced_per_sec", label: "insert/s",    axis: "INSERT / SEC",       fmt: fmtRate },
    { key: "latency_p95_ms",   label: "p95",         axis: "P95 LATENCY (MS)",   fmt: fmtMs },
    { key: "producer_avg_ms",  label: "prod avg",    axis: "PRODUCER AVG (MS)",  fmt: fmtMs },
    { key: "select_avg_ms",    label: "recv avg",    axis: "RECEIVE AVG (MS)",   fmt: fmtMs },
    { key: "ack_avg_ms",       label: "ack avg",     axis: "ACK AVG (MS)",       fmt: fmtMs },
    { key: "empty_polls",      label: "empty polls", axis: "EMPTY POLLS",        fmt: fmtInt },
    { key: "receive_calls",    label: "recv calls",  axis: "RECEIVE CALLS",      fmt: fmtInt },
    { key: "elapsed_secs",     label: "elapsed",     axis: "ELAPSED (S)",        fmt: fmtSecs }
  ];
  var DEFAULT_COLS = ["acked_per_sec", "produced_per_sec", "latency_p95_ms"];
  var METRIC_LABEL_MAP = {
    latency: "Job latency",
    producer_query: "Producer insert",
    select_query: "Receive claim",
    ack_query: "Ack complete"
  };

  var matrixState = null;
  var matrixRowsCached = null;
  var matrixVaryingDims = [];
  var matrixHasMultiRepeat = false;

  function getDimMeta(key) {
    for (var i = 0; i < MATRIX_PARAMS.length; i++)  if (MATRIX_PARAMS[i].key === key)  return MATRIX_PARAMS[i];
    for (var j = 0; j < MATRIX_METRICS.length; j++) if (MATRIX_METRICS[j].key === key) return MATRIX_METRICS[j];
    return null;
  }
  function isCategorical(key) { var m = getDimMeta(key); return !!(m && m.categorical); }
  function fmtDimValue(key, v) {
    var m = getDimMeta(key);
    if (m && m.fmt) return m.fmt(v);
    if (v == null) return "—";
    return finite(Number(v)) ? Number(v).toLocaleString() : String(v);
  }
  function uniqueValues(arr) {
    var seen = Object.create(null), out = [];
    for (var i = 0; i < arr.length; i++) {
      var key = String(arr[i]);
      if (!(key in seen)) { seen[key] = true; out.push(arr[i]); }
    }
    return out;
  }
  function computeVaryingDims(rows) {
    var result = [];
    MATRIX_PARAMS.forEach(function(p) {
      var vals = uniqueValues(rows.map(function(r) { return r[p.key]; }));
      if (vals.length > 1) result.push(p.key);
    });
    return result;
  }
  function compactShape(r) {
    var bits = [];
    matrixVaryingDims.forEach(function(k) {
      // When a regime is selected, mode is constant across the visible runs — don't repeat it
      if (k === "mode" && matrixState && matrixState.regimeFilter) return;
      var v = r[k];
      if (k === "consumers") bits.push(v + "w");
      else if (k === "producers") bits.push(v + "p");
      else if (k === "receive_batch") bits.push("recv " + v);
      else if (k === "producer_batch") bits.push("prod " + v);
      else if (k === "buckets") bits.push("buckets " + v);
      else if (k === "job_ms") bits.push("job " + v + "ms");
      else if (k === "tasks") bits.push(fmtInt(v) + " tasks");
      else if (k === "mode") bits.push(String(v));
      else bits.push((getDimMeta(k) || { label: k }).label + " " + v);
    });
    if (matrixHasMultiRepeat) bits.push("rep " + r.repeat);
    return bits.join(" · ") || "single config";
  }
  function formatRunId(r) {
    return matrixHasMultiRepeat ? "#" + r.run_index + "." + r.repeat : "#" + r.run_index;
  }
  function runKey(r) { return r.run_index + "." + r.repeat; }
  function colorForRun(r) {
    if (!matrixState || !matrixState.color) return MATRIX_PALETTE[0];
    var distinct = matrixState.colorDistinct;
    var v = r[matrixState.color];
    for (var i = 0; i < distinct.length; i++) {
      if (String(distinct[i]) === String(v)) return MATRIX_PALETTE[i % MATRIX_PALETTE.length];
    }
    return MATRIX_PALETTE[0];
  }

  function initMatrixState(rows) {
    var varying = matrixVaryingDims;
    var firstVarying = varying.filter(function(k) { return k !== "mode"; })[0] || varying[0] || "consumers";
    var secondVarying = varying.filter(function(k) { return k !== firstVarying; })[0] || null;
    var visible = new Set(DEFAULT_COLS);
    return {
      x: firstVarying,
      y: "acked_per_sec",
      color: secondVarying,
      colorDistinct: [],
      colorFilter: new Set(),
      sortKey: "acked_per_sec",
      sortDir: "desc",
      visibleCols: visible,
      expanded: new Set(),
      selected: new Set(),
      focusedRun: null,
      regimeFilter: null
    };
  }

  function currentRows() {
    var rows = matrixRowsCached;
    if (matrixState && matrixState.regimeFilter) {
      rows = rows.filter(function(r) { return r.mode === matrixState.regimeFilter; });
    }
    var cKey = matrixState && matrixState.color;
    var filter = matrixState && matrixState.colorFilter;
    if (cKey && filter && filter.size) {
      rows = rows.filter(function(r) { return !filter.has(String(r[cKey])); });
    }
    return rows;
  }

  function allRowsForColorDim() {
    var rows = matrixRowsCached;
    if (matrixState && matrixState.regimeFilter) {
      rows = rows.filter(function(r) { return r.mode === matrixState.regimeFilter; });
    }
    return rows;
  }

  // ---- Scatter pivot ----

  function renderScatterSection() {
    var section = document.getElementById("throughput");
    if (!section) return;
    setSection("throughput", "§ 01 — Pivot", "Compare runs across two dimensions",
      "Pick X, Y, and a color dimension. Hover for run details. Click a point to focus the run row below.",
      "FIG. 1 — Run scatter");

    // collapse the two-column wrapper if present, keep one frame
    var colsWrap = section.querySelector(".cols-2");
    if (colsWrap) {
      var firstFrame = colsWrap.querySelector(".frame");
      if (firstFrame) {
        colsWrap.parentNode.replaceChild(firstFrame, colsWrap);
      }
    }
    var frameBody = section.querySelector(".frame .body");
    if (!frameBody) return;
    frameBody.innerHTML =
      '<div class="scatter-controls">' +
        scatterDropdown("x", "X axis") +
        scatterDropdown("y", "Y axis") +
        scatterDropdown("color", "Color") +
      '</div>' +
      '<div class="scatter-svg-wrap"><svg class="scatter-svg" viewBox="0 0 1200 480" preserveAspectRatio="xMidYMid meet"></svg></div>' +
      '<div class="scatter-legend" id="scatter-legend"></div>';

    ["x", "y", "color"].forEach(function(axis) {
      var sel = document.getElementById("scatter-" + axis);
      if (!sel) return;
      sel.innerHTML = scatterOptionsHtml(axis);
      sel.value = matrixState[axis] || "";
      sel.addEventListener("change", function() {
        matrixState[axis] = sel.value || null;
        if (axis === "color") matrixState.colorFilter = new Set();
        drawScatter();
        updateRunTable();
        refreshMatrixKpis();
      });
    });
    drawScatter();
  }

  function scatterOptionsHtml(axis) {
    var varying = matrixVaryingDims;
    if (axis === "color") {
      var opts = '<option value="">(none)</option>';
      varying.forEach(function(k) {
        var m = getDimMeta(k);
        opts += '<option value="' + esc(k) + '">' + esc(m.label.toUpperCase()) + '</option>';
      });
      return opts;
    }
    var paramOpts = varying.filter(function(k) { return !isCategorical(k); }).map(function(k) {
      var m = getDimMeta(k);
      return '<option value="' + esc(k) + '">' + esc(m.label.toUpperCase()) + '</option>';
    }).join("");
    var metricOpts = MATRIX_METRICS.map(function(m) {
      return '<option value="' + esc(m.key) + '">' + esc(m.label.toUpperCase()) + '</option>';
    }).join("");
    return (paramOpts ? '<optgroup label="PARAMS">' + paramOpts + '</optgroup>' : "")
      + '<optgroup label="METRICS">' + metricOpts + '</optgroup>';
  }

  function scatterDropdown(axis, label) {
    return '<label><span>' + esc(label) + '</span><select id="scatter-' + axis + '"></select></label>';
  }

  function drawScatter() {
    var svg = document.querySelector(".scatter-svg");
    if (!svg) return;
    var ns = "http://www.w3.org/2000/svg";
    while (svg.firstChild) svg.removeChild(svg.firstChild);

    var W = 1200, H = 480;
    var padL = 90, padR = 30, padT = 30, padB = 64;
    var iw = W - padL - padR;
    var ih = H - padT - padB;

    var rows = currentRows();
    var xKey = matrixState.x, yKey = matrixState.y, cKey = matrixState.color;

    var pts = rows.map(function(r) {
      return { run: r, xv: r[xKey], yv: r[yKey], cv: cKey ? r[cKey] : null };
    }).filter(function(p) {
      return p.xv != null && p.yv != null && finite(Number(p.yv));
    });

    if (cKey) {
      var universe = allRowsForColorDim().map(function(r) { return r[cKey]; });
      matrixState.colorDistinct = uniqueValues(universe);
    } else {
      matrixState.colorDistinct = [];
    }

    var xNums = pts.map(function(p) { return Number(p.xv); }).filter(finite);
    var xCategorical = isCategorical(xKey) || xNums.length === 0;
    var xMin = xCategorical ? 0 : Math.min.apply(null, xNums);
    var xMax = xCategorical ? 0 : Math.max.apply(null, xNums);
    if (!xCategorical && xMax === xMin) { xMin -= 1; xMax += 1; }
    var xDistinct = xCategorical ? uniqueValues(pts.map(function(p) { return p.xv; })) : [];

    var yNums = pts.map(function(p) { return Number(p.yv); }).filter(finite);
    var yMin = 0;
    var yMax = yNums.length ? Math.max.apply(null, yNums) : 1;
    if (yMax === yMin) yMax = yMin + 1;

    function xScale(v) {
      if (xCategorical) {
        var idx = xDistinct.indexOf(v);
        if (xDistinct.length <= 1) return padL + iw / 2;
        return padL + (idx / (xDistinct.length - 1)) * iw;
      }
      return padL + ((Number(v) - xMin) / (xMax - xMin)) * iw;
    }
    function yScale(v) {
      return padT + ih - ((Number(v) - yMin) / (yMax - yMin)) * ih;
    }
    function el(tag, attrs, parent) {
      var node = document.createElementNS(ns, tag);
      if (attrs) for (var k in attrs) node.setAttribute(k, attrs[k]);
      (parent || svg).appendChild(node);
      return node;
    }

    // y gridlines + labels
    var yTicks = 6;
    for (var ti = 0; ti <= yTicks; ti++) {
      var yv = yMin + ((yMax - yMin) * ti) / yTicks;
      var yPx = yScale(yv);
      el("line", { class: "gridline", x1: padL, x2: padL + iw, y1: yPx, y2: yPx });
      var lbl = el("text", { class: "axis-label", x: padL - 12, y: yPx + 4, "text-anchor": "end" });
      lbl.textContent = fmtDimValue(yKey, yv);
    }

    // x ticks
    if (xCategorical) {
      xDistinct.forEach(function(v) {
        var px = xScale(v);
        var t = el("text", { class: "axis-label", x: px, y: padT + ih + 22, "text-anchor": "middle" });
        t.textContent = String(v);
      });
    } else {
      var xTicks = 5;
      for (var xi = 0; xi <= xTicks; xi++) {
        var xv = xMin + ((xMax - xMin) * xi) / xTicks;
        var px = xScale(xv);
        var t = el("text", { class: "axis-label", x: px, y: padT + ih + 22, "text-anchor": "middle" });
        t.textContent = fmtDimValue(xKey, xv);
      }
    }

    // axes
    el("line", { class: "axis-line", x1: padL, x2: padL + iw, y1: padT + ih, y2: padT + ih });
    el("line", { class: "axis-line", x1: padL, x2: padL, y1: padT, y2: padT + ih });

    // axis titles
    var xT = el("text", { class: "axis-title", x: padL + iw / 2, y: H - 18, "text-anchor": "middle" });
    xT.textContent = (getDimMeta(xKey) || { axis: xKey }).axis;
    var yT = el("text", { class: "axis-title", x: -(padT + ih / 2), y: 22, "text-anchor": "middle", transform: "rotate(-90)" });
    yT.textContent = (getDimMeta(yKey) || { axis: yKey }).axis;

    // points
    pts.forEach(function(p) {
      var color = colorForRun(p.run);
      var cls = "point" + (p.run.valid ? "" : " invalid");
      var rk = runKey(p.run);
      if (matrixState.focusedRun === rk) cls += " focused";
      var radius = matrixState.selected.has(rk) ? 8 : 6;
      var dot = el("circle", {
        class: cls,
        cx: xScale(p.xv).toFixed(2),
        cy: yScale(p.yv).toFixed(2),
        r: radius,
        fill: color
      });
      dot.setAttribute("tabindex", "0");
      dot.setAttribute("role", "button");
      var xMeta = getDimMeta(xKey) || {};
      var yMeta = getDimMeta(yKey) || {};
      dot.setAttribute("aria-label", formatRunId(p.run) + " " + (xMeta.label||xKey) + "=" + fmtDimValue(xKey, p.xv) + ", " + (yMeta.label||yKey) + "=" + fmtDimValue(yKey, p.yv));
      var show = function(e) {
        var cx, cy;
        if (e && e.clientX != null) { cx = e.clientX; cy = e.clientY; }
        else { var r = dot.getBoundingClientRect(); cx = r.left + r.width / 2; cy = r.top; }
        var html = '<strong>' + esc(formatRunId(p.run)) + (p.run.valid ? "" : " · invalid") + '</strong>'
          + '<div class="row"><span>' + esc(xMeta.label || xKey) + '</span><span>' + esc(fmtDimValue(xKey, p.xv)) + '</span></div>'
          + '<div class="row"><span>' + esc(yMeta.label || yKey) + '</span><span>' + esc(fmtDimValue(yKey, p.yv)) + '</span></div>'
          + (cKey ? '<div class="row"><span>' + esc((getDimMeta(cKey)||{}).label || cKey) + '</span><span>' + esc(fmtDimValue(cKey, p.cv)) + '</span></div>' : '')
          + '<div class="row"><span>shape</span><span>' + esc(compactShape(p.run)) + '</span></div>';
        showTip(html, cx, cy);
      };
      dot.addEventListener("mouseenter", show);
      dot.addEventListener("mousemove", show);
      dot.addEventListener("mouseleave", hideTip);
      dot.addEventListener("focus", function() { show(null); });
      dot.addEventListener("blur", hideTip);
      dot.addEventListener("click", function(e) {
        if (e.shiftKey) toggleSelect(rk);
        else focusRun(p.run);
      });
      dot.addEventListener("keydown", function(e) {
        if (e.key === "Enter" || e.key === " ") { e.preventDefault(); focusRun(p.run); }
      });
    });

    // legend
    var legend = document.getElementById("scatter-legend");
    legend.innerHTML = "";
    if (cKey && matrixState.colorDistinct.length > 1) {
      var lbl = document.createElement("span");
      lbl.className = "lbl";
      lbl.textContent = (getDimMeta(cKey) || { label: cKey }).label;
      legend.appendChild(lbl);

      matrixState.colorDistinct.forEach(function(v, i) {
        var btn = document.createElement("button");
        btn.type = "button";
        var key = String(v);
        var muted = matrixState.colorFilter.has(key);
        btn.className = "item" + (muted ? " muted" : "");
        btn.setAttribute("aria-pressed", muted ? "false" : "true");
        btn.innerHTML = '<span class="swatch" style="background:' + MATRIX_PALETTE[i % MATRIX_PALETTE.length] + '"></span>' + esc(fmtDimValue(cKey, v));
        btn.addEventListener("click", function() { toggleColorFilter(key); });
        legend.appendChild(btn);
      });

      var hasFilter = matrixState.colorFilter.size > 0;
      var allBtn = document.createElement("button");
      allBtn.type = "button";
      allBtn.className = "item ghost";
      allBtn.textContent = hasFilter ? "show all" : "isolate…";
      allBtn.title = hasFilter ? "Clear filter" : "Click any swatch to hide that series";
      allBtn.addEventListener("click", function() {
        if (matrixState.colorFilter.size) {
          matrixState.colorFilter = new Set();
        } else {
          matrixState.colorFilter = new Set(matrixState.colorDistinct.slice(1).map(String));
        }
        drawScatter();
        updateRunTable();
        refreshMatrixKpis();
      });
      legend.appendChild(allBtn);

      var count = document.createElement("span");
      count.className = "lbl muted-count";
      count.textContent = fmtInt(pts.length) + " of " + fmtInt(allRowsForColorDim().length) + " runs";
      legend.appendChild(count);
    } else {
      var single = document.createElement("span");
      single.className = "lbl";
      single.textContent = fmtInt(pts.length) + " of " + fmtInt(rows.length) + " runs plotted";
      legend.appendChild(single);
    }
  }

  function toggleColorFilter(key) {
    if (!matrixState || !matrixState.color) return;
    if (matrixState.colorFilter.has(key)) matrixState.colorFilter.delete(key);
    else matrixState.colorFilter.add(key);
    drawScatter();
    updateRunTable();
    refreshMatrixKpis();
  }

  function focusRun(r) {
    matrixState.focusedRun = runKey(r);
    drawScatter();
    var tbody = document.getElementById("run-tbody");
    if (!tbody) return;
    var row = tbody.querySelector('tr.row[data-run="' + runKey(r) + '"]');
    if (row) row.scrollIntoView({ block: "center", behavior: "smooth" });
  }

  // ---- Run table ----

  function renderRunTableSection() {
    var section = document.getElementById("counters");
    if (!section) return;
    setSection("counters", "§ 02 — Runs", "All runs · sort, expand, overlay",
      "Click a row to inline-expand its sparklines. Shift-click rows (or scatter points) to multi-select and overlay.",
      "TAB. 1 — Run table");
    var body = section.querySelector(".frame .body");
    if (!body) return;
    body.innerHTML =
      '<div class="run-table-controls">' +
        '<details class="col-toggle"><summary>columns</summary>' +
          '<div class="col-grid">' + columnToggleHtml() + '</div>' +
        '</details>' +
        '<span class="run-table-count" id="run-table-count"></span>' +
      '</div>' +
      '<div class="run-table-wrap"><table class="run-table" id="run-table"><thead></thead><tbody id="run-tbody"></tbody></table></div>' +
      '<p class="run-table-hint">click row to expand · shift-click for multi-select · click header to sort</p>';

    body.querySelectorAll(".col-toggle input[type=checkbox]").forEach(function(cb) {
      cb.addEventListener("change", function() {
        var key = cb.dataset.col;
        if (cb.checked) matrixState.visibleCols.add(key);
        else matrixState.visibleCols.delete(key);
        updateRunTable();
      });
    });
    updateRunTable();
  }

  function columnToggleHtml() {
    return MATRIX_METRICS.map(function(m) {
      var checked = matrixState.visibleCols.has(m.key) ? " checked" : "";
      return '<label><input type="checkbox" data-col="' + esc(m.key) + '"' + checked + '>' + esc(m.label) + '</label>';
    }).join("");
  }

  function updateRunTable() {
    var table = document.getElementById("run-table");
    if (!table) return;
    var thead = table.querySelector("thead");
    var tbody = document.getElementById("run-tbody");
    var rows = currentRows().slice();

    var sortKey = matrixState.sortKey;
    var sortDir = matrixState.sortDir;
    rows.sort(function(a, b) {
      var av, bv;
      if (sortKey === "id")        { av = a.run_index * 1000 + a.repeat; bv = b.run_index * 1000 + b.repeat; }
      else if (sortKey === "status") { av = a.valid ? 1 : 0; bv = b.valid ? 1 : 0; }
      else if (sortKey === "shape")  { av = compactShape(a); bv = compactShape(b); }
      else                           { av = a[sortKey]; bv = b[sortKey]; }
      if (typeof av === "string" && typeof bv === "string") return sortDir === "asc" ? av.localeCompare(bv) : bv.localeCompare(av);
      av = Number(av) || 0; bv = Number(bv) || 0;
      return sortDir === "asc" ? av - bv : bv - av;
    });

    var fixedCols = [
      { key: "id",     label: "RUN" },
      { key: "status", label: "STATUS" },
      { key: "shape",  label: "SHAPE" }
    ];
    var metricCols = MATRIX_METRICS.filter(function(m) { return matrixState.visibleCols.has(m.key); });
    var allCols = fixedCols.concat(metricCols.map(function(m) {
      return { key: m.key, label: m.label.toUpperCase(), num: true, fmt: m.fmt };
    }));

    thead.innerHTML = "<tr>" + allCols.map(function(c) {
      var ind = matrixState.sortKey === c.key
        ? '<span class="sort-ind">' + (matrixState.sortDir === "desc" ? "↓" : "↑") + '</span>'
        : "";
      var cls = c.num ? ' class="num"' : "";
      return '<th data-key="' + esc(c.key) + '"' + cls + '>' + esc(c.label) + ind + '</th>';
    }).join("") + "</tr>";

    thead.querySelectorAll("th").forEach(function(th) {
      th.addEventListener("click", function() {
        var key = th.dataset.key;
        if (matrixState.sortKey === key) {
          matrixState.sortDir = matrixState.sortDir === "asc" ? "desc" : "asc";
        } else {
          matrixState.sortKey = key;
          matrixState.sortDir = (key === "id" || key === "shape") ? "asc" : "desc";
        }
        updateRunTable();
      });
    });

    tbody.innerHTML = rows.map(function(r) {
      var rk = runKey(r);
      var rowCls = ["row"];
      if (matrixState.selected.has(rk)) rowCls.push("selected");
      if (matrixState.expanded.has(rk)) rowCls.push("expanded");
      if (matrixState.focusedRun === rk) rowCls.push("focused");
      var color = colorForRun(r);
      var idCell = '<td><span class="run-id"><span class="swatch-dot" style="background:' + color + '"></span>' + esc(formatRunId(r)) + '</span></td>';
      var statusCell = '<td><span class="status-badge' + (r.valid ? "" : " invalid") + '">' + (r.valid ? "ok" : "✕ fail") + '</span></td>';
      var shapeCell = '<td><span class="shape-cell">' + esc(compactShape(r)) + '</span></td>';
      var metricCells = metricCols.map(function(m) {
        var v = r[m.key];
        var inner = (m.key === "acked_per_sec") ? '<strong>' + esc(m.fmt(v)) + '</strong>' : esc(m.fmt(v));
        return '<td class="num">' + inner + '</td>';
      }).join("");
      var dataRow = '<tr class="' + rowCls.join(" ") + '" data-run="' + esc(rk) + '">' +
        idCell + statusCell + shapeCell + metricCells + '</tr>';
      var expandRow = matrixState.expanded.has(rk)
        ? '<tr class="expand-row" data-run="' + esc(rk) + '"><td colspan="' + allCols.length + '">' + expansionHtml(r) + '</td></tr>'
        : "";
      return dataRow + expandRow;
    }).join("");

    tbody.querySelectorAll("tr.row").forEach(function(tr) {
      tr.addEventListener("click", function(e) {
        var rk = tr.dataset.run;
        if (e.shiftKey) {
          toggleSelect(rk);
          window.getSelection && window.getSelection().removeAllRanges();
        } else {
          toggleExpand(rk);
        }
      });
    });

    document.querySelectorAll("tr.expand-row .range-row").forEach(function(row) {
      if (typeof setupRangeRow === "function" && !row.dataset.wired) {
        setupRangeRow(row);
        row.dataset.wired = "1";
        var open = function(e) {
          var data = {
            op: row.dataset.op || "",
            samples: (row.dataset.samples || "").split(",").filter(Boolean).map(parseFloat).filter(finite),
            count: parseInt(row.dataset.count, 10) || 0,
            min: parseFloat(row.dataset.min) || 0,
            avg: parseFloat(row.dataset.avg) || 0,
            p50: parseFloat(row.dataset.p50) || 0,
            p95: parseFloat(row.dataset.p95) || 0,
            p99: parseFloat(row.dataset.p99) || 0,
            max: parseFloat(row.dataset.max) || 0
          };
          openChart(data);
          if (e) { e.preventDefault(); e.stopPropagation(); }
        };
        // The per-run-report init that wires sparkline/track click handlers is NOT
        // called in matrix mode, so wire them here. Stop propagation so the click
        // doesn't bubble up to the table row and collapse the expand.
        var spark = row.querySelector(".sparkline svg");
        if (spark) {
          spark.addEventListener("click", open);
          spark.addEventListener("keydown", function(e) { if (e.key === "Enter") open(e); });
        }
        var track = row.querySelector(".range-track");
        if (track) {
          track.addEventListener("click", open);
          track.addEventListener("keydown", function(e) { if (e.key === "Enter") open(e); });
        }
        var btn = row.querySelector("[data-expand]");
        if (btn) btn.addEventListener("click", open);
      }
    });

    var count = document.getElementById("run-table-count");
    if (count) count.innerHTML = '<em>' + fmtInt(rows.length) + '</em> runs · sort: <em>' +
      esc(matrixState.sortKey) + " " + (matrixState.sortDir === "desc" ? "↓" : "↑") + '</em>';

    updateOverlayBar();
  }

  function expansionHtml(r) {
    var rep = r.report || {};
    var timings = rep.timings || {};
    var latency = rep.latency || {};
    var parts = [];
    [["producer_query","Producer insert"], ["select_query","Receive claim"], ["ack_query","Ack complete"]].forEach(function(p) {
      if (timings[p[0]]) parts.push(rangeRow(p[1], timings[p[0]]));
    });
    if (latency && latency.count) parts.push(rangeRow("Job latency", latency, fmtInt(latency.count) + " acked"));
    if (!parts.length) return '<div style="font-family:var(--serif);color:var(--ink-faint)">No per-op samples captured for this run.</div>';
    return '<div class="expand-grid">' + parts.join("") + '</div>';
  }

  function toggleExpand(rk) {
    if (matrixState.expanded.has(rk)) matrixState.expanded.delete(rk);
    else matrixState.expanded.add(rk);
    updateRunTable();
  }
  function toggleSelect(rk) {
    if (matrixState.selected.has(rk)) matrixState.selected.delete(rk);
    else matrixState.selected.add(rk);
    drawScatter();
    updateRunTable();
  }

  // ---- Overlay action bar ----

  function setupOverlayBar() {
    if (document.querySelector(".overlay-bar")) return;
    var bar = document.createElement("div");
    bar.className = "overlay-bar";
    bar.innerHTML =
      '<span class="count" id="overlay-count">0 runs</span>' +
      '<label style="display:inline-flex;gap:6px;align-items:center;color:var(--paper);font-size:11px;letter-spacing:0.04em;">METRIC ' +
        '<select id="overlay-metric">' +
          '<option value="latency">Job latency</option>' +
          '<option value="producer_query">Producer insert</option>' +
          '<option value="select_query">Receive claim</option>' +
          '<option value="ack_query">Ack complete</option>' +
        '</select>' +
      '</label>' +
      '<button type="button" class="primary" id="overlay-open">⤢ overlay</button>' +
      '<button type="button" class="ghost" id="overlay-clear">clear</button>';
    document.body.appendChild(bar);
    document.getElementById("overlay-clear").addEventListener("click", function() {
      matrixState.selected.clear();
      drawScatter();
      updateRunTable();
    });
    document.getElementById("overlay-open").addEventListener("click", openOverlay);
    updateOverlayBar();
  }

  function updateOverlayBar() {
    var bar = document.querySelector(".overlay-bar");
    if (!bar) return;
    var n = matrixState.selected.size;
    var countEl = document.getElementById("overlay-count");
    if (countEl) countEl.textContent = n + " run" + (n === 1 ? "" : "s") + " selected";
    if (n >= 2) bar.classList.add("show");
    else bar.classList.remove("show");
  }

  function openOverlay() {
    var selected = Array.from(matrixState.selected);
    if (selected.length < 2) return;
    var metricKey = (document.getElementById("overlay-metric") || {}).value || "latency";
    var lines = selected.map(function(rk) {
      var r = matrixRowsCached.find(function(x) { return runKey(x) === rk; });
      if (!r) return null;
      var rep = r.report || {};
      var timing = metricKey === "latency" ? (rep.latency || {}) : ((rep.timings || {})[metricKey] || {});
      return {
        run: r, rk: rk,
        samples: Array.isArray(timing.samples_ms) ? timing.samples_ms : [],
        p50: Number(timing.p50_ms) || 0,
        p95: Number(timing.p95_ms) || 0,
        p99: Number(timing.p99_ms) || 0,
        avg: Number(timing.avg_ms) || Number(timing.mean_ms) || 0,
        min: Number(timing.min_ms) || 0,
        max: Number(timing.max_ms) || 0,
        count: Number(timing.count) || 0,
        color: colorForRun(r)
      };
    }).filter(Boolean);
    if (!lines.length) return;

    var titleEl = document.getElementById("chartTitle");
    var subEl = document.getElementById("chartSubtitle");
    var stage = document.getElementById("chartStage");
    var modal = document.getElementById("chartModal");
    if (titleEl) titleEl.textContent = METRIC_LABEL_MAP[metricKey] || metricKey;
    if (subEl) subEl.textContent = lines.length + " runs overlaid · " + selected.join(", ");
    stage.innerHTML = "";
    stage.appendChild(buildOverlayChart(lines));
    modal.classList.add("open");
    hideTip();
  }

  function buildOverlayChart(lines) {
    var ns = "http://www.w3.org/2000/svg";
    var wrap = document.createElement("div");
    wrap.className = "big-chart";

    var W = 1400, H = 520;
    var padL = 90, padR = 40, padT = 40, padB = 70;
    var iw = W - padL - padR;
    var ih = H - padT - padB;

    var yMin = 0, yMax = 0;
    lines.forEach(function(l) {
      if (l.max > yMax) yMax = l.max;
      l.samples.forEach(function(s) { if (s > yMax) yMax = s; });
    });
    if (yMax === 0) yMax = 1;

    function xFor(i, total) { return padL + (total <= 1 ? 0 : (i / (total - 1)) * iw); }
    function yFor(v) { return padT + ih - ((v - yMin) / (yMax - yMin)) * ih; }

    var svg = document.createElementNS(ns, "svg");
    svg.setAttribute("class", "canvas");
    svg.setAttribute("viewBox", "0 0 " + W + " " + H);
    svg.setAttribute("preserveAspectRatio", "xMidYMid meet");

    function el(tag, attrs, parent) {
      var node = document.createElementNS(ns, tag);
      if (attrs) for (var k in attrs) node.setAttribute(k, attrs[k]);
      (parent || svg).appendChild(node);
      return node;
    }

    var yTicks = 6;
    for (var ti = 0; ti <= yTicks; ti++) {
      var yv = yMin + ((yMax - yMin) * ti) / yTicks;
      var yPx = yFor(yv);
      el("line", { class: "gridline", x1: padL, x2: padL + iw, y1: yPx, y2: yPx });
      var t = el("text", { class: "axis-label", x: padL - 12, y: yPx + 4, "text-anchor": "end" });
      t.textContent = fmtMs(yv);
    }
    el("line", { class: "axis-line", x1: padL, x2: padL + iw, y1: padT + ih, y2: padT + ih });
    el("line", { class: "axis-line", x1: padL, x2: padL, y1: padT, y2: padT + ih });
    var xT = el("text", { class: "axis-title", x: padL + iw / 2, y: H - 22, "text-anchor": "middle" });
    xT.textContent = "SAMPLE INDEX (normalized)";
    var yT = el("text", { class: "axis-title", x: -(padT + ih / 2), y: 22, "text-anchor": "middle", transform: "rotate(-90)" });
    yT.textContent = "LATENCY (MS)";

    lines.forEach(function(l) {
      if (l.samples.length < 2) return;
      var pts = l.samples.map(function(v, i) { return xFor(i, l.samples.length) + "," + yFor(v); }).join(" ");
      var poly = el("polyline", { class: "data-line overlay", points: pts });
      poly.style.stroke = l.color;
    });

    wrap.appendChild(svg);

    var legend = document.createElement("div");
    legend.className = "overlay-legend";
    legend.innerHTML = lines.map(function(l) {
      return '<div class="item"><span class="swatch" style="background:' + l.color + '"></span>' +
        '<strong>' + esc(formatRunId(l.run)) + '</strong> ' + esc(compactShape(l.run)) +
        ' · p95 ' + esc(fmtMs(l.p95)) + ' · avg ' + esc(fmtMs(l.avg)) +
        '</div>';
    }).join("");
    wrap.appendChild(legend);
    return wrap;
  }

  // ---- Header, config, plans ----

  function refreshMatrixKpis() {
    var rows = currentRows();
    var validRows = rows.filter(function(r) { return r.valid; });
    var invalidRows = rows.filter(function(r) { return !r.valid; });
    var bestAck = validRows.slice().sort(function(a, b) { return b.acked_per_sec - a.acked_per_sec; })[0]
      || rows.slice().sort(function(a, b) { return b.acked_per_sec - a.acked_per_sec; })[0] || null;
    var bestInsert = validRows.slice().sort(function(a, b) { return b.produced_per_sec - a.produced_per_sec; })[0] || rows[0] || null;
    var medAck = median(validRows.map(function(r) { return r.acked_per_sec; }));
    var regimeNote = matrixState && matrixState.regimeFilter ? " · " + matrixState.regimeFilter : "";
    $("kpis").innerHTML = [
      ["Runs" + regimeNote, fmtInt(rows.length), invalidRows.length ? invalidRows.length + " invalid" : "all valid"],
      ["Best ack", bestAck ? fmtRate(bestAck.acked_per_sec) : "0/s", bestAck ? formatRunId(bestAck) + " · " + compactShape(bestAck) : "none"],
      ["Best insert", bestInsert ? fmtRate(bestInsert.produced_per_sec) : "0/s", bestInsert ? formatRunId(bestInsert) + " · " + compactShape(bestInsert) : "none"],
      ["Median ack", fmtRate(medAck), validRows.length + " valid runs"]
    ].map(function(k) {
      return '<div class="kpi"><span class="label">' + esc(k[0]) + '</span><div class="value">' + esc(k[1]) + '</div><div class="hint">' + esc(k[2]) + '</div></div>';
    }).join("");

    // Also keep subtitle in sync with the current view
    var subtitleEl = $("subtitle");
    if (subtitleEl) {
      var v = matrixVaryingDims.filter(function(k) { return !(k === "mode" && matrixState && matrixState.regimeFilter); });
      var base = fmtInt(rows.length) + " benchmark cell" + (rows.length === 1 ? "" : "s") + " across " + (v.length || 1) + " varying dimension" + (v.length === 1 ? "" : "s");
      subtitleEl.textContent = matrixState && matrixState.regimeFilter
        ? base + " · regime: " + matrixState.regimeFilter
        : base + ".";
    }
  }

  function setupRegimeBar() {
    var existing = document.querySelector(".regime-bar");
    if (existing) existing.remove();
    var modes = uniqueValues(matrixRowsCached.map(function(r) { return r.mode; }));
    if (modes.length <= 1) return;

    var bar = document.createElement("div");
    bar.className = "regime-bar";
    var buttonHtml = [
      '<button type="button" data-regime="" class="' + (matrixState.regimeFilter ? "" : "active") + '">' +
        'all <span class="count">' + matrixRowsCached.length + '</span></button>'
    ];
    modes.forEach(function(m) {
      var count = matrixRowsCached.filter(function(r) { return r.mode === m; }).length;
      var active = matrixState.regimeFilter === m ? "active" : "";
      buttonHtml.push(
        '<button type="button" data-regime="' + esc(m) + '" class="' + active + '">' +
          esc(m).replace(/_/g, "-") + ' <span class="count">' + count + '</span></button>'
      );
    });
    bar.innerHTML = '<span class="label">regime</span>' + buttonHtml.join("");

    var cover = document.querySelector("header.cover");
    var topBanner = document.querySelector(".top-banner");
    var anchor = topBanner || cover;
    if (anchor && anchor.parentNode) anchor.parentNode.insertBefore(bar, anchor.nextSibling);

    bar.querySelectorAll("button").forEach(function(btn) {
      btn.addEventListener("click", function() {
        matrixState.regimeFilter = btn.dataset.regime || null;
        bar.querySelectorAll("button").forEach(function(b) { b.classList.toggle("active", b === btn); });
        // selections may now point to filtered-out runs — clear them to avoid stale overlay
        matrixState.selected.clear();
        matrixState.expanded.clear();
        matrixState.focusedRun = null;
        refreshMatrixKpis();
        drawScatter();
        updateRunTable();
      });
    });
  }

  function setupMatrixHeader(matrix, cfg, rows, validRows, invalidRows, varying) {
    var generated = new Date((matrix.generated_at_unix_secs || 0) * 1000);
    var datePart = generated.toISOString().slice(0, 10).replace(/-/g, ".");
    document.title = "queue matrix report";
    $("rev").textContent = "FIG. MATRIX / REV " + datePart + " / " + fmtInt(rows.length) + " RUNS";

    var metaParts = [];
    varying.forEach(function(k) {
      if (k === "mode") return;
      var vals = uniqueValues(rows.map(function(r) { return r[k]; }));
      var m = getDimMeta(k);
      metaParts.push(m.label.toUpperCase() + " " + vals.join(","));
    });
    var modeVals = uniqueValues(rows.map(function(r) { return r.mode; }));
    if (modeVals.length) metaParts.push("MODE " + modeVals.join(","));
    if (cfg.queue_model) metaParts.push(cfg.queue_model);
    $("meta-strip").innerHTML = metaParts.filter(Boolean).map(function(v) { return '<span>' + esc(v) + '</span>'; }).join("");

    $("title").innerHTML = esc(cfg.queue || "queue") + '<em>·</em>matrix';
    $("subtitle").textContent = fmtInt(rows.length) + " benchmark cells across " + (varying.length || 1) + " varying dimension" + (varying.length === 1 ? "" : "s") + ".";

    var pills = [];
    varying.forEach(function(k) {
      var vals = uniqueValues(rows.map(function(r) { return r[k]; }));
      var m = getDimMeta(k);
      pills.push(m.label + " " + vals.join(","));
    });
    if ((cfg.repeats || 1) > 1) pills.push("repeats " + cfg.repeats);
    pills.push(generated.toLocaleString());
    $("run-meta").innerHTML = pills.map(function(v) { return '<span class="pill">' + esc(v) + '</span>'; }).join("");

    refreshMatrixKpis();

    var existing = document.querySelector(".top-banner");
    if (existing) existing.remove();
    if (invalidRows.length) {
      var banner = document.createElement("div");
      banner.className = "top-banner";
      var ids = invalidRows.slice(0, 10).map(function(r) { return formatRunId(r); });
      var more = invalidRows.length > 10 ? " + " + (invalidRows.length - 10) + " more" : "";
      banner.innerHTML = '<strong>Invalid cells</strong> <span>' + invalidRows.length + ' of ' + rows.length + ' runs failed correctness — ' + esc(ids.join(", ")) + esc(more) + '. Filter or exclude before reading throughput.</span>';
      var cover = document.querySelector("header.cover");
      if (cover && cover.parentNode) cover.parentNode.insertBefore(banner, cover.nextSibling);
    }
  }

  function buildMatrixConfigSection(cfg) {
    setSection("config", "§ 03 — Sweep config", "Inputs that define the matrix", "", "TAB. 2 — Sweep configuration");
    var holder = document.getElementById("config-table");
    if (!holder) return;
    var pairs = [
      ["total_runs", cfg.total_runs],
      ["repeats", cfg.repeats],
      ["tasks", (cfg.matrix_tasks || []).join(", ")],
      ["producers", (cfg.matrix_producers || []).join(", ")],
      ["workers", (cfg.matrix_consumers || []).join(", ")],
      ["buckets", (cfg.matrix_buckets || []).join(", ")],
      ["producer batches", (cfg.matrix_producer_batches || []).join(", ")],
      ["receive batches", (cfg.matrix_receive_batches || []).join(", ")],
      ["job ms", (cfg.matrix_job_ms || []).join(", ")],
      ["modes", (cfg.matrix_modes || []).join(", ")],
      ["payload_bytes", cfg.payload_bytes],
      ["lease_secs", cfg.lease_secs],
      ["queue_model", cfg.queue_model],
      ["endpoint", cfg.endpoint],
      ["namespace", cfg.namespace],
      ["database", cfg.database]
    ];
    holder.innerHTML = renderTable(["Input", "Value"], pairs.map(function(p) {
      return ['<code>' + esc(p[0]) + '</code>', esc(p[1] == null ? "" : p[1])];
    }));
  }

  function buildMatrixExplainSection(rows) {
    setSection("explain", "§ 04 — Query plans", "EXPLAIN ANALYZE captures",
      "Hot read paths should show RecordIdScan — never table scans or secondary-index seeks.", null);
    var planBlocks = rows.filter(function(r) {
      var ex = r.report.explain || {};
      return ex.receive_ready_range || ex.recover_lease_range || (ex.errors || []).length;
    }).slice(0, 12).map(function(r, idx) {
      var ex = r.report.explain || {};
      var text = [
        ex.receive_ready_range ? "receive_ready_range\n" + ex.receive_ready_range : "",
        ex.recover_lease_range ? "recover_lease_range\n" + ex.recover_lease_range : "",
        (ex.errors || []).length ? "errors\n" + ex.errors.join("\n") : ""
      ].filter(Boolean).join("\n\n");
      return '<details ' + (idx === 0 ? "open" : "") + '><summary>' + esc(formatRunId(r)) + ' · ' + esc(compactShape(r)) + '</summary><pre>' + esc(text) + '</pre></details>';
    }).join("");
    var holder = document.getElementById("explain-blocks");
    if (holder) holder.innerHTML = planBlocks;
  }

  function renderMatrixReport() {
    var matrix = report;
    var cfg = matrix.config || {};
    var rows = matrixRuns(matrix);
    var validRows = rows.filter(function(r) { return r.valid; });
    var invalidRows = rows.filter(function(r) { return !r.valid; });
    matrixHasMultiRepeat = (cfg.repeats || 1) > 1 || rows.some(function(r) { return r.repeat > 1; });
    matrixVaryingDims = computeVaryingDims(rows);
    matrixRowsCached = rows;
    matrixState = initMatrixState(rows);

    setupMatrixHeader(matrix, cfg, rows, validRows, invalidRows, matrixVaryingDims);
    setupRegimeBar();

    // hide unused sections
    ["timings", "latency", "health"].forEach(function(id) {
      var s = document.getElementById(id);
      if (s) s.style.display = "none";
    });
    // hide TOC links to hidden sections
    document.querySelectorAll('nav.toc a').forEach(function(a) {
      var href = a.getAttribute("href") || "";
      var id = href.replace("#", "");
      if (id === "timings" || id === "latency" || id === "health") a.style.display = "none";
    });

    renderScatterSection();
    renderRunTableSection();
    buildMatrixConfigSection(cfg);

    var hasPlans = rows.some(function(r) {
      var ex = r.report.explain || {};
      return ex.receive_ready_range || ex.recover_lease_range;
    });
    var explainSection = document.getElementById("explain");
    if (hasPlans) {
      buildMatrixExplainSection(rows);
    } else if (explainSection) {
      explainSection.style.display = "none";
      document.querySelectorAll('nav.toc a[href="#explain"]').forEach(function(a) { a.style.display = "none"; });
    }

    // relabel TOC indices that remain
    var visibleLinks = Array.prototype.slice.call(document.querySelectorAll("nav.toc a"))
      .filter(function(a) { return a.style.display !== "none"; });
    var matrixTocLabels = ["Pivot", "Runs", "Config", "Plans"];
    visibleLinks.forEach(function(a, i) {
      var idx = ("0" + (i + 1)).slice(-2);
      var lbl = matrixTocLabels[i] || a.textContent.trim();
      a.innerHTML = '<span class="idx">' + idx + '</span>' + esc(lbl);
    });

    setupOverlayBar();

    $("footer-r").textContent = "surreal-queue-bench " + (matrix.version || "") + " · matrix export";
    initToc();
  }

  if (Array.isArray(report.runs)) {
    renderMatrixReport();
    return;
  }

  // header
  var generated = new Date((report.generated_at_unix_secs || 0) * 1000);
  var datePart = generated.toISOString().slice(0, 10).replace(/-/g, ".");
  document.title = (cfg.queue || "queue") + " " + (cfg.mode || "run") + " report";
  $("rev").textContent = "FIG. RUN / REV " + datePart + " / MODE " + String(cfg.mode || "").toUpperCase();
  $("meta-strip").innerHTML = [
    (cfg.producers || 0) + " PROD",
    (cfg.consumers || 0) + " CONS",
    (cfg.buckets || 0) + " BUCKETS",
    cfg.queue_model || ""
  ].filter(Boolean).map(function(v) { return '<span>' + esc(v) + '</span>'; }).join("");
  $("title").innerHTML = esc(cfg.queue || "queue") + '<em>·</em>' + esc(cfg.mode || "run");
  $("subtitle").textContent = fmtInt(sum.produced_total) + " produced · " + fmtInt(sum.acked_total) + " acked · " + fmtSecs(sum.elapsed_secs) + " elapsed.";
  $("run-meta").innerHTML = [
    "producer batch " + (cfg.producer_batch_size || 0),
    "receive batch " + (cfg.receive_batch_size || 0),
    "lease " + (cfg.lease_secs || 0) + "s",
    "payload " + (cfg.payload_bytes || 0) + "B",
    "job " + (cfg.job_ms || 0) + "ms",
    cfg.fixed_workdown ? "FIXED WORK-DOWN" : "STEADY-STATE",
    generated.toLocaleString()
  ].map(function(v) { return '<span class="pill">' + esc(v) + '</span>'; }).join("");

  $("kpis").innerHTML = [
    ["Produced",   fmtInt(sum.produced_total),  "jobs inserted"],
    ["Acked",      fmtInt(sum.acked_total),     "jobs completed"],
    ["Drain rate", fmtRate(sum.acked_per_sec),  "end-to-end throughput"],
    ["Backlog",    fmtInt(sum.final_backlog),   sum.drained ? "drained cleanly" : "residual backlog"]
  ].map(function(k) {
    return '<div class="kpi"><span class="label">' + esc(k[0]) + '</span><div class="value">' + esc(k[1]) + '</div><div class="hint">' + esc(k[2]) + '</div></div>';
  }).join("");

  var maxRate = Math.max(sum.produced_per_sec || 0, sum.acked_per_sec || 0, 1);
  $("rate-bars").innerHTML = [
    bar("Produced/sec", sum.produced_per_sec || 0, maxRate, false, "/s"),
    bar("Acked/sec",    sum.acked_per_sec || 0,    maxRate, true,  "/s")
  ].join("");

  var maxPhase = Math.max(sum.producer_window_secs || 0, sum.elapsed_secs || 0, 0.001);
  $("phase-bars").innerHTML = [
    bar("Producer window", sum.producer_window_secs || 0, maxPhase, false, "s"),
    bar("Total elapsed",   sum.elapsed_secs || 0,         maxPhase, true,  "s")
  ].join("");

  $("timing-ranges").innerHTML = Object.keys(opLabels)
    .filter(function(k) { return timings[k]; })
    .map(function(k) { return rangeRow(opLabels[k], timings[k]); })
    .join("");

  $("latency-range").innerHTML = rangeRow("Job latency", latency, fmtInt(latency.count || 0) + " acked jobs");

  renderIssues();
  renderCounters();
  $("config-table").innerHTML = renderTable(["Input", "Value"], configRows().map(function(p) {
    return ['<code>' + esc(p[0]) + '</code>', esc(p[1] == null ? "" : p[1])];
  }));
  renderExplain();
  $("footer-r").textContent = "surreal-queue-bench " + (report.version || "") + " · RecordIdScan or it didn't happen";
  initInteractivity();
  initToc();
})();
</script>
</body>
</html>
"##;
