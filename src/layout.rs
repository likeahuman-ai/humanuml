//! Layout: nested boxes, top to bottom, with real swimlanes.
//!
//! Every block is laid out on its own, with its entry at the top and x = 0
//! in the column of its swimlane. A block reports how far it reaches left and
//! right in each lane (`ext`). Branches of an `if`, a `switch` or a `fork` are
//! placed side by side so that their extents in the same lane do not touch.
//! A `switch` with n cases is n flat branches under one bus line.
//! The lane columns are placed last, from the extents of the whole diagram.

use crate::ast::*;
use crate::font::{text_width, wrap};
use crate::model::Model;
use std::collections::HashMap;

pub const FS: f64 = 12.0; // font size of node text
pub const LFS: f64 = 11.0; // font size of edge labels
pub const LH: f64 = 15.0; // line height of node text
pub const LLH: f64 = 13.0; // line height of labels
const G: f64 = 22.0; // vertical gap between two nodes
const BRANCH_GAP: f64 = 26.0; // horizontal gap between two branches
const ACTION_MAX: f64 = 260.0;
const DECISION_MAX: f64 = 190.0;
const LABEL_MAX: f64 = 130.0;

#[derive(Clone, Copy, Debug)]
pub struct Port {
    pub lane: usize,
    pub dx: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Action,
    BackAction,
    Decision,
    Start,
    Stop,
    End,
    Merge,
    Label,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub shape: Shape,
    pub lane: usize,
    pub dx: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub lines: Vec<String>,
    pub id: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Edge {
    /// Down from one port to another; an elbow when the columns differ.
    Flow { from: Port, to: Port, ymid: Option<f64>, arrow: bool },
    /// From a decision to the top of a branch.
    Fan { dec: Port, dw: f64, dh: f64, to: Port, bus_y: Option<f64>, arrow: bool },
    Poly { pts: Vec<Port>, arrow: bool },
    /// A thick bar over the x range of the ports.
    Bar { pts: Vec<Port>, id: Option<String> },
}

type Ext = Vec<Option<(f64, f64)>>;

#[derive(Clone, Debug)]
pub struct Blk {
    pub h: f64,
    pub ext: Ext,
    pub items: Vec<Item>,
    pub edges: Vec<Edge>,
    pub entry: Port,
    pub exit: Option<Port>,
}

fn sh(p: &mut Port, dx: f64, dy: f64) {
    p.dx += dx;
    p.y += dy;
}

impl Blk {
    fn shift(&mut self, dx: f64, dy: f64) {
        self.h += dy;
        for it in &mut self.items {
            it.dx += dx;
            it.y += dy;
        }
        for e in &mut self.edges {
            match e {
                Edge::Flow { from, to, ymid, .. } => {
                    sh(from, dx, dy);
                    sh(to, dx, dy);
                    if let Some(y) = ymid {
                        *y += dy;
                    }
                }
                Edge::Fan { dec, to, bus_y, .. } => {
                    sh(dec, dx, dy);
                    sh(to, dx, dy);
                    if let Some(y) = bus_y {
                        *y += dy;
                    }
                }
                Edge::Poly { pts, .. } | Edge::Bar { pts, .. } => {
                    for p in pts {
                        sh(p, dx, dy);
                    }
                }
            }
        }
        for x in self.ext.iter_mut().flatten() {
            x.0 -= dx;
            x.1 += dx;
        }
        sh(&mut self.entry, dx, dy);
        if let Some(e) = &mut self.exit {
            sh(e, dx, dy);
        }
    }
}

fn ext_add(ext: &mut Ext, lane: usize, l: f64, r: f64) {
    match &mut ext[lane] {
        Some(x) => {
            x.0 = x.0.max(l);
            x.1 = x.1.max(r);
        }
        None => ext[lane] = Some((l, r)),
    }
}

fn ext_union(a: &mut Ext, b: &Ext) {
    for (i, x) in b.iter().enumerate() {
        if let Some((l, r)) = x {
            ext_add(a, i, *l, *r);
        }
    }
}

struct Lay<'a> {
    model: &'a Model,
    nl: usize,
    lane_idx: HashMap<String, usize>,
    cur: usize,
}

pub struct Laid {
    pub root: Blk,
    pub lanes: Vec<String>,
}

