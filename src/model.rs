//! The flow as a graph: nodes with a stable id, edges with branch labels,
//! and the nesting as a tree. See docs/model.md.

use crate::ast::*;
use crate::json::Json;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    /// start | stop | end | action | decision | fork | join | note | state
    pub kind: &'static str,
    /// For a decision: if | elseif | switch | while | repeat.
    pub construct: Option<&'static str>,
    pub text: String,
    pub lane: Option<String>,
    pub line: usize,
    pub col: usize,
    /// The author's own letter in front of the text: `g · a filter?` has key `g`.
    pub key: Option<String>,
    /// The decision or fork node that holds this node.
    pub parent: Option<String>,
    /// Which branch of the parent: then, elseif, else, case, body, backward, branch.
    pub branch: Option<String>,
    pub backward: bool,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    /// A loop edge that goes back up.
    pub back: bool,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub name: String,
    pub kind: Kind,
    pub lanes: Vec<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub tree: Json,
    /// Node id by source position (line, col): the layout uses this.
    pub by_pos: HashMap<(usize, usize), String>,
}

#[derive(Clone)]
struct Pred {
    id: String,
    label: Option<String>,
}

struct B {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    lanes: Vec<String>,
    lane: Option<String>,
    used: HashSet<String>,
    ctr: usize,
    by_pos: HashMap<(usize, usize), String>,
}

/// `g · text` → (`g`, `text`). A key is 1 to 3 lowercase letters or digits.
pub fn split_key(text: &str) -> (Option<String>, &str) {
    if let Some(p) = text.find(" · ") {
        let k = &text[..p];
        if !k.is_empty() && k.len() <= 3 && k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return (Some(k.to_string()), &text[p + " · ".len()..]);
        }
    }
    (None, text)
}

impl B {
    fn new_id(&mut self, key: &Option<String>) -> String {
        self.ctr += 1;
        let base = match key {
            Some(k) => k.clone(),
            None => format!("n{}", self.ctr),
        };
        let mut id = base.clone();
        let mut n = 2;
        while self.used.contains(&id) {
            id = format!("{}_{}", base, n);
            n += 1;
        }
        self.used.insert(id.clone());
        id
    }

    #[allow(clippy::too_many_arguments)]
    fn node(&mut self, kind: &'static str, construct: Option<&'static str>, text: &str, pos: Pos, parent: &Option<(String, String)>, backward: bool) -> String {
        let (key, _) = split_key(text);
        let id = self.new_id(&key);
        self.nodes.push(Node {
            id: id.clone(),
            kind,
            construct,
            text: text.to_string(),
            lane: self.lane.clone(),
            line: pos.line,
            col: pos.col,
            key,
            parent: parent.as_ref().map(|p| p.0.clone()),
            branch: parent.as_ref().map(|p| p.1.clone()),
            backward,
        });
        self.by_pos.insert((pos.line, pos.col), id.clone());
        id
    }

    fn connect(&mut self, preds: &[Pred], to: &str) {
        for p in preds {
            self.edges.push(Edge { from: p.id.clone(), to: to.to_string(), label: p.label.clone(), back: false });
        }
    }

    fn seq(&mut self, body: &[Stmt], mut preds: Vec<Pred>, parent: &Option<(String, String)>) -> (Vec<Pred>, Json) {
        let mut tree = vec![];
        for s in body {
            let (p, j) = self.stmt(s, preds, parent);
            preds = p;
            if let Some(j) = j {
                tree.push(j);
            }
        }
        (preds, Json::Arr(tree))
    }

    fn leaf(&mut self, kind: &'static str, text: &str, pos: Pos, preds: Vec<Pred>, parent: &Option<(String, String)>) -> (String, Json) {
        let id = self.node(kind, None, text, pos, parent, false);
        self.connect(&preds, &id);
        let mut o = vec![("id".to_string(), Json::str(id.clone())), ("type".to_string(), Json::str(kind))];
        if kind == "action" {
            o.push(("text".to_string(), Json::str(text)));
        }
        (id, Json::Obj(o))
    }

