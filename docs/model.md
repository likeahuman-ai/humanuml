# The JSON model

`humanuml json <file>` prints one object per diagram (an array when a `.md` file holds several fences).

```json
{
  "name": "FL-1_6_6",
  "kind": "activity",
  "lanes": ["participant", "trainer"],
  "nodes": [ ... ],
  "edges": [ ... ],
  "tree": [ ... ]
}
```

## Nodes

| field | meaning |
|---|---|
| `id` | Stable id. The author's own key when the text starts `g · ...` (then the id is `g`), otherwise `n<ordinal>`. A repeated key gets `_2`, `_3`. Unique in the diagram. |
| `kind` | `start`, `stop`, `end`, `action`, `decision`, `fork`, `join`, `note`, `state` (overview only). |
| `construct` | Only on a decision: `if`, `elseif`, `switch`, `while`, `repeat`. |
| `text` | The text as written, with the key. For a decision: the condition or the question. |
| `key` | The letter before ` · `, or `null`. A key is 1 to 3 characters of `a-z0-9`. |
| `lane` | The swimlane name that is active where the node stands, or `null` when the diagram has no lanes yet. |
| `parent` | The id of the decision or fork that holds this node, or `null` at the top level. |
| `branch` | The branch of the parent: `then`, `elseif`, `else`, `case`, `body`, `backward`, `branch`. |
| `backward` | `true` on the `backward :text;` action of a `repeat`. |
| `status` | The status marks in the text: `["new"]`. Only on actions and decisions, only when present. |
| `cites` | The citations `⤳ path:line` in the text: `["src/queries/invites.ts:75"]`. |
| `pitfalls` | The parts after `⚠`. |
| `events` | The names after `event:`. |
| `refs` | The flow ids in the text: `["FL-1.4"]`. |
| `line`, `col` | Source position, 1-based. For a fence inside a `.md` file the line is the line of the file. |

The marks stay in `text` as written. Their notation: `src/marks.rs`.

A `repeat` decision stands at the line of `repeat`; its condition text comes from `repeat while`.
A `join` node closes a `fork` (it only exists when at least one branch does not end).

## Edges

`{ "from", "to", "label", "back" }`

- `label` is the branch label that stands on the edge: `then (yes)`, `else (no)`, `case (label)`, `is (label)`, `not (label)`, the `endwhile (label)`. It is `null` when there is none.
- `back: true` marks an edge that returns up in a loop (end of a `while` body to its decision, the `repeat while` decision back to the first node of the body, through the `backward` action when there is one).
- There are no merge nodes. The nodes after an `endif`, `endswitch` or `end fork` have one incoming edge from each branch that does not stop.
- An `if` without `else` has an edge from the decision to the next node, with no label.

## Tree

`tree` is the nesting. Each item has `id` and `type`:

- leaf: `start`, `stop`, `end`, `action` (with `text`), `note`
- `if`: `branches` = `[{kind: then|elseif|else, decision, label, body}]`
- `switch`: `cases` = `[{label, body}]`
- `while`: `label`, `exit_label`, `body`
- `repeat`: `label` (the `is` label), `exit_label` (the `not` label), `backward` (an action item or `null`), `body`
- `fork`: `branches` = `[{body}]`

`body` is an array of items. Swimlane changes are not items: they only set `lane` on the nodes after them.

## State diagrams (the derived overview)

A fence with `state "title" as id` and `a --> b` lines has `kind: "state"`. Its nodes have kind `state`, `id` = the id, `text` = the title. Its edges are the transitions. It has no tree.

## Counts

`tests/check_model.py` takes, for each fence, the number of actions, decisions and lanes from the fence text and compares them with the model.