impl<'a> Lay<'a> {
    fn id_at(&self, pos: Pos, col0: bool) -> Option<String> {
        self.model.by_pos.get(&(pos.line, if col0 { 0 } else { pos.col })).cloned()
    }

    fn new_ext(&self) -> Ext {
        vec![None; self.nl]
    }

    fn empty(&self, lane: usize) -> Blk {
        let mut ext = self.new_ext();
        ext[lane] = Some((0.0, 0.0));
        let p = Port { lane, dx: 0.0, y: 0.0 };
        Blk { h: 0.0, ext, items: vec![], edges: vec![], entry: p, exit: Some(p) }
    }

    fn leaf(&self, shape: Shape, w: f64, h: f64, lines: Vec<String>, id: Option<String>, terminal: bool) -> Blk {
        let lane = self.cur;
        let mut ext = self.new_ext();
        ext[lane] = Some((w / 2.0, w / 2.0));
        Blk {
            h,
            ext,
            items: vec![Item { shape, lane, dx: 0.0, y: 0.0, w, h, lines, id }],
            edges: vec![],
            entry: Port { lane, dx: 0.0, y: 0.0 },
            exit: if terminal { None } else { Some(Port { lane, dx: 0.0, y: h }) },
        }
    }

    fn action(&self, text: &str, id: Option<String>, shape: Shape) -> Blk {
        let lines = wrap(text, FS, ACTION_MAX);
        let tw = lines.iter().map(|l| text_width(l, FS)).fold(0.0, f64::max);
        let w = (tw + 22.0).max(44.0);
        let h = lines.len() as f64 * LH + 14.0;
        self.leaf(shape, w, h, lines, id, false)
    }

    fn decision_item(&self, text: &str, id: Option<String>) -> Item {
        let lines = wrap(text, FS, DECISION_MAX);
        let tw = lines.iter().map(|l| text_width(l, FS)).fold(0.0, f64::max);
        let w = tw + 2.0 * 22.0;
        let h = (lines.len() as f64 * LH + 16.0).max(32.0);
        Item { shape: Shape::Decision, lane: self.cur, dx: 0.0, y: 0.0, w, h, lines, id }
    }

    fn append(&self, mut a: Blk, mut b: Blk) -> Blk {
        let y = a.h + G;
        b.shift(0.0, y);
        if let Some(ex) = a.exit {
            a.edges.push(Edge::Flow { from: ex, to: b.entry, ymid: None, arrow: true });
        }
        ext_union(&mut a.ext, &b.ext);
        a.items.extend(b.items);
        a.edges.extend(b.edges);
        a.h = a.h.max(b.h);
        a.exit = b.exit;
        a
    }

    fn seq(&mut self, body: &[Stmt]) -> Blk {
        let mut acc: Option<Blk> = None;
        for s in body {
            match s {
                Stmt::Lane { name, .. } => {
                    self.cur = self.lane_idx[name];
                }
                Stmt::Note { .. } => {}
                _ => {
                    let b = self.stmt(s);
                    acc = Some(match acc {
                        None => b,
                        Some(a) => self.append(a, b),
                    });
                }
            }
        }
        match acc {
            Some(a) => a,
            None => self.empty(self.cur),
        }
    }

    fn stmt(&mut self, s: &Stmt) -> Blk {
        match s {
            Stmt::Start(p) => self.leaf(Shape::Start, 16.0, 16.0, vec![], self.id_at(*p, false), false),
            Stmt::Stop(p) => self.leaf(Shape::Stop, 22.0, 22.0, vec![], self.id_at(*p, false), true),
            Stmt::End(p) => self.leaf(Shape::End, 22.0, 22.0, vec![], self.id_at(*p, false), true),
            Stmt::Action { text, pos } => self.action(text, self.id_at(*pos, false), Shape::Action),
            Stmt::If { pos, parts } => self.if_chain(parts, 0, *pos),
            Stmt::Switch { pos, cond, parts } => {
                let dec = self.decision_item(cond, self.id_at(*pos, false));
                let mut branches = vec![];
                for p in parts {
                    let b = self.seq(&p.body);
                    branches.push((p.label.clone(), b));
                }
                let bus = branches.len() >= 3;
                self.branch_group(dec, branches, bus)
            }
            Stmt::While { pos, cond, is_label, end_label, body } => self.while_blk(*pos, cond, is_label, end_label, body),
            Stmt::Repeat { pos, body, backward, cond, is_label, not_label } => self.repeat_blk(*pos, body, backward, cond, is_label, not_label),
            Stmt::Fork { pos, parts } => self.fork_blk(*pos, parts),
            Stmt::Lane { .. } | Stmt::Note { .. } => unreachable!(),
        }
    }

