#!/usr/bin/env python3
"""For every fence: counts of actions, decisions and lanes in `humanuml json`
must equal the counts taken from the fence text. Also: every edge names real
nodes, and every node except `start` has an incoming edge."""
import glob, json, os, re, subprocess, sys
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
hu = os.path.join(root, "target/release/humanuml")
bad = 0; n = 0; tot = {"actions": 0, "decisions": 0, "lanes": 0, "nodes": 0, "edges": 0}
for f in sorted(glob.glob(os.path.join(root, "tests/fences/*.puml")) + glob.glob(os.path.join(root, "tests/examples/*.puml"))):
    text = open(f).read()
    if "OVERVIEW" in f:
        continue
    n += 1
    m = json.loads(subprocess.run([hu, "json", f], capture_output=True, text=True, check=True).stdout)
    lines = [l.strip() for l in text.splitlines()]
    t_actions = sum(1 for l in lines if l.startswith(":") or l.startswith("backward :"))
    t_dec = sum(1 for l in lines if re.match(r"(if|elseif|switch|while)\b", l) or l.startswith("repeat while"))
    t_lanes = len({l for l in lines if re.fullmatch(r"\|[^|]+\|", l)})
    kinds = [x["kind"] for x in m["nodes"]]
    got = {"actions": kinds.count("action"), "decisions": kinds.count("decision"), "lanes": len(m["lanes"])}
    exp = {"actions": t_actions, "decisions": t_dec, "lanes": t_lanes}
    ids = {x["id"] for x in m["nodes"]}
    ok = got == exp and len(ids) == len(m["nodes"])
    for e in m["edges"]:
        if e["from"] not in ids or e["to"] not in ids: ok = False
    incoming = {e["to"] for e in m["edges"]}
    lone = [x["id"] for x in m["nodes"] if x["kind"] != "start" and x["id"] not in incoming]
    if lone: ok = False
    if not ok:
        bad += 1; print("MISMATCH", os.path.basename(f), got, exp, "no incoming:", lone)
    for k in got: tot[k] += got[k]
    tot["nodes"] += len(m["nodes"]); tot["edges"] += len(m["edges"])
print(f"{n - bad}/{n} activity fences: actions, decisions and lanes match the fence text. totals: {tot}")
sys.exit(1 if bad else 0)