    fn stmt(&mut self, s: &Stmt, preds: Vec<Pred>, parent: &Option<(String, String)>) -> (Vec<Pred>, Option<Json>) {
        match s {
            Stmt::Lane { name, .. } => {
                if !self.lanes.contains(name) {
                    self.lanes.push(name.clone());
                }
                self.lane = Some(name.clone());
                (preds, None)
            }
            Stmt::Start(p) => {
                let (id, j) = self.leaf("start", "start", *p, preds, parent);
                (vec![Pred { id, label: None }], Some(j))
            }
            Stmt::Stop(p) => {
                let (_, j) = self.leaf("stop", "stop", *p, preds, parent);
                (vec![], Some(j))
            }
            Stmt::End(p) => {
                let (_, j) = self.leaf("end", "end", *p, preds, parent);
                (vec![], Some(j))
            }
            Stmt::Action { text, pos } => {
                let (id, j) = self.leaf("action", text, *pos, preds, parent);
                (vec![Pred { id, label: None }], Some(j))
            }
            Stmt::Note { side, text, pos } => {
                let prev = preds.first().map(|p| p.id.clone());
                let id = self.node("note", None, text, *pos, parent, false);
                if let Some(pv) = prev {
                    self.edges.push(Edge { from: pv, to: id.clone(), label: Some(format!("note {}", side)), back: false });
                }
                (preds, Some(Json::Obj(vec![("id".into(), Json::str(id)), ("type".into(), Json::str("note")), ("text".into(), Json::str(text.as_str()))])))
            }
            Stmt::If { pos, parts } => {
                let mut exits: Vec<Pred> = vec![];
                let mut branches = vec![];
                let mut head: Option<String> = None;
                let mut incoming = preds;
                let mut last_dec: Option<String> = None;
                let mut has_else = false;
                let mut first_id = String::new();
                for part in parts {
                    match part.kind {
                        PartKind::Then | PartKind::ElseIf => {
                            let construct = if part.kind == PartKind::Then { "if" } else { "elseif" };
                            let par = if part.kind == PartKind::Then { parent.clone() } else { Some((head.clone().unwrap(), "elseif".to_string())) };
                            let d = self.node("decision", Some(construct), &part.cond, if part.kind == PartKind::Then { *pos } else { part.pos }, &par, false);
                            self.connect(&incoming, &d);
                            if head.is_none() {
                                head = Some(d.clone());
                                first_id = d.clone();
                            }
                            let bp = Some((d.clone(), if part.kind == PartKind::Then { "then".to_string() } else { "elseif".to_string() }));
                            let (ex, t) = self.seq(&part.body, vec![Pred { id: d.clone(), label: part.label.clone() }], &bp);
                            exits.extend(ex);
                            branches.push(Json::Obj(vec![("kind".into(), Json::str(if part.kind == PartKind::Then { "then" } else { "elseif" })), ("decision".into(), Json::str(d.clone())), ("label".into(), Json::opt(&part.label)), ("body".into(), t)]));
                            incoming = vec![Pred { id: d.clone(), label: None }];
                            last_dec = Some(d);
                        }
                        PartKind::Else => {
                            has_else = true;
                            let d = last_dec.clone().unwrap();
                            let bp = Some((first_id.clone(), "else".to_string()));
                            let (ex, t) = self.seq(&part.body, vec![Pred { id: d, label: part.label.clone() }], &bp);
                            exits.extend(ex);
                            branches.push(Json::Obj(vec![("kind".into(), Json::str("else")), ("label".into(), Json::opt(&part.label)), ("body".into(), t)]));
                        }
                        _ => {}
                    }
                }
                if !has_else {
                    if let Some(d) = last_dec {
                        exits.push(Pred { id: d, label: None });
                    }
                }
                (exits, Some(Json::Obj(vec![("id".into(), Json::str(first_id)), ("type".into(), Json::str("if")), ("branches".into(), Json::Arr(branches))])))
            }
            Stmt::Switch { pos, cond, parts } => {
                let d = self.node("decision", Some("switch"), cond, *pos, parent, false);
                self.connect(&preds, &d);
                let mut exits = vec![];
                let mut cases = vec![];
                for part in parts {
                    let bp = Some((d.clone(), "case".to_string()));
                    let (ex, t) = self.seq(&part.body, vec![Pred { id: d.clone(), label: part.label.clone() }], &bp);
                    exits.extend(ex);
                    cases.push(Json::Obj(vec![("label".into(), Json::opt(&part.label)), ("body".into(), t)]));
                }
                if parts.is_empty() {
                    exits.push(Pred { id: d.clone(), label: None });
                }
                (exits, Some(Json::Obj(vec![("id".into(), Json::str(d)), ("type".into(), Json::str("switch")), ("cases".into(), Json::Arr(cases))])))
            }
            Stmt::While { pos, cond, is_label, end_label, body } => {
                let d = self.node("decision", Some("while"), cond, *pos, parent, false);
                self.connect(&preds, &d);
                let bp = Some((d.clone(), "body".to_string()));
                let (ex, t) = self.seq(body, vec![Pred { id: d.clone(), label: is_label.clone() }], &bp);
                for p in ex {
                    self.edges.push(Edge { from: p.id.clone(), to: d.clone(), label: if p.id == d { p.label } else { None }, back: true });
                }
                let j = Json::Obj(vec![("id".into(), Json::str(d.clone())), ("type".into(), Json::str("while")), ("label".into(), Json::opt(is_label)), ("exit_label".into(), Json::opt(end_label)), ("body".into(), t)]);
                (vec![Pred { id: d, label: end_label.clone() }], Some(j))
            }
            Stmt::Repeat { pos, body, backward, cond, is_label, not_label } => {
                let before = self.nodes.len();
                let bp_body = Some((String::new(), "body".to_string()));
                // the decision comes last in the source but is created after the body;
                // its id is needed for the parent link, so the body gets its parent patched.
                let (ex, t) = self.seq(body, preds, &bp_body);
                let body_end = self.nodes.len();
                let head = if body_end > before { Some(self.nodes[before].id.clone()) } else { None };
                let d = self.node("decision", Some("repeat"), cond, *pos, parent, false);
                // the repeat decision is at the `repeat while` line; keep the `repeat` line for the tree
                for n in self.nodes[before..body_end].iter_mut() {
                    if n.parent.as_deref() == Some("") {
                        n.parent = Some(d.clone());
                    }
                }
                self.connect(&ex, &d);
                let mut back_json = Json::Null;
                if let Some(h) = head {
                    if let Some((btext, bpos)) = backward {
                        let bp = Some((d.clone(), "backward".to_string()));
                        let b = self.node("action", None, btext, *bpos, &bp, true);
                        self.edges.push(Edge { from: d.clone(), to: b.clone(), label: is_label.clone(), back: true });
                        self.edges.push(Edge { from: b.clone(), to: h, label: None, back: true });
                        back_json = Json::Obj(vec![("id".into(), Json::str(b)), ("type".into(), Json::str("action")), ("text".into(), Json::str(btext.as_str()))]);
                    } else {
                        self.edges.push(Edge { from: d.clone(), to: h, label: is_label.clone(), back: true });
                    }
                }
                let j = Json::Obj(vec![("id".into(), Json::str(d.clone())), ("type".into(), Json::str("repeat")), ("label".into(), Json::opt(is_label)), ("exit_label".into(), Json::opt(not_label)), ("backward".into(), back_json), ("body".into(), t)]);
                (vec![Pred { id: d, label: not_label.clone() }], Some(j))
            }
            Stmt::Fork { pos, parts } => {
                let f = self.node("fork", None, "fork", *pos, parent, false);
                self.connect(&preds, &f);
                let mut exits = vec![];
                let mut branches = vec![];
                for part in parts {
                    let bp = Some((f.clone(), "branch".to_string()));
                    let (ex, t) = self.seq(&part.body, vec![Pred { id: f.clone(), label: None }], &bp);
                    exits.extend(ex);
                    branches.push(Json::Obj(vec![("body".into(), t)]));
                }
                let j = Json::Obj(vec![("id".into(), Json::str(f.clone())), ("type".into(), Json::str("fork")), ("branches".into(), Json::Arr(branches))]);
                if exits.is_empty() {
                    (vec![], Some(j))
                } else {
                    let join = self.node("join", None, "end fork", *pos, parent, false);
                    // the fork keeps its position in the map; the join is found at column 0
                    self.by_pos.insert((pos.line, pos.col), f.clone());
                    self.by_pos.insert((pos.line, 0), join.clone());
                    self.connect(&exits, &join);
                    (vec![Pred { id: join, label: None }], Some(j))
                }
            }
        }
    }
}

