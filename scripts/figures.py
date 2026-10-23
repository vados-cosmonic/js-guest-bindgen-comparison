#!/usr/bin/env python3
"""Render per-engine SVG comparisons and the README results block from saved data."""
from html import escape
from io import StringIO
from pathlib import Path
import json
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Patch
from matplotlib.ticker import FuncFormatter, LogLocator, NullFormatter
from svg_style import BACKGROUND, GRID, MUTED, PALETTE, TEXT, css

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/diagrams"
SIZES = json.loads((ROOT / "results/sizes.json").read_text())["components"]
TIMES = json.loads((ROOT / "results/timings.json").read_text())
COLORS = [PALETTE[k][1] for k in ["bindings", "bindings", "adapter", "runtime"]]
LABELS = ["A", "A*", "B", "C"]
LEGEND = ["A · js-guest-bindgen (JS ABI)", "A* · tuned js-guest-bindgen (JS ABI)", "B · new componentize-js + native shims", "C · new componentize-js → wit-dylib"]
CASES = [("Scalar", "scalar"), ("String · 1 KiB", "text-1024"), ("Bytes · 1 KiB", "bytes-1024"), ("100 host imports", "imports-100")]
CONTROLS = {"boa": "no generated debug logging", "porffor": "no generated debug logging + native UTF-8"}
matplotlib.rcParams.update({
    "font.family": "DejaVu Sans", "font.size": 12, "svg.fonttype": "none",
    "svg.hashsalt": "js-guest-bindgen-comparison", "text.color": TEXT,
    "axes.labelcolor": TEXT, "axes.edgecolor": GRID, "xtick.color": MUTED,
    "ytick.color": TEXT, "axes.facecolor": BACKGROUND, "figure.facecolor": BACKGROUND,
    "grid.color": GRID, "grid.linewidth": 0.8,
})


def cells(engine):
    control = "boa-a-nolog" if engine == "boa" else "porffor-a-native-utf8"
    return [f"{engine}-a", control, f"{engine}-b", f"{engine}-c"]


def save(fig, name, title, description):
    buffer = StringIO()
    fig.savefig(buffer, format="svg", metadata={"Date": None, "Title": title, "Description": description})
    svg = buffer.getvalue()
    start = svg.index("<svg ")
    end = svg.index(">", start)
    svg = svg[:end] + ' role="img" aria-labelledby="title desc"' + svg[end:]
    end = svg.index(">", start)
    svg = svg[:end+1] + f'\n<title id="title">{escape(title)}</title>\n<desc id="desc">{escape(description)}</desc>\n<style>{css()}</style>\n' + svg[end+1:]
    # Matplotlib leaves spaces before line breaks in path attributes.
    (OUT / name).write_text("\n".join(line.rstrip() for line in svg.splitlines()) + "\n")
    plt.close(fig)


def base_axis(ax):
    for side in ("top", "right", "left"):
        ax.spines[side].set_visible(False)
    ax.set_yticks(range(4), LABELS, fontweight="bold")
    ax.set_ylim(3.6, -0.6)
    ax.tick_params(axis="y", length=0, pad=10)
    ax.set_axisbelow(True)
    ax.grid(axis="x")


def legend(fig, dots=False):
    if dots:
        handles = [Line2D([], [], color=c, marker="D" if i==1 else "o", markerfacecolor=BACKGROUND if i==1 else c, linewidth=2, markersize=7) for i,c in enumerate(COLORS)]
    else:
        handles = [Patch(facecolor=BACKGROUND if i==1 else c, edgecolor=c, hatch="///" if i==1 else None) for i,c in enumerate(COLORS)]
    fig.legend(handles, LEGEND, loc="lower left", bbox_to_anchor=(0.07, 0.055), ncol=2, frameon=False, fontsize=11, title="Color legend · approach / binding implementation", title_fontsize=11)


def size_chart(engine):
    fig, ax = plt.subplots(figsize=(12.8, 4.9))
    fig.subplots_adjust(left=0.11, right=0.94, top=0.75, bottom=0.32)
    title = f"{engine.title()} · stripped component size"
    fig.text(0.065, 0.91, title, fontsize=21, weight="bold")
    fig.text(0.065, 0.84, "Lower is better · all custom sections removed · decimal MB (1,000,000 bytes)", color=MUTED, fontsize=12)
    values = [SIZES[c]["stripped_bytes"] / 1e6 for c in cells(engine)]
    for i, (value, color) in enumerate(zip(values, COLORS)):
        ax.barh(i, value, height=0.6, color=BACKGROUND if i==1 else color, edgecolor=color, hatch="///" if i==1 else None)
        ax.text(value+max(values)*0.015, i, f"{value:.3f} MB", va="center", fontsize=12)
    base_axis(ax)
    ax.set_xlim(0, max(values)*1.21)
    ax.set_xlabel("Stripped component size (MB)")
    legend(fig)
    fig.text(0.075, 0.025, f"A*: {CONTROLS[engine]}. C includes shared libc / PIC packaging.", fontsize=11, color=MUTED)
    save(fig, f"{engine}-sizes.svg", title, "A, tuned A, B and C within one engine. " + "; ".join(f"{a}: {v:.3f} MB" for a,v in zip(LABELS, values)) + ". " + CONTROLS[engine])