    fn if_chain(&mut self, parts: &[Part], idx: usize, first_pos: Pos) -> Blk {
        let part = &parts[idx];
        let pos = if idx == 0 { first_pos } else { part.pos };
        let dec = self.decision_item(&part.cond, self.id_at(pos, false));
        let dl = dec.lane;
        let then_b = self.seq(&part.body);
        let else_b = match parts.get(idx + 1) {
            Some(n) if n.kind == PartKind::ElseIf => (None, self.if_chain(parts, idx + 1, first_pos)),
            Some(n) => (n.label.clone(), self.seq(&n.body)),
            None => (None, self.empty(dl)),
        };
        let mut else_b = else_b;
        if parts.get(idx + 1).is_none() {
            // an `if` without `else`: the empty branch stays in the lane of the decision
            else_b.1 = self.empty(dl);
        }
        self.branch_group(dec, vec![(part.label.clone(), then_b), else_b], false)
    }

    /// A decision above several branches, merged again below.
    fn branch_group(&mut self, dec: Item, mut branches: Vec<(Option<String>, Blk)>, bus: bool) -> Blk {
        let dl = dec.lane;
        let (dw, dh) = (dec.w, dec.h);
        // labels
        let mut labels: Vec<Vec<String>> = vec![];
        let mut lab_h: f64 = 0.0;
        for (label, b) in branches.iter_mut() {
            let lines = match label {
                Some(l) if !l.is_empty() => wrap(l, LFS, LABEL_MAX),
                _ => vec![],
            };
            if !lines.is_empty() {
                let lw = lines.iter().map(|l| text_width(l, LFS)).fold(0.0, f64::max);
                let lane = b.entry.lane;
                ext_add(&mut b.ext, lane, 0.0, 7.0 + lw);
                lab_h = lab_h.max(lines.len() as f64 * LLH);
            }
            labels.push(lines);
        }
        // offsets: branches side by side, no overlap in a shared lane
        let n = branches.len();
        let mut off = vec![0.0f64; n];
        for j in 1..n {
            let mut o = f64::NEG_INFINITY;
            for i in 0..j {
                for l in 0..self.nl {
                    if let (Some((_, ri)), Some((lj, _))) = (branches[i].1.ext[l], branches[j].1.ext[l]) {
                        o = o.max(off[i] + ri + BRANCH_GAP + lj);
                    }
                }
            }
            off[j] = if o.is_finite() { o } else { 0.0 };
        }
        // the main line: the one non-empty branch when the other is empty, else the middle
        let is_empty = |b: &Blk| b.items.is_empty();
        let center = if n == 2 && is_empty(&branches[1].1) && !is_empty(&branches[0].1) {
            off[0]
        } else if n == 2 && is_empty(&branches[0].1) && !is_empty(&branches[1].1) {
            off[1]
        } else {
            let mn = off.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = off.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (mn + mx) / 2.0
        };
        let top_gap = if bus { 10.0 + 12.0 + lab_h } else { 16.0 + lab_h };
        let row_y = dh + top_gap;
        let mut ext = self.new_ext();
        ext_add(&mut ext, dl, dw / 2.0, dw / 2.0);
        let mut items = vec![dec.clone()];
        let mut edges: Vec<Edge> = vec![];
        let mut bottom = row_y;
        let mut exits: Vec<Port> = vec![];
        let bus_y = if bus { Some(dh + 10.0) } else { None };
        for (i, (_, b)) in branches.iter_mut().enumerate() {
            let empty = is_empty(b);
            b.shift(off[i] - center, row_y);
            ext_union(&mut ext, &b.ext);
            edges.push(Edge::Fan { dec: Port { lane: dl, dx: 0.0, y: 0.0 }, dw, dh, to: b.entry, bus_y, arrow: !empty });
            if !labels[i].is_empty() {
                let nl = labels[i].len() as f64;
                items.push(Item { shape: Shape::Label, lane: b.entry.lane, dx: b.entry.dx + 5.0, y: b.entry.y - 3.0 - nl * LLH, w: 0.0, h: nl * LLH, lines: labels[i].clone(), id: None });
            }
            bottom = bottom.max(b.h);
            if let Some(e) = b.exit {
                exits.push(e);
            }
            items.extend(std::mem::take(&mut b.items));
            edges.extend(std::mem::take(&mut b.edges));
        }
        let (exit, h);
        if exits.len() >= 2 {
            let y_m = bottom + 14.0;
            for e in &exits {
                edges.push(Edge::Flow { from: *e, to: Port { lane: dl, dx: 0.0, y: y_m + 6.0 }, ymid: Some(y_m + 6.0), arrow: false });
            }
            items.push(Item { shape: Shape::Merge, lane: dl, dx: 0.0, y: y_m, w: 12.0, h: 12.0, lines: vec![], id: None });
            ext_add(&mut ext, dl, 6.0, 6.0);
            exit = Some(Port { lane: dl, dx: 0.0, y: y_m + 12.0 });
            h = y_m + 12.0;
        } else {
            exit = exits.first().copied();
            h = bottom;
        }
        Blk { h, ext, items, edges, entry: Port { lane: dl, dx: 0.0, y: 0.0 }, exit }
    }

