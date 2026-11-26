# Results of the first version (26 Nov 2026)

Input: the 125 real flow fences of Like A Human (kept private, not in this repo; the checks read `tests/fences/` when it exists, and always `tests/examples/`). The contact sheets are private too.
Reference: PlantUML native v1.2026.10. Reproduce: `cargo build --release`, then the commands below.

## Step 1: `humanuml check`

```
$ humanuml check tests/fences/*.puml
ok: 125 diagrams

$ plantuml -checkonly tests/fences/*.puml
exit 0

$ python3 tests/check_broken.py
humanuml: 28/28 broken fences give an error on the expected line with the expected hint
PlantUML -checkonly fails on 19/28; accepts (we are stricter): 03-unclosed-if, 04-unclosed-switch, 05-unclosed-while, 06-unclosed-repeat, 07-unclosed-fork, 09-missing-then, 11-unclosed-paren, 20-end-if-two-words, 28-empty-action

$ python3 tests/check_model.py
124/124 activity fences: actions, decisions and lanes match the fence text. totals: {'actions': 1435, 'decisions': 468, 'lanes': 91, 'nodes': 2307, 'edges': 2723}
```

- All 125 fences pass, and PlantUML `-checkonly` passes all 125.
- 28 broken fences (`tests/broken/`, one mistake each, made by `tests/make_broken.py`): all 28 give the first error on the expected line, with the expected hint.
- One mistake gives one error. A typo of a keyword (`iff`, `endiff`, `elif`, `swich`, `fork agin`, `end if`, `repeat whle`) is reported once and the parser goes on as if the writer had typed the right word.
- **Where PlantUML and humanuml disagree** (PlantUML accepts, humanuml rejects): 9 of the 28. PlantUML accepts a block that is never closed (`if`, `switch`, `while`, `repeat`, `fork`), an `if (c) (yes)` with no `then`, a `(` that is never closed, `end if` and an empty action `:;`. We chose to be stricter: these are mistakes that a model makes. On the 125 real fences there is no disagreement. On the 19 other broken fences both fail.

Sample output:

```
tests/broken/14-wrong-closer.puml:5:1: error: expected `endwhile` to close the `while` from line 3, found `endif`
   |
 5 | endif
   | ^
   = hint: did you mean `endwhile`?
```

## Step 2: `humanuml json`

Shape: `docs/model.md`.

```
$ python3 tests/check_model.py
124/124 activity fences: actions, decisions and lanes match the fence text. totals: {'actions': 1435, 'decisions': 468, 'lanes': 91, 'nodes': 2307, 'edges': 2723}
```

For each of the 124 activity fences the number of actions, decisions and lanes equals the count taken from the fence text. Every edge names real nodes. Every node except `start` has an incoming edge. (The check uses the fence text, not the PlantUML SVG.)

## Step 3: `humanuml render`

```
$ python3 tests/check_render.py
124/124 activity fences: every word present, every node has data-node
```

- All 125 fences render. In the 124 activity fences every word of every action, decision, branch label and lane name is in the SVG, and every node has a `data-node` id.
- Layout: nested boxes, top to bottom, real swimlane columns, flat `switch`. FL-1.6.6 draws its five cases as five flat branches under one bus line (PlantUML puts two of them on one line).

## Step 4: numbers

Machine: this Mac. Best of 5. The timer is `python3` and adds about 20 ms per measurement, so the one-process numbers are an upper bound. Script: `tests/bench.sh`.

| run | time for 125 fences |
|---|---|
| `humanuml check` (1 process) | 33 ms |
| `humanuml render` (1 process) | 80 ms |
| `humanuml check` (125 processes) | 493 ms |
| `humanuml json` (125 processes) | 551 ms |
| `humanuml render` (125 processes) | 570 ms |
| `plantuml -checkonly` (1 process) | 67 ms |
| `plantuml -tsvg` (1 process) | 1110 ms |

A process start costs about 4 ms for humanuml. PlantUML native needs one process for all files, so compare the one-process rows: render is about 14 times faster; check is about equal (PlantUML `-checkonly` is also fast).

## What is not done

- Notes: parsed (`note left|right: text`, multi-line) and in the JSON, but not drawn.
- Not in the subset (named error): `split`, `detach`, `kill`. Not parsed: colors, `skinparam` effects, `title`, `!include`, action styles `:text|`, `:text>`.
- The `OVERVIEW` state diagram is drawn as a plain grid of columns with curved lines (by the number of the FL id). It is readable only as an index. It is a different diagram type and outside the first scope.
- No crossing avoidance for edges. No font file is used: the text width comes from Helvetica metrics, so a viewer with another font can show small width differences.
- The gate rules of `uml-server/check.mjs` (ids, handovers, citations) are not ported. The JSON model has what they need.

## What draws worse than PlantUML

- A `switch` or `if` whose branches stand in different lanes (FL-1.2 has 12 cases over 5 lanes) is very wide: each lane column is as wide as the cases it holds side by side, and the horizontal bus line crosses the lane borders.
- An `if` without `else` draws its empty branch as a line around the right side of the then-branch, with a merge diamond.
- A loop (`while`, `repeat`) takes room on both sides; long loop bodies with wide nodes make wide pictures.
- An `elseif` chain nests to the right (as PlantUML does), not flat.
- Very long action texts (some have 1000 characters) wrap at 260 px into tall boxes; there is no abbreviation.
