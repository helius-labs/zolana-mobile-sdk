#!/usr/bin/env python3
"""The proof timing report from the `ZOLANA_TIMING {json}` lines of the
`fresh` and `restart` runs of integration_test/proof_timing_test.dart.

    proof-timing-report.py --logs DIR [--baseline report.json] [--before TEXT]
                           [--after TEXT] [--out report.json] [--summary FILE]
                           [--title TEXT]

Rows are the four scenarios of the client's table; columns are the shapes,
transfers (`8x2`) and then merges (`merge-24x1`); a cell is the total, with
the download in brackets when there was one, or `failed`. A second table
splits the stages. With a baseline, each row comes twice, `before` (the
baseline) and `after` (these logs) with the change, and a change past
--flag percent is marked; --before and --after name the two commits.
"""

import argparse
import json
import re
import statistics
import sys
from pathlib import Path

LINE = re.compile(r"ZOLANA_TIMING (\{.*\})")
ROWS = [
    ("first run, including download", "fresh", "first"),
    ("warm", "fresh", "rest"),
    ("first run after restart", "restart", "first"),
    ("repeated after restart", "restart", "rest"),
]
STAGES = ["load_ms", "witness_ms", "prove_ms", "verify_ms"]
# Hosted runners vary by about 20 % between runs of one commit.
FLAG = 25.0


def records(logs):
    found = []
    for path in sorted(Path(logs).glob("*.log")):
        for line in path.read_text(errors="replace").splitlines():
            match = LINE.search(line)
            if match:
                found.append(json.loads(match.group(1)))
    return found


def shape_key(shape):
    merge = shape.startswith("merge-")
    inputs, outputs = shape.removeprefix("merge-").split("x")
    return merge, int(inputs), int(outputs)


def heading(shape):
    merge, inputs, outputs = shape_key(shape)
    return f"{'merge ' if merge else ''}{inputs} → {outputs}"


def median(values):
    return statistics.median(values) if values else None


def errors(found):
    """{shape: {run: error}} of the shapes that failed."""
    table = {}
    for r in found:
        if "error" in r:
            table.setdefault(r["shape"], {})[r["run"]] = r["error"]
    return table


def cells(found):
    """{shape: {row: {"total_ms", "download_ms", "downloaded_bytes"}}}"""
    table = {}
    for shape in sorted({r["shape"] for r in found}, key=shape_key):
        table[shape] = {}
        for row, run, which in ROWS:
            rounds = [
                r for r in found if r["shape"] == shape and r["run"] == run and "error" not in r
            ]
            rounds = [r for r in rounds if (r["round"] == 0) == (which == "first")]
            if not rounds:
                continue
            table[shape][row] = {
                "total_ms": median([r["total_ms"] for r in rounds]),
                "download_ms": median([r["download_ms"] for r in rounds]),
                "downloaded_bytes": max(r["downloaded_bytes"] for r in rounds),
            }
    return table


def stages(found):
    """{shape: {stage: median ms}}; the load is the restart run's first."""
    table = {}
    for shape in sorted({r["shape"] for r in found}, key=shape_key):
        rounds = [r for r in found if r["shape"] == shape and "error" not in r]
        restart = [r for r in rounds if r["run"] == "restart" and r["round"] == 0]
        table[shape] = {
            "load_ms": median([r["load_ms"] for r in restart]),
            **{
                stage: median([r[stage] for r in rounds])
                for stage in STAGES
                if stage != "load_ms"
            },
        }
    return table


def seconds(ms):
    return f"{ms / 1000:.2f} s"


def change(now, before, flag):
    if before in (None, 0) or now is None:
        return ""
    delta = (now - before) / before * 100
    mark = " **!**" if abs(delta) >= flag else ""
    return f" ({delta:+.0f} %{mark})"


def scenario_text(report, shape, row):
    cell = (report or {}).get("cells", {}).get(shape, {}).get(row)
    if not cell:
        return "failed" if shape in (report or {}).get("errors", {}) else ""
    text = seconds(cell["total_ms"])
    if cell["downloaded_bytes"]:
        text += (
            f" (download: {seconds(cell['download_ms'])},"
            f" {cell['downloaded_bytes'] / 1048576:.1f} MiB)"
        )
    return text


