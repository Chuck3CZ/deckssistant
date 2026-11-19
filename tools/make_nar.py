#!/usr/bin/env python3
"""Pack manifest.json + the prebuilt (asset-stripped) wasm and its icon assets into an installable Home Control .nar (no compiler needed).

  python3 tools/make_nar.py --manifest widgets-wasm/home-control/manifest.json --out home-control.nar
"""
import argparse, os, shutil, subprocess, sys, tempfile

LAUNCHER = """#!/bin/sh
exec /run/current-profile/bin/bmc-wasm-thin-v0 \\
    --wasm "${0%/*}/../lib/wasm/home-control.wasm" \\
    --asset-root "${0%/*}/../lib/assets" \\
    "$@"
"""

ap = argparse.ArgumentParser()
ap.add_argument("--manifest", required=True)
ap.add_argument("--wasm", default="prebuilt/home-control.wasm")
ap.add_argument("--assets", default="prebuilt/assets")
ap.add_argument("--out", required=True)
a = ap.parse_args()
here = os.path.dirname(os.path.abspath(__file__))
with tempfile.TemporaryDirectory() as tmp:
    w = os.path.join(tmp, "lib/bmc-widgets/community/home-control")
    os.makedirs(os.path.join(w, "bin")); os.makedirs(os.path.join(w, "lib/wasm"))
    open(os.path.join(w, "bin/home-control"), "w").write(LAUNCHER)
    os.chmod(os.path.join(w, "bin/home-control"), 0o755)
    shutil.copy(a.wasm, os.path.join(w, "lib/wasm/home-control.wasm"))
    os.chmod(os.path.join(w, "lib/wasm/home-control.wasm"), 0o644)
    shutil.copytree(a.assets, os.path.join(w, "lib/assets"))
    for root, _, files in os.walk(os.path.join(w, "lib/assets")):
        for f in files:
            os.chmod(os.path.join(root, f), 0o644)
    shutil.copy(a.manifest, os.path.join(w, "manifest.json"))
    subprocess.run([sys.executable, "-I", os.path.join(here, "nar.py"), "pack", tmp, a.out], check=True)
print("wrote", a.out)
