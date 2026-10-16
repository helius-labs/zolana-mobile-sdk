#!/usr/bin/env python3
"""The proof timing history of `main`, kept on the `proof-timing` branch: a
line per commit in `history/<label>.jsonl` for each label's report.json,
and a README with each label's latest report against the commit before it
and the medians of its last commits.

    proof-timing-history.py --reports DIR --history DIR --commit SHA
                            [--date ISO] [--repo OWNER/NAME] [--last N]

--reports holds a directory per CI artifact, `proof-timing-<label>/report.json`;
--history is a checkout of the `proof-timing` branch, possibly empty.
"""

import argparse
import importlib.util
import json
import sys
from pathlib import Path

# The report script is loaded from its file; no bytecode next to it.
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location(
    "report", Path(__file__).with_name("proof-timing-report.py")
)
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)

TRENDS = [
    ("warm", "Warm proof"),
    ("first run after restart", "First proof after a restart"),
]


def entries(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def link(entry, repo):
    short = f"`{entry['commit'][:7]}`"
    return f"[{short}](https://github.com/{repo}/commit/{entry['commit']})" if repo else short


def section(label, found, repo, last):
    latest = found[-1]
    before = found[-2] if len(found) > 1 else None
    lines = [f"## {label}", ""]
    lines.append(
        report.markdown(
            latest["report"],
            before and before["report"],
            None,
            report.FLAG,
            before=before and f"{link(before, repo)} of {before['date'][:10]}",
            after=f"{link(latest, repo)} of {latest['date'][:10]}",
            intro=False,
        )
    )
    recent = found[-last:]
    shapes = sorted({s for e in recent for s in e["report"]["cells"]}, key=report.shape_key)
    for row, title in TRENDS:
        lines += [f"### {title}, last {len(recent)} commits", ""]
        lines.append("| commit | date | " + " | ".join(report.heading(s) for s in shapes) + " |")
        lines.append("|---|---|" + "---|" * len(shapes))
        for entry in reversed(recent):
            cells = entry["report"]["cells"]
            values = [
                report.seconds(cells[s][row]["total_ms"]) if row in cells.get(s, {}) else ""
                for s in shapes
            ]
            lines.append(
                f"| {link(entry, repo)} | {entry['date'][:10]} | " + " | ".join(values) + " |"
            )
        lines.append("")
    return "\n".join(lines)


def readme(history, repo, last):
    lines = [
        "# Proof timing on main",
        "",
        "Written by CI after every push to `main` (`scripts/proof-timing-history.py`"
        " on `main`), from the proof timing report of each runner. Runner numbers"
        " compare commits, not phones: about 20 % between two commits can be the"
        " runner. The data is in `history/`, a line per commit.",
        "",
    ]
    for path in sorted(history.glob("history/*.jsonl")):
        found = entries(path)
        if found:
            lines.append(section(path.stem, found, repo, last))
    return "\n".join(lines).rstrip() + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--reports", required=True)
    parser.add_argument("--history", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--date", default="")
    parser.add_argument("--repo", default="")
    parser.add_argument("--last", type=int, default=20)
    args = parser.parse_args()

    history = Path(args.history)
    added = 0
    for path in sorted(Path(args.reports).glob("proof-timing-*/report.json")):
        label = path.parent.name.removeprefix("proof-timing-")
        target = history / "history" / f"{label}.jsonl"
        target.parent.mkdir(parents=True, exist_ok=True)
        if any(e["commit"] == args.commit for e in (entries(target) if target.exists() else [])):
            continue
        entry = {"commit": args.commit, "date": args.date, "report": json.loads(path.read_text())}
        with target.open("a") as out:
            out.write(json.dumps(entry, separators=(",", ":")) + "\n")
        added += 1
    if not added:
        print(f"no new reports under {args.reports}", file=sys.stderr)
        return 0
    (history / "README.md").write_text(readme(history, args.repo, args.last))
    print(f"{added} reports added to {history}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