    fn lane_span(&self, ext: &Ext, extra: usize) -> (usize, usize) {
        let mut lo = extra;
        let mut hi = extra;
        for (i, e) in ext.iter().enumerate() {
            if e.is_some() {
                lo = lo.min(i);
                hi = hi.max(i);
            }
        }
        (lo, hi)
    }

    fn while_blk(&mut self, pos: Pos, cond: &str, is_label: &Option<String>, end_label: &Option<String>, body: &[Stmt]) -> Blk {
        let dec = self.decision_item(cond, self.id_at(pos, false));
        let dl = dec.lane;
        let (dw, dh) = (dec.w, dec.h);
        let mut b = self.seq(body);
        let is_lines = match is_label {
            Some(l) if !l.is_empty() => wrap(l, LFS, LABEL_MAX),
            _ => vec![],
        };
        let lab_h = is_lines.len() as f64 * LLH;
        let mut ext = self.new_ext();
        ext_add(&mut ext, dl, dw / 2.0, dw / 2.0);
        let by = dh + 16.0 + lab_h;
        b.shift(0.0, by);
        ext_union(&mut ext, &b.ext);
        let mut items = vec![dec];
        let mut edges = vec![Edge::Flow { from: Port { lane: dl, dx: 0.0, y: dh }, to: b.entry, ymid: None, arrow: true }];
        if !is_lines.is_empty() {
            let nl = is_lines.len() as f64;
            items.push(Item { shape: Shape::Label, lane: b.entry.lane, dx: b.entry.dx + 5.0, y: b.entry.y - 3.0 - nl * LLH, w: 0.0, h: nl * LLH, lines: is_lines.clone(), id: None });
            let lw = is_lines.iter().map(|l| text_width(l, LFS)).fold(0.0, f64::max);
            ext_add(&mut ext, b.entry.lane, 0.0, 7.0 + lw);
        }
        let (lm, rm) = self.lane_span(&ext, dl);
        let leftx = -(ext[lm].map_or(0.0, |x| x.0) + 18.0);
        let rightx = ext[rm].map_or(0.0, |x| x.1) + 18.0;
        ext_add(&mut ext, lm, -leftx + 8.0, 0.0);
        let end_lines = match end_label {
            Some(l) if !l.is_empty() => wrap(l, LFS, LABEL_MAX),
            _ => vec![],
        };
        let elw = end_lines.iter().map(|l| text_width(l, LFS)).fold(0.0, f64::max);
        ext_add(&mut ext, rm, 0.0, rightx + 8.0 + elw);
        let ymid_dec = dh / 2.0;
        let mut bottom = b.h;
        if let Some(ex) = b.exit {
            let yb = ex.y + 16.0;
            bottom = bottom.max(yb);
            edges.push(Edge::Poly { pts: vec![ex, Port { lane: ex.lane, dx: ex.dx, y: yb }, Port { lane: lm, dx: leftx, y: yb }, Port { lane: lm, dx: leftx, y: ymid_dec }, Port { lane: dl, dx: -dw / 2.0, y: ymid_dec }], arrow: true });
        } else {
            // the body never comes back: no loop edge
        }
        let y_end = bottom + 20.0;
        edges.push(Edge::Poly { pts: vec![Port { lane: dl, dx: dw / 2.0, y: ymid_dec }, Port { lane: rm, dx: rightx, y: ymid_dec }, Port { lane: rm, dx: rightx, y: y_end }, Port { lane: dl, dx: 0.0, y: y_end }], arrow: false });
        if !end_lines.is_empty() {
            let nl = end_lines.len() as f64;
            items.push(Item { shape: Shape::Label, lane: dl, dx: dw / 2.0 + 5.0, y: ymid_dec - 3.0 - nl * LLH, w: 0.0, h: nl * LLH, lines: end_lines, id: None });
        }
        items.extend(std::mem::take(&mut b.items));
        edges.extend(std::mem::take(&mut b.edges));
        Blk { h: y_end, ext, items, edges, entry: Port { lane: dl, dx: 0.0, y: 0.0 }, exit: Some(Port { lane: dl, dx: 0.0, y: y_end }) }
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat_blk(&mut self, pos: Pos, body: &[Stmt], backward: &Option<(String, Pos)>, cond: &str, is_label: &Option<String>, not_label: &Option<String>) -> Blk {
        let mut b = self.seq(body);
        let entry = b.entry;
        let dec = self.decision_item(cond, self.id_at(pos, false));
        let dl = dec.lane;
        let (dw, dh) = (dec.w, dec.h);
        let yd = b.h + G;
        let mut ext = self.new_ext();
        ext_union(&mut ext, &b.ext);
        ext_add(&mut ext, dl, dw / 2.0, dw / 2.0);
        let mut items = std::mem::take(&mut b.items);
        let mut edges = std::mem::take(&mut b.edges);
        if let Some(ex) = b.exit {
            edges.push(Edge::Flow { from: ex, to: Port { lane: dl, dx: 0.0, y: yd }, ymid: None, arrow: true });
        }
        let mut dec_item = dec;
        dec_item.y = yd;
        items.push(dec_item);
        let (_, rm) = self.lane_span(&ext, dl);
        let rightx = ext[rm].map_or(0.0, |x| x.1) + 20.0;
        let ydm = yd + dh / 2.0;
        let mut h = yd + dh;
        let top = -12.0;
        let is_lines = match is_label {
            Some(l) if !l.is_empty() => wrap(l, LFS, LABEL_MAX),
            _ => vec![],
        };
        let elw = is_lines.iter().map(|l| text_width(l, LFS)).fold(0.0, f64::max);
        if let Some((btext, bpos)) = backward {
            let bb = self.action(btext, self.id_at(*bpos, false), Shape::BackAction);
            let (bw, bh) = (bb.items[0].w, bb.items[0].h);
            let cx = rightx + bw / 2.0;
            let ybox = ((ydm + top) / 2.0 - bh / 2.0).max(0.0);
            items.push(Item { shape: Shape::BackAction, lane: rm, dx: cx, y: ybox, w: bw, h: bh, lines: bb.items[0].lines.clone(), id: bb.items[0].id.clone() });
            ext_add(&mut ext, rm, 0.0, cx + bw / 2.0 + 6.0);
            edges.push(Edge::Poly { pts: vec![Port { lane: dl, dx: dw / 2.0, y: ydm }, Port { lane: rm, dx: cx, y: ydm }, Port { lane: rm, dx: cx, y: ybox + bh }], arrow: true });
            edges.push(Edge::Poly { pts: vec![Port { lane: rm, dx: cx, y: ybox }, Port { lane: rm, dx: cx, y: top }, Port { lane: entry.lane, dx: 0.0, y: top }, Port { lane: entry.lane, dx: 0.0, y: 0.0 }], arrow: true });
            h = h.max(ybox + bh);
        } else {
            ext_add(&mut ext, rm, 0.0, rightx + 8.0 + elw);
            edges.push(Edge::Poly { pts: vec![Port { lane: dl, dx: dw / 2.0, y: ydm }, Port { lane: rm, dx: rightx, y: ydm }, Port { lane: rm, dx: rightx, y: top }, Port { lane: entry.lane, dx: 0.0, y: top }, Port { lane: entry.lane, dx: 0.0, y: 0.0 }], arrow: true });
        }
        if !is_lines.is_empty() {
            let nl = is_lines.len() as f64;
            items.push(Item { shape: Shape::Label, lane: dl, dx: dw / 2.0 + 5.0, y: ydm - 3.0 - nl * LLH, w: 0.0, h: nl * LLH, lines: is_lines, id: None });
        }
        let not_lines = match not_label {
            Some(l) if !l.is_empty() => wrap(l, LFS, LABEL_MAX),
            _ => vec![],
        };
        let nl = not_lines.len() as f64;
        let y_exit = yd + dh + 14.0 + nl * LLH;
        if !not_lines.is_empty() {
            let lw = not_lines.iter().map(|l| text_width(l, LFS)).fold(0.0, f64::max);
            items.push(Item { shape: Shape::Label, lane: dl, dx: 5.0, y: yd + dh + 2.0, w: 0.0, h: nl * LLH, lines: not_lines, id: None });
            ext_add(&mut ext, dl, 0.0, 7.0 + lw);
        }
        edges.push(Edge::Poly { pts: vec![Port { lane: dl, dx: 0.0, y: yd + dh }, Port { lane: dl, dx: 0.0, y: y_exit }], arrow: false });
        h = h.max(y_exit);
        Blk { h, ext, items, edges, entry, exit: Some(Port { lane: dl, dx: 0.0, y: y_exit }) }
    }

    fn fork_blk(&mut self, pos: Pos, parts: &[Part]) -> Blk {
        let fl = self.cur;
        let fork_id = self.id_at(pos, false);
        let join_id = self.id_at(pos, true);
        let mut branches: Vec<Blk> = vec![];
        for p in parts {
            branches.push(self.seq(&p.body));
        }
        let n = branches.len();
        let mut off = vec![0.0f64; n];
        for j in 1..n {
            let mut o = f64::NEG_INFINITY;
            for i in 0..j {
                for l in 0..self.nl {
                    if let (Some((_, ri)), Some((lj, _))) = (branches[i].ext[l], branches[j].ext[l]) {
                        o = o.max(off[i] + ri + BRANCH_GAP + lj);
                    }
                }
            }
            off[j] = if o.is_finite() { o } else { 0.0 };
        }
        let mn = off.iter().cloned().fold(f64::INFINITY, f64::min);
        let mx = off.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let center = (mn + mx) / 2.0;
        let bar_h = 4.0;
        let row_y = bar_h + G;
        let mut ext = self.new_ext();
        ext_add(&mut ext, fl, 12.0, 12.0);
        let mut items = vec![];
        let mut edges = vec![];
        let mut top_pts = vec![Port { lane: fl, dx: 0.0, y: bar_h / 2.0 }];
        let mut bottom = row_y;
        let mut exits = vec![];
        for (i, b) in branches.iter_mut().enumerate() {
            b.shift(off[i] - center, row_y);
            ext_union(&mut ext, &b.ext);
            top_pts.push(Port { lane: b.entry.lane, dx: b.entry.dx, y: bar_h / 2.0 });
            edges.push(Edge::Flow { from: Port { lane: b.entry.lane, dx: b.entry.dx, y: bar_h }, to: b.entry, ymid: None, arrow: !b.items.is_empty() });
            bottom = bottom.max(b.h);
            if let Some(e) = b.exit {
                exits.push(e);
            }
            items.extend(std::mem::take(&mut b.items));
            edges.extend(std::mem::take(&mut b.edges));
        }
        edges.push(Edge::Bar { pts: top_pts, id: fork_id });
        let (exit, h);
        if exits.is_empty() {
            exit = None;
            h = bottom;
        } else {
            let yb = bottom + 14.0;
            let mut bot = vec![Port { lane: fl, dx: 0.0, y: yb + bar_h / 2.0 }];
            for e in &exits {
                edges.push(Edge::Flow { from: *e, to: Port { lane: e.lane, dx: e.dx, y: yb }, ymid: None, arrow: false });
                bot.push(Port { lane: e.lane, dx: e.dx, y: yb + bar_h / 2.0 });
            }
            edges.push(Edge::Bar { pts: bot, id: join_id });
            exit = Some(Port { lane: fl, dx: 0.0, y: yb + bar_h });
            h = yb + bar_h;
        }
        Blk { h, ext, items, edges, entry: Port { lane: fl, dx: 0.0, y: 0.0 }, exit }
    }
}

pub fn layout(model: &Model, d: &Diagram) -> Laid {
    let nl = model.lanes.len().max(1);
    let lane_idx: HashMap<String, usize> = model.lanes.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();
    let mut l = Lay { model, nl, lane_idx, cur: 0 };
    let root = l.seq(&d.body);
    Laid { root, lanes: model.lanes.clone() }
}
