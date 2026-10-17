#!/usr/bin/env python3
"""The proof timing report of the phones from the `ZOLANA_TIMING {json}`
lines of the `fresh` and `restart` runs of
integration_test/proof_timing_test.dart.

    proof-timing-report.py --logs DIR [--devices devices.json] [--repo OWNER/NAME]
                           [--after TEXT] [--summary FILE] [--flag PERCENT]

DIR holds a directory per phone, `<label>/`, with its timing logs (`*.log`)
and, in CI, `baseline.json`: the phone's last entry in the history of `main`
(`{"commit", "date", "report"}`). The script writes `<label>/report.json`
and prints the report: the proof time by phone, then each phone's tables.
In those, rows are the four scenarios of the client's table and then the
stages; columns are the shapes, transfers (`8x2`) and then merges
(`merge-24x1`); a scenario cell is the total, with the download in brackets
when there was one, or `failed`. With a baseline, each row comes twice,
`before` (the baseline) and `after` (these logs) with the change, and a
change past --flag percent is marked; --after names this commit.
devices.json (scripts/devicefarm/) names the phones and orders them.
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
# The proof's verification stays in report.json but out of the tables: it
# is a check of the prover, not a cost of proving.
STAGES = ["load_ms", "witness_ms", "prove_ms"]
# A change in time below this is not reported: a witness of 1 ms against
# 3 ms is +200 % of nothing.
LEAST_CHANGE_MS = 50
# Two runs of one commit on a phone differ by up to about 4 %.
FLAG = 10.0
DEVICES = Path(__file__).with_name("devicefarm") / "devices.json"


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
    if before in (None, 0) or now is None or abs(now - before) < LEAST_CHANGE_MS:
        return ""
    delta = (now - before) / before * 100
    if round(delta) == 0:
        return ""
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


def markdown(report, baseline, flag, before=None, after=None):
    """One phone's tables; `before` and `after` name the two commits, as
    markdown."""
    shapes = sorted(
        set(report["cells"]) | set((baseline or {}).get("cells", {})), key=shape_key
    )
    lines = []
    if after:
        lines.append(
            f"**Before:** {before}. **After:** {after}."
            if baseline and before
            else f"**After:** {after}. No report of `main` to compare with yet."
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


def devices(path):
    """{label: device} in the order of devices.json."""
    found = json.loads(Path(path).read_text()) if Path(path).is_file() else []
    return {d["label"]: d for d in found}


def phone(known, label):
    device = known.get(label)
    return f"{device['name']} ({device['tier']})" if device else label


def proof_text(reports, shape, label):
    report = reports.get(label)
    text = stage_text(report, shape, "prove_ms")
    return text or ("failed" if shape in (report or {}).get("errors", {}) else "")


def proof(reports, shape, label):
    return stage(reports.get(label), shape, "prove_ms")


def commit_link(commit, repo):
    short = f"`{commit[:7]}`"
    return f"[{short}](https://github.com/{repo}/commit/{commit})" if repo else short


def phones_markdown(reports, baselines, known, flag, repo="", after=None):
    """The proof time by phone, then each phone's tables. `reports` is
    {label: report.json}, `baselines` {label: history entry}."""
    labels = [label for label in known if label in reports] + sorted(set(reports) - set(known))
    shapes = sorted({s for r in reports.values() for s in r["cells"]}, key=shape_key)
    before = ", ".join(
        f"`main` at {commit_link(c, repo)}"
        for c in dict.fromkeys(baselines[label]["commit"] for label in labels if label in baselines)
    )
    lines = ["### Proof timing on phones", ""]
    if after:
        lines.append(
            f"**Before:** {before}. **After:** {after}."
            if before
            else f"**After:** {after}. No report of `main` to compare with yet."
        )
        lines.append("")
    lines.append(
        "The proof, key loaded, of release builds in AWS Device Farm, by shape "
        "(notes → outputs): medians of each phone. Each phone's key load, "
        "witness and first runs are in its tables below."
    )
    lines.append("")
    lines += table(
        reports,
        {label: entry["report"] for label, entry in baselines.items()} if baselines else None,
        shapes, "by phone", [(label, phone(known, label)) for label in labels],
        proof_text, proof, flag,
    )
    for label in labels:
        device = known.get(label)
        summary = f"{device['name']}, {device['os']}, {device['tier']} tier" if device else label
        baseline = baselines.get(label)
        lines += ["", f"<details><summary>{summary}</summary>", ""]
        lines.append(markdown(reports[label], baseline and baseline["report"], flag))
        lines.append("</details>")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--logs", required=True)
    parser.add_argument("--devices", default=DEVICES)
    parser.add_argument("--repo", default="", help="OWNER/NAME, for commit links")
    parser.add_argument("--after", help="this report's commit, as markdown")
    parser.add_argument("--summary")
    parser.add_argument("--flag", type=float, default=FLAG, help="percent change to mark")
    args = parser.parse_args()

    reports = {}
    baselines = {}
    for path in sorted(p for p in Path(args.logs).iterdir() if p.is_dir()):
        found = records(path)
        if not found:
            print(f"no ZOLANA_TIMING lines under {path}", file=sys.stderr)
            continue
        report = {"cells": cells(found), "stages": stages(found)}
        if errors(found):
            report["errors"] = errors(found)
        (path / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        reports[path.name] = report
        if (path / "baseline.json").is_file():
            baselines[path.name] = json.loads((path / "baseline.json").read_text())
    if not reports:
        print(f"no ZOLANA_TIMING lines under {args.logs}", file=sys.stderr)
        return 1
    text = phones_markdown(
        reports, baselines, devices(args.devices), args.flag, args.repo, args.after
    )
    print(text)
    if args.summary:
        with open(args.summary, "a") as summary:
            summary.write(text + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
