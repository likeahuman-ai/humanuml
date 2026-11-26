//! `humanuml tree`: the flow as the text tree of the first Like A Human flows,
//! readable in a terminal, in a diff and on GitHub without a renderer.
//!
//! ```text
//! claim_invite
//! │
//! ├── ● start
//! ├── a · open the link from the mail
//! │       ⤳ src/routes/claim.ts:12
//! ├── ◆ c · signed in?
//! │   ├── no
//! │   │   └── d · sign in or create an account · FL-1.4
//! │   └── (else) → on
//! └── ◉ stop
//! ```

use crate::ast::*;
use crate::font::wrap;
use crate::marks;

const WIDTH: f64 = 420.0; // wrap width, in the 12 px font: about 70 characters

pub fn tree(d: &Diagram) -> String {
    let mut out = format!("{}\n│\n", d.name);
    if d.kind == Kind::State {
        for (i, s) in d.states.iter().enumerate() {
            let last = i + 1 == d.states.len();
            out.push_str(&format!("{}{}\n", if last { "└── " } else { "├── " }, s.title));
            let pre = if last { "    " } else { "│   " };
            for (a, b, _) in d.transitions.iter().filter(|t| t.0 == s.id) {
                let _ = a;
                out.push_str(&format!("{}    → {}\n", pre, b));
            }
        }
        return out;
    }
    seq(&mut out, &d.body, "");
    out
}

fn node(out: &mut String, pre: &str, last: bool, head: &str, text: &str) {
    let m = marks::parse(text);
    let conn = if last { "└── " } else { "├── " };
    let child = format!("{}{}", pre, if last { "    " } else { "│   " });
    let status = if m.status.is_empty() { String::new() } else { format!("  [{}]", m.status.join("] [")) };
    let lines = wrap(&m.text, 12.0, WIDTH);
    for (i, l) in lines.iter().enumerate() {
        if i == 0 {
            out.push_str(&format!("{}{}{}{}{}\n", pre, conn, head, l, if lines.len() == 1 { status.as_str() } else { "" }));
        } else {
            out.push_str(&format!("{}{}{}{}\n", child, " ".repeat(head.chars().count()), l, if i + 1 == lines.len() { status.as_str() } else { "" }));
        }
    }
    for p in &m.pitfalls {
        out.push_str(&format!("{}    ⚠ {}\n", child, p));
    }
    for c in &m.cites {
        out.push_str(&format!("{}    ⤳ {}{}\n", child, c.path, c.line.as_ref().map(|l| format!(":{}", l)).unwrap_or_default()));
    }
    for e in &m.events {
        out.push_str(&format!("{}    event: {}\n", child, e));
    }
}

fn simple(out: &mut String, pre: &str, last: bool, s: &str) {
    out.push_str(&format!("{}{}{}\n", pre, if last { "└── " } else { "├── " }, s));
}

fn child_pre(pre: &str, last: bool) -> String {
    format!("{}{}", pre, if last { "    " } else { "│   " })
}

/// A labelled branch with its body under it.
fn branch(out: &mut String, pre: &str, last: bool, label: &str, body: &[Stmt]) {
    simple(out, pre, last, label);
    let cp = child_pre(pre, last);
    if body.iter().all(|s| matches!(s, Stmt::Lane { .. })) {
        out.push_str(&format!("{}└── → on\n", cp));
    } else {
        seq(out, body, &cp);
    }
}

fn seq(out: &mut String, body: &[Stmt], pre: &str) {
    let n = body.len();
    for (i, s) in body.iter().enumerate() {
        let last = i + 1 == n;
        let cp = child_pre(pre, last);
        match s {
            Stmt::Lane { name, .. } => simple(out, pre, last, &format!("|{}|", name)),
            Stmt::Start(_) => simple(out, pre, last, "● start"),
            Stmt::Stop(_) => simple(out, pre, last, "◉ stop"),
            Stmt::End(_) => simple(out, pre, last, "⊗ end"),
            Stmt::Action { text, .. } => node(out, pre, last, "", text),
            Stmt::Note { side, text, .. } => simple(out, pre, last, &format!("note {}: {}", side, text.replace('\n', " "))),
            Stmt::If { parts, .. } => {
                // the first condition heads the block; elseif and else are branches
                node(out, pre, last, "◆ ", &parts[0].cond);
                let has_else = parts.iter().any(|p| p.kind == PartKind::Else);
                let k = parts.len() + if has_else { 0 } else { 1 };
                for (j, p) in parts.iter().enumerate() {
                    let bl = j + 1 == k;
                    let label = match p.kind {
                        PartKind::Then => p.label.clone().unwrap_or_else(|| "then".into()),
                        PartKind::ElseIf => format!("else if ◆ {} → {}", marks::parse(&p.cond).text, p.label.clone().unwrap_or_else(|| "then".into())),
                        _ => p.label.clone().unwrap_or_else(|| "else".into()),
                    };
                    branch(out, &cp, bl, &label, &p.body);
                }
                if !has_else {
                    simple(out, &cp, true, "(else) → on");
                }
            }
            Stmt::Switch { cond, parts, .. } => {
                node(out, pre, last, "◆ ", cond);
                for (j, p) in parts.iter().enumerate() {
                    branch(out, &cp, j + 1 == parts.len(), p.label.as_deref().unwrap_or("case"), &p.body);
                }
            }
            Stmt::While { cond, is_label, end_label, body, .. } => {
                node(out, pre, last, "↻ while ", cond);
                branch(out, &cp, false, is_label.as_deref().unwrap_or("loop"), body);
                simple(out, &cp, true, &format!("{} → on", end_label.as_deref().unwrap_or("done")));
            }
            Stmt::Repeat { body, backward, cond, is_label, not_label, .. } => {
                simple(out, pre, last, "↻ repeat");
                seq(out, body, &cp);
                let tail = format!("◆ {} → again: {} · on: {}", marks::parse(cond).text, is_label.as_deref().unwrap_or("yes"), not_label.as_deref().unwrap_or("no"));
                out.push_str(&format!("{}{}\n", cp, tail));
                if let Some((b, _)) = backward {
                    out.push_str(&format!("{}    ↺ back: {}\n", cp, marks::parse(b).text));
                }
            }
            Stmt::Fork { parts, .. } => {
                simple(out, pre, last, "⑂ fork, all at once");
                for (j, p) in parts.iter().enumerate() {
                    branch(out, &cp, j + 1 == parts.len(), &format!("branch {}", j + 1), &p.body);
                }
            }
        }
    }
}
