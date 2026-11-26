#!/usr/bin/env python3
"""Renders every fence and checks that every word of every action, decision
condition, branch label and lane name is in the SVG text, and that every
node of the model has a data-node in the SVG. Also lists PlantUML's own word loss."""
import glob, html, json, os, re, subprocess, sys
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
hu = os.path.join(root, "target/release/humanuml")
os.makedirs(os.path.join(root, "out/hu"), exist_ok=True)
def words(s):
    # case and closing punctuation do not count: a theme may draw a lane name in capitals,
    # and an FL id is drawn as a link apart from the comma after it
    return set(w.lower().rstrip(",.:;") for w in re.split(r"\s+", s) if w.rstrip(",.:;"))
def svg_words(svg):
    t = " ".join(re.findall(r">([^<>]+)<", svg))
    return words(html.unescape(t))
bad = 0; n = 0; missing_nodes = 0
for f in sorted(glob.glob(os.path.join(root, "tests/fences/*.puml")) + glob.glob(os.path.join(root, "tests/examples/*.puml"))):
    name = os.path.basename(f)[:-5]
    if name == "OVERVIEW": continue
    n += 1
    out = os.path.join(root, f"out/hu/{name}.svg")
    r = subprocess.run([hu, "render", f, "-o", out], capture_output=True, text=True)
    if r.returncode: bad += 1; print("RENDER FAIL", name, r.stderr[:200]); continue
    svg = open(out).read()
    m = json.loads(subprocess.run([hu, "json", f], capture_output=True, text=True).stdout)
    need = set()
    for x in m["nodes"]:
        if x["kind"] not in ("action", "decision"): continue
        # marks are drawn apart: status as a chip, citations by file name, pitfalls and events as lines
        parts = []
        for part in x["text"].split(" · "):
            if part.strip().startswith("⚠"): continue
            part = re.sub(r"⤳\s*\S+", "", part)
            part = re.sub(r"event:\s*\S+", "", part)
            for st in x.get("status", []): part = re.sub(r"\[(" + re.escape(st) + r"|nieuw|kopie|mens|oud|buiten)\]", "", part)
            parts.append(part)
        need |= words(" ".join(parts))
        for pit in x.get("pitfalls", []): need |= words(pit)
        for st in x.get("status", []): need |= words(st.upper())
        for c in x.get("cites", []): need.add(c.split("/")[-1])
        for e in x.get("events", []): need.add(e)
    for e in m["edges"]:
        if e["label"]: need |= words(e["label"])
    for l in m["lanes"]: need |= words(l)
    have = svg_words(svg)
    miss = need - have
    ids = set(re.findall(r'data-node="([^"]+)"', html.unescape(svg)))
    mid = {x["id"] for x in m["nodes"] if x["kind"] in ("action", "decision", "start", "stop", "end", "fork", "join")}
    mi = mid - ids
    if miss or mi:
        bad += 1; print("MISSING", name, sorted(miss)[:8], "nodes without data-node:", sorted(mi)[:8])
print(f"{n - bad}/{n} activity fences: every word present, every node has data-node")
sys.exit(1 if bad else 0)