def latency_chart(engine):
    fig, axes = plt.subplots(2, 2, figsize=(12.8, 7.5))
    fig.subplots_adjust(left=0.09, right=0.85, top=0.78, bottom=0.28, hspace=0.85, wspace=0.70)
    title = f"{engine.title()} · complete component-call latency"
    fig.text(0.065, 0.94, title, fontsize=21, weight="bold")
    fig.text(0.065, 0.89, "Lower is better · logarithmic axes · values are median µs across three processes", color=MUTED, fontsize=12)
    fig.text(0.065, 0.85, "Whiskers: min–max process medians (not confidence intervals); lifting and post-return included", color=MUTED, fontsize=11)
    for ax, (label, case) in zip(axes.flat, CASES):
        measurements = [TIMES[c]["measurements"][case] for c in cells(engine)]
        for i, (m, color) in enumerate(zip(measurements, COLORS)):
            value, lo, hi = [m[k]/1000 for k in ("median_ns", "min_process_median_ns", "max_process_median_ns")]
            ax.errorbar(value, i, xerr=[[value-lo],[hi-value]], fmt="D" if i==1 else "o", color=color, markerfacecolor=BACKGROUND if i==1 else color, markersize=7, capsize=4, elinewidth=2)
            ax.text(1.04, i, f"{value:,.3f}", transform=ax.get_yaxis_transform(), va="center", fontsize=11)
        base_axis(ax)
        ax.set_xscale("log")
        lo = min(m["min_process_median_ns"] for m in measurements)/1000
        hi = max(m["max_process_median_ns"] for m in measurements)/1000
        ax.set_xlim(lo/1.5, hi*1.6)
        ax.xaxis.set_major_locator(LogLocator(base=10, subs=(1, 2, 5), numticks=8))
        ax.xaxis.set_major_formatter(FuncFormatter(lambda x, pos: f"{x:g}"))
        ax.xaxis.set_minor_formatter(NullFormatter())
        ax.tick_params(axis="x", labelsize=10)
        ax.set_title(label, loc="left", fontsize=14, weight="bold", pad=9)
        ax.set_xlabel("µs / call · log scale", fontsize=11)
    legend(fig, dots=True)
    fig.text(0.075, 0.025, f"A*: {CONTROLS[engine]}. Generated JS still owns canonical layout in A*.", fontsize=11, color=MUTED)
    save(fig, f"{engine}-latency.svg", title, "Four logarithmic dot plots compare A, tuned A, B and C within one engine. Dots are median microseconds; whiskers span three process medians. A*: " + CONTROLS[engine])


def readme_results():
    lines = ["## Results at a glance", "", "**JS-side lifting/lowering is not an automatic size or speed win.** The tables below compare A/B/C within each engine. B means **new componentize-js machinery + native engine shims**; C means **new componentize-js targeting wit-dylib**.", "", "Sizes are stripped decimal MB; timings are µs per complete component call, including result lifting and post-return. Each timing is the median of three process medians. A* is a diagnostic A build that keeps canonical layout conversion in JS."]
    for engine in ("boa", "porffor"):
        lines += ["", f"### {engine.title()}", "", f"A*: {CONTROLS[engine]}.", "", "| Approach | Stripped MB | Scalar µs | String 1 KiB µs | Bytes 1 KiB µs | 100 imports µs |", "|---|---:|---:|---:|---:|---:|"]
        for label, cell in zip(LABELS, cells(engine)):
            values = [f"{TIMES[cell]['measurements'][case]['median_ns']/1000:,.3f}" for _,case in CASES]
            lines += [f"| {label} | {SIZES[cell]['stripped_bytes']/1e6:.3f} | " + " | ".join(values) + " |"]
        if engine == "boa":
            lines += ["", "Native B/C are substantially faster on these calls, including against A*. A is only **1.4% smaller than B**; C is **28.8% larger than B**."]
        else:
            lines += ["", "A* matches B's scalar range and is faster on the import loop. B is faster on byte lists and **43.5% smaller** than A* (**53.6% smaller than default A**). C is **163.7% larger than B**, but is faster in the measured 1 KiB string case."]
        lines += ["", f"![{engine.title()} component sizes, comparing A, A*, B and C](docs/diagrams/{engine}-sizes.svg)", "", f"![{engine.title()} latency, comparing A, A*, B and C with process ranges](docs/diagrams/{engine}-latency.svg)"]
    lines += ["", "C's deployment-size gap includes shared libc, PIC and linking scaffolding; it is not the cost of generic conversion alone. Latency graphs use **logarithmic axes**, with min–max process medians as whiskers, not confidence intervals. Small differences and overlapping ranges on this shared host are inconclusive. These results cover the declared synchronous P2 world, not general Promise/I/O workloads.", "", "See [SUMMARY.md](SUMMARY.md) for all controls, cold behavior and qualifications, and [results/TABLES.md](results/TABLES.md) for all measured cases. SVG charts use embedded light/dark styles and include an approach legend. Regenerate the charts and this section with `python3 scripts/figures.py` after installing `scripts/requirements-figures.txt`.", ""]
    path = ROOT / "README.md"
    text = path.read_text()
    start, end = "<!-- BEGIN GENERATED RESULTS -->", "<!-- END GENERATED RESULTS -->"
    block = start + "\n" + "\n".join(lines) + end + "\n\n"
    if start in text:
        before, rest = text.split(start, 1)
        _, after = rest.split(end, 1)
        text = before + block.rstrip() + after
    else:
        text = text.replace("## Call paths and jobs", block + "## Call paths and jobs", 1)
    path.write_text(text)


if __name__ == "__main__":
    for engine in ("boa", "porffor"):
        size_chart(engine)
        latency_chart(engine)
    readme_results()
    print("Wrote four per-engine SVG charts and the README results section")