pub fn build(d: &Diagram) -> Model {
    if d.kind == Kind::State {
        let mut nodes = vec![];
        let mut by_pos = HashMap::new();
        for s in &d.states {
            nodes.push(Node { id: s.id.clone(), kind: "state", construct: None, text: s.title.clone(), lane: None, line: s.pos.line, col: s.pos.col, key: None, parent: None, branch: None, backward: false });
            by_pos.insert((s.pos.line, s.pos.col), s.id.clone());
        }
        let edges = d.transitions.iter().map(|(a, b, _)| Edge { from: a.clone(), to: b.clone(), label: None, back: false }).collect();
        return Model { name: d.name.clone(), kind: d.kind, lanes: vec![], nodes, edges, tree: Json::Arr(vec![]), by_pos };
    }
    let mut b = B { nodes: vec![], edges: vec![], lanes: vec![], lane: None, used: HashSet::new(), ctr: 0, by_pos: HashMap::new() };
    let (_, tree) = b.seq(&d.body, vec![], &None);
    Model { name: d.name.clone(), kind: d.kind, lanes: b.lanes, nodes: b.nodes, edges: b.edges, tree, by_pos: b.by_pos }
}

impl Model {
    pub fn to_json(&self) -> Json {
        let nodes: Vec<Json> = self
            .nodes
            .iter()
            .map(|n| {
                let mut o = vec![("id".to_string(), Json::str(n.id.clone())), ("kind".to_string(), Json::str(n.kind))];
                if let Some(c) = n.construct {
                    o.push(("construct".into(), Json::str(c)));
                }
                o.push(("text".into(), Json::str(n.text.clone())));
                o.push(("key".into(), Json::opt(&n.key)));
                o.push(("lane".into(), Json::opt(&n.lane)));
                o.push(("parent".into(), Json::opt(&n.parent)));
                o.push(("branch".into(), Json::opt(&n.branch)));
                if n.backward {
                    o.push(("backward".into(), Json::Bool(true)));
                }
                o.push(("line".into(), Json::int(n.line)));
                o.push(("col".into(), Json::int(n.col)));
                Json::Obj(o)
            })
            .collect();
        let edges: Vec<Json> = self
            .edges
            .iter()
            .map(|e| {
                let mut o = vec![("from".to_string(), Json::str(e.from.clone())), ("to".to_string(), Json::str(e.to.clone())), ("label".to_string(), Json::opt(&e.label))];
                if e.back {
                    o.push(("back".into(), Json::Bool(true)));
                }
                Json::Obj(o)
            })
            .collect();
        Json::Obj(vec![
            ("name".into(), Json::str(self.name.clone())),
            ("kind".into(), Json::str(if self.kind == Kind::Activity { "activity" } else { "state" })),
            ("lanes".into(), Json::Arr(self.lanes.iter().map(|l| Json::str(l.clone())).collect())),
            ("nodes".into(), Json::Arr(nodes)),
            ("edges".into(), Json::Arr(edges)),
            ("tree".into(), self.tree.clone()),
        ])
    }
}

impl Clone for Json {
    fn clone(&self) -> Json {
        match self {
            Json::Null => Json::Null,
            Json::Bool(b) => Json::Bool(*b),
            Json::Num(n) => Json::Num(*n),
            Json::Str(s) => Json::Str(s.clone()),
            Json::Arr(a) => Json::Arr(a.clone()),
            Json::Obj(o) => Json::Obj(o.clone()),
        }
    }
}
