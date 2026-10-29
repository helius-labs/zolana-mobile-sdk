#!/usr/bin/env python3
"""Sync the reviewed upstream zopro manifest; protocol pins stay independent."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import urllib.request

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--check", action="store_true", help="fail on drift without writing")
args = parser.parse_args()
source = json.loads((root / "key-catalog-source.json").read_text())
path = "prover/server/prover/provingkeys/zopro-keys.lock"
if os.environ.get("ZOLANA_SRC"):
    data = subprocess.check_output(["git", "-C", os.environ["ZOLANA_SRC"], "show",
                                    f"{source['revision']}:{path}"])
else:
    url = f"https://raw.githubusercontent.com/helius-labs/zolana/{source['revision']}/{path}"
    with urllib.request.urlopen(url, timeout=30) as response:
        data = response.read(2 * 1024 * 1024)
if hashlib.sha256(data).hexdigest() != source["sha256"]:
    raise SystemExit("upstream zopro manifest does not match the reviewed SHA-256")
manifest = json.loads(data)
destination = root / "packages/zolana_mobile/native/zolana-mobile/zopro-keys.lock"
if args.check:
    if json.loads(destination.read_text()) != manifest:
        raise SystemExit("zopro-keys.lock differs from the reviewed catalog; run scripts/sync-key-catalog.py")
else:
    destination.write_text(json.dumps(manifest, indent=2) + "\n")
print(f"{len(manifest['keys'])} zopro artifacts match upstream {source['revision'][:12]}")
