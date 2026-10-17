#!/usr/bin/env python3
"""The proof timing history of `main`, kept on the `proof-timing` branch: a
line per commit in `history/<label>.jsonl` for each phone's report.json,
and a README with each phone's latest report against the commit before it
and the medians of its last commits.

    proof-timing-history.py --reports DIR --history DIR --commit SHA
                            [--date ISO] [--repo OWNER/NAME] [--last N]

--reports holds a directory per phone, `<label>/report.json`
(scripts/proof-timing-report.py); --history is a checkout of the
`proof-timing` branch, possibly empty.
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

# The proof as in the pull request comments, and what a spend after a
# restart waits for.
TRENDS = [
    ("Proof", lambda r, s: report.stage(r, s, "prove_ms")),
    ("First run after a restart", lambda r, s: report.total(r, s, "first run after restart")),
]


def entries(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def link(entry, repo):
    return report.commit_link(entry["commit"], repo)


def section(title, found, repo, last):
    latest = found[-1]
    before = found[-2] if len(found) > 1 else None
    lines = [f"## {title}", ""]
    lines.append(
        report.markdown(
            latest["report"],
            before and before["report"],
            report.FLAG,
            before=before and f"{link(before, repo)} of {before['date'][:10]}",
            after=f"{link(latest, repo)} of {latest['date'][:10]}",
        )
    )
    recent = found[-last:]
    shapes = sorted({s for e in recent for s in e["report"]["cells"]}, key=report.shape_key)
    for title, value in TRENDS:
        lines += [f"### {title}, last {len(recent)} commits", ""]
        lines.append("| commit | date | " + " | ".join(report.heading(s) for s in shapes) + " |")
        lines.append("|---|---|" + "---|" * len(shapes))
        for entry in reversed(recent):
            values = [
                "" if (ms := value(entry["report"], s)) is None else report.seconds(ms)
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
        " on `main`), from release builds on the phones of"
        " `scripts/devicefarm/devices.json` in AWS Device Farm. Two runs of one"
        " commit differ by up to 5 % on the Galaxy A16 and A56 and up to 15 % on"
        " the S25 and the iPhones. The data is in"
        " `history/`, a line per commit.",
        "",
    ]
    known = report.devices(report.DEVICES)
    paths = {path.stem: path for path in history.glob("history/*.jsonl")}
    for label in [label for label in known if label in paths] + sorted(set(paths) - set(known)):
        found = entries(paths[label])
        if found:
            device = known.get(label)
            title = f"{device['name']}, {device['os']}, {device['tier']} tier" if device else label
            lines.append(section(title, found, repo, last))
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
    for path in sorted(Path(args.reports).glob("*/report.json")):
        label = path.parent.name
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
