#!/usr/bin/env python3
"""Runs humanuml check on every broken fence: the first error must be on the
expected line and carry a hint with the expected text. Also asks PlantUML
(-checkonly) whether it fails too, and prints where the two tools disagree."""
import os, re, subprocess, sys
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
hu = os.path.join(root, "target/release/humanuml")
pu = os.environ.get("PLANTUML", os.path.expanduser("~/.config/backbone/bin/plantuml"))
bd = os.path.join(root, "tests/broken")
ok = 0; disagree = []; fails = 0
for line in open(os.path.join(bd, "expect.tsv")):
    name, exp_line, hint = line.rstrip("\n").split("\t")
    f = os.path.join(bd, name + ".puml")
    r = subprocess.run([hu, "check", f], capture_output=True, text=True)
    m = re.search(r":(\d+):(\d+): error: (.*)", r.stderr)
    good = r.returncode == 1 and m and int(m.group(1)) == int(exp_line) and hint in r.stderr
    if good: ok += 1
    else:
        fails += 1
        print("FAIL", name, "expected line", exp_line, "hint", repr(hint)); print(r.stderr)
    # PlantUML is optional: set PLANTUML to its path to compare
    if os.path.exists(pu):
        p = subprocess.run([pu, "-checkonly", f], capture_output=True, text=True)
        if p.returncode == 0: disagree.append(name)
print(f"humanuml: {ok}/{ok+fails} broken fences give an error on the expected line with the expected hint")
if os.path.exists(pu):
    print(f"PlantUML -checkonly fails on {ok+fails-len(disagree)}/{ok+fails}; accepts (we are stricter): {', '.join(disagree)}")
sys.exit(1 if fails else 0)