def stage_text(report, shape, stage):
    now = (report or {}).get("stages", {}).get(shape, {}).get(stage)
    return "" if now is None else seconds(now)


def table(report, baseline, shapes, name, keys, text, value, flag):
    """With a baseline, every row twice: `before` (the baseline), then
    `after` (this report) with the change."""
    if baseline is None:
        lines = [f"| {name} | " + " | ".join(heading(s) for s in shapes) + " |"]
        lines.append("|---|" + "---|" * len(shapes))
        for key, label in keys:
            lines.append(
                f"| {label} | " + " | ".join(text(report, s, key) for s in shapes) + " |"
            )
        return lines
    lines = [f"| {name} | | " + " | ".join(heading(s) for s in shapes) + " |"]
    lines.append("|---|---|" + "---|" * len(shapes))
    for key, label in keys:
        lines.append(
            f"| {label} | before | " + " | ".join(text(baseline, s, key) for s in shapes) + " |"
        )
        after = [
            text(report, s, key)
            + change(value(report, s, key), value(baseline, s, key), flag)
            for s in shapes
        ]
        lines.append("| | after | " + " | ".join(after) + " |")
    return lines


def total(report, shape, row):
    cell = (report or {}).get("cells", {}).get(shape, {}).get(row)
    return cell and cell["total_ms"]


def stage(report, shape, key):
    return (report or {}).get("stages", {}).get(shape, {}).get(key)


def markdown(report, baseline, title, flag, before=None, after=None, intro=True):
    """`before` and `after` name the two commits, as markdown."""
    shapes = sorted(
        set(report["cells"]) | set((baseline or {}).get("cells", {})), key=shape_key
    )
    lines = [f"### {title}", ""] if title else []
    if after:
        lines.append(
            f"**Before:** {before}. **After:** {after}."
            if baseline and before
            else f"**After:** {after}. No report of `main` to compare with yet."
        )
        lines.append("")
    if intro:
        lines.append(
            "Medians of one device, by shape (notes → outputs). Runner numbers "
            "are for comparing commits, not for how fast a phone is."
        )
        lines.append("")
    lines += table(
        report, baseline, shapes, "by scenario",
        [(row, row) for row, _, _ in ROWS], scenario_text, total, flag,
    )
    lines.append("")
    lines += table(
        report, baseline, shapes, "by stage",
        [(s, s.removesuffix("_ms").replace("load", "key load")) for s in STAGES],
        stage_text, stage, flag,
    )
    failures = [
        f"- {heading(shape)}, {run} run: {error}"
        for shape, runs in report.get("errors", {}).items()
        for run, error in runs.items()
    ]
    if failures:
        lines += ["", "Failed:", ""] + failures
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--logs", required=True)
    parser.add_argument("--baseline")
    parser.add_argument("--before", help="the baseline's commit, as markdown")
    parser.add_argument("--after", help="this report's commit, as markdown")
    parser.add_argument("--out")
    parser.add_argument("--summary")
    parser.add_argument("--title", default="Proof timing")
    parser.add_argument("--flag", type=float, default=FLAG, help="percent change to mark")
    args = parser.parse_args()

    found = records(args.logs)
    if not found:
        print(f"no ZOLANA_TIMING lines under {args.logs}", file=sys.stderr)
        return 1
    report = {"cells": cells(found), "stages": stages(found)}
    if errors(found):
        report["errors"] = errors(found)
    baseline = None
    if args.baseline and Path(args.baseline).is_file():
        baseline = json.loads(Path(args.baseline).read_text())
    text = markdown(report, baseline, args.title, args.flag, args.before, args.after)
    print(text)
    if args.out:
        Path(args.out).write_text(json.dumps(report, indent=2) + "\n")
    if args.summary:
        with open(args.summary, "a") as summary:
            summary.write(text + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
