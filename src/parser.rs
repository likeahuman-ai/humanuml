//! Line parser for the PlantUML activity subset.
//!
//! Every error has a line, a column, a message and, where one is known,
//! a hint ("did you mean ...").

use crate::ast::*;

const KEYWORDS: &[&str] = &[
    "start", "stop", "end", "if", "elseif", "else", "endif", "switch", "case", "endswitch", "while",
    "endwhile", "repeat", "backward", "fork", "split", "detach", "kill", "note",
];

/// Words that people write for a keyword of ours.
const ALIASES: &[(&str, &str)] = &[
    ("elif", "elseif"),
    ("elsif", "elseif"),
    ("elsei", "elseif"),
    ("endswitch", "endswitch"),
    ("endcase", "endswitch"),
    ("default", "else"),
    ("otherwise", "else"),
    ("until", "repeat while (condition)"),
    ("endrepeat", "repeat while (condition)"),
    ("endfor", "endwhile"),
    ("fi", "endif"),
    ("begin", "start"),
    ("finish", "stop"),
    ("loop", "repeat"),
    ("do", "repeat"),
    ("when", "case (label)"),
    ("then", "if (condition) then (label)"),
];

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur.push((prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The keyword nearest to `word`, when it is close enough to be a typo.
pub fn did_you_mean(word: &str) -> Option<String> {
    suggest(word).map(|k| format!("did you mean `{}`?", k))
}

/// The keyword (with its shape) that `word` was most likely meant to be.
pub fn suggest(word: &str) -> Option<String> {
    let w = word.to_lowercase();
    for (a, k) in ALIASES {
        if *a == w && *a != *k {
            return Some((*k).to_string());
        }
    }
    let mut best: Option<(usize, &str)> = None;
    for k in KEYWORDS {
        let d = levenshtein(&w, k);
        let limit = if k.len() <= 3 { 1 } else { 2 };
        if d > 0 && d <= limit && best.map_or(true, |(bd, _)| d < bd) {
            best = Some((d, k));
        }
    }
    best.map(|(_, k)| match k {
        "if" => "if (condition) then (label)".to_string(),
        "elseif" => "elseif (condition) then (label)".to_string(),
        "switch" => "switch (question)".to_string(),
        "case" => "case (label)".to_string(),
        "while" => "while (condition)".to_string(),
        other => other.to_string(),
    })
}

#[derive(Debug)]
enum Tok {
    Start,
    Stop,
    End,
    Lane(String),
    Action(String),
    Backward(String),
    If { cond: String, label: Option<String> },
    ElseIf { cond: String, label: Option<String> },
    Else { label: Option<String> },
    EndIf,
    Switch { cond: String },
    Case { label: String },
    EndSwitch,
    While { cond: String, is: Option<String> },
    EndWhile { label: Option<String> },
    Repeat,
    RepeatWhile { cond: String, is: Option<String>, not: Option<String> },
    Fork,
    ForkAgain,
    EndFork,
    Note { side: String, text: String },
    Ignore,
}

struct LineErr {
    /// Byte offset inside the trimmed line.
    at: usize,
    msg: String,
    hint: Option<String>,
    /// The line as the writer most likely meant it; the parser goes on with it.
    fix: Option<String>,
}

fn le(at: usize, msg: impl Into<String>, hint: Option<String>) -> LineErr {
    LineErr { at, msg: msg.into(), hint, fix: None }
}

fn le_fix(at: usize, msg: impl Into<String>, hint: Option<String>, fix: String) -> LineErr {
    LineErr { at, msg: msg.into(), hint, fix: Some(fix) }
}

/// `s[from]` is `(`. Returns the text inside, balanced, and the offset after `)`.
fn paren(s: &str, from: usize) -> Option<(String, usize)> {
    let b = s.as_bytes();
    if b.get(from) != Some(&b'(') {
        return None;
    }
    let mut depth = 0;
    for (i, c) in s[from..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((s[from + 1..from + i].trim().to_string(), from + i + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn skip_ws(s: &str, mut i: usize) -> usize {
    while i < s.len() && (s.as_bytes()[i] == b' ' || s.as_bytes()[i] == b'\t') {
        i += 1;
    }
    i
}

/// `(text)` at `i` (after spaces). Errors name `what`.
fn need_paren(s: &str, i: usize, what: &str) -> Result<(String, usize), LineErr> {
    let i = skip_ws(s, i);
    if s.as_bytes().get(i) != Some(&b'(') {
        return Err(le(i.min(s.len()), format!("expected `(` after `{}`", what), Some(format!("write it as `{} (text)`", what))));
    }
    match paren(s, i) {
        Some(r) => Ok(r),
        None => Err(le(i, format!("`(` of `{}` is never closed", what), Some("add the closing `)` at the end of the text".into()))),
    }
}

/// After `word` at `i`: an optional `(label)`, then end of line.
fn opt_label_end(s: &str, i: usize, what: &str) -> Result<Option<String>, LineErr> {
    let i = skip_ws(s, i);
    if i >= s.len() {
        return Ok(None);
    }
    if s.as_bytes()[i] == b'(' {
        match paren(s, i) {
            Some((l, e)) => {
                let e = skip_ws(s, e);
                if e < s.len() {
                    return Err(le(e, format!("unexpected text after `{}` label", what), Some("one statement per line".into())));
                }
                Ok(Some(l))
            }
            None => Err(le(i, format!("`(` of the `{}` label is never closed", what), Some("add the closing `)`".into()))),
        }
    } else {
        Err(le(i, format!("expected `(label)` or end of line after `{}`", what), Some(format!("put the label in brackets: `{} (label)`", what))))
    }
}

/// Word at the start of `s`: letters only.
fn first_word(s: &str) -> &str {
    let end = s.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(s.len());
    &s[..end]
}

fn parse_line(t: &str) -> Result<Tok, LineErr> {
    if t.is_empty() || t.starts_with('\'') || t.starts_with("/'") {
        return Ok(Tok::Ignore);
    }
    if t.starts_with('|') {
        // |lane|  or  |#color|lane|
        let inner = t.trim_end();
        if inner.len() >= 3 && inner.ends_with('|') {
            let body = &inner[1..inner.len() - 1];
            let name = match body.rfind('|') {
                Some(p) if body.starts_with('#') => &body[p + 1..],
                _ => body,
            };
            if name.trim().is_empty() {
                return Err(le(0, "swimlane has no name", Some("write `|lane name|`".into())));
            }
            return Ok(Tok::Lane(name.trim().to_string()));
        }
        return Err(le(t.len(), "swimlane is not closed", Some("end the line with `|`, as in `|trainer|`".into())));
    }
    let w = first_word(t);
    let lw = w.to_lowercase();
    let rest = &t[w.len()..];
    let exact = |k: &str| lw == k;
    match lw.as_str() {
        "start" if exact("start") => {
            if rest.trim().is_empty() { Ok(Tok::Start) } else { Err(le(5, "unexpected text after `start`", None)) }
        }
        "stop" => {
            if rest.trim().is_empty() { Ok(Tok::Stop) } else { Err(le(4, "unexpected text after `stop`", None)) }
        }
        "end" => {
            let r = rest.trim();
            if r.is_empty() {
                Ok(Tok::End)
            } else if r == "fork" || r == "merge" {
                if r == "fork" { Ok(Tok::EndFork) } else { Err(le(0, "`end merge` is not supported", None)) }
            } else if r == "if" {
                Err(le_fix(0, "`end if` is written as one word", Some("did you mean `endif`?".into()), "endif".into()))
            } else if r == "while" {
                Err(le_fix(0, "`end while` is written as one word", Some("did you mean `endwhile`?".into()), "endwhile".into()))
            } else if r == "switch" {
                Err(le_fix(0, "`end switch` is written as one word", Some("did you mean `endswitch`?".into()), "endswitch".into()))
            } else {
                Err(le(3, format!("unexpected `{}` after `end`", r), Some("`end` alone stops the flow; `end fork` closes a fork".into())))
            }
        }
        "endif" => {
            if rest.trim().is_empty() { Ok(Tok::EndIf) } else { Err(le(5, "unexpected text after `endif`", None)) }
        }
        "endswitch" => {
            if rest.trim().is_empty() { Ok(Tok::EndSwitch) } else { Err(le(9, "unexpected text after `endswitch`", None)) }
        }
        "endwhile" => Ok(Tok::EndWhile { label: opt_label_end(t, 8, "endwhile")? }),
        "if" => {
            let (cond, mut i) = need_paren(t, 2, "if")?;
            i = skip_ws(t, i);
            if !t[i..].starts_with("then") {
                return Err(le_fix(i.min(t.len()), "expected `then` after the condition of `if`", Some(format!("write `if ({}) then (yes)`", short(&cond))), format!("{}then {}", &t[..i], &t[i..])));
            }
            let label = opt_label_end(t, i + 4, "then")?;
            Ok(Tok::If { cond, label })
        }
        "elseif" => {
            let (cond, mut i) = need_paren(t, 6, "elseif")?;
            i = skip_ws(t, i);
            if !t[i..].starts_with("then") {
                return Err(le_fix(i.min(t.len()), "expected `then` after the condition of `elseif`", Some(format!("write `elseif ({}) then (yes)`", short(&cond))), format!("{}then {}", &t[..i], &t[i..])));
            }
            let label = opt_label_end(t, i + 4, "then")?;
            Ok(Tok::ElseIf { cond, label })
        }
        "else" => Ok(Tok::Else { label: opt_label_end(t, 4, "else")? }),
        "switch" => {
            let (cond, i) = need_paren(t, 6, "switch")?;
            if skip_ws(t, i) < t.len() {
                return Err(le(skip_ws(t, i), "unexpected text after the `switch` question", None));
            }
            Ok(Tok::Switch { cond })
        }
        "case" => {
            let (label, i) = need_paren(t, 4, "case")?;
            if skip_ws(t, i) < t.len() {
                return Err(le(skip_ws(t, i), "unexpected text after the `case` label", None));
            }
            Ok(Tok::Case { label })
        }
        "while" => {
            let (cond, mut i) = need_paren(t, 5, "while")?;
            i = skip_ws(t, i);
            if i >= t.len() {
                return Ok(Tok::While { cond, is: None });
            }
            if t[i..].starts_with("is") {
                let is = opt_label_end(t, i + 2, "is")?;
                return Ok(Tok::While { cond, is });
            }
            Err(le(i, "expected `is (label)` or end of line after the `while` condition", None))
        }
        "repeat" => {
            let r = rest.trim_start();
            if r.is_empty() {
                return Ok(Tok::Repeat);
            }
            if let Some(after) = r.strip_prefix("while") {
                let base = t.len() - after.len();
                let (cond, mut i) = need_paren(t, base, "repeat while")?;
                let mut is = None;
                let mut not = None;
                loop {
                    i = skip_ws(t, i);
                    if i >= t.len() {
                        break;
                    }
                    if t[i..].starts_with("is") && is.is_none() {
                        let (l, e) = need_paren(t, i + 2, "is")?;
                        is = Some(l);
                        i = e;
                    } else if t[i..].starts_with("not") && not.is_none() {
                        let (l, e) = need_paren(t, i + 3, "not")?;
                        not = Some(l);
                        i = e;
                    } else {
                        return Err(le(i, "expected `is (label)`, `not (label)` or end of line", Some("example: `repeat while (done?) is (no) not (yes)`".into())));
                    }
                }
                return Ok(Tok::RepeatWhile { cond, is, not });
            }
            let w2 = first_word(r);
            if levenshtein(&w2.to_lowercase(), "while") <= 2 && !w2.is_empty() {
                return Err(le_fix(t.len() - r.len(), format!("unexpected `{}` after `repeat`", w2), Some("did you mean `repeat while (condition)`?".into()), format!("repeat while{}", &r[w2.len()..])));
            }
            Err(le(t.len() - r.len(), format!("unexpected `{}` after `repeat`", w2), Some("`repeat` stands alone; the loop ends with `repeat while (condition)`".into())))
        }
        "fork" => {
            let r = rest.trim();
            if r.is_empty() {
                Ok(Tok::Fork)
            } else if r == "again" {
                Ok(Tok::ForkAgain)
            } else {
                if levenshtein(&r.to_lowercase(), "again") <= 2 {
                    Err(le_fix(5, format!("unexpected `{}` after `fork`", r), Some("did you mean `fork again`?".into()), "fork again".into()))
                } else {
                    Err(le(5, format!("unexpected `{}` after `fork`", r), Some("`fork again` starts the next branch; `end fork` closes the block".into())))
                }
            }
        }
        "backward" => {
            let r = rest.trim_start();
            if r.starts_with(':') {
                // text is handled by the driver (it can span lines)
                Ok(Tok::Backward(r.to_string()))
            } else {
                Err(le(8, "expected `:text;` after `backward`", Some("backward :text;".into())))
            }
        }
        "note" => {
            let r = rest.trim_start();
            let side = first_word(r).to_lowercase();
            if side != "left" && side != "right" {
                return Err(le(4, "expected `left` or `right` after `note`", Some("note right: text".into())));
            }
            let after = r[side.len()..].trim_start();
            if let Some(txt) = after.strip_prefix(':') {
                Ok(Tok::Note { side, text: txt.trim().to_string() })
            } else if after.is_empty() {
                Ok(Tok::Note { side, text: String::new() })
            } else {
                Err(le(t.len() - after.len(), "expected `:` after the note side", None))
            }
        }
        "split" | "detach" | "kill" => Err(le(0, format!("`{}` is valid PlantUML but is not in the humanuml subset", lw), None)),
        _ => {
            if t.starts_with(':') {
                return Ok(Tok::Action(t.to_string()));
            }
            // not a statement
            let hint = if w.is_empty() { None } else { did_you_mean(w) };
            let msg = if w.is_empty() {
                format!("expected a statement, found `{}`", short(t))
            } else {
                format!("`{}` is not a statement", w)
            };
            let hint = hint.or_else(|| {
                if w.is_empty() { Some("an action is written `:text;`".into()) } else { Some("an action starts with `:` and ends with `;`".into()) }
            });
            // a fix when the suggestion is one word, or a word with the same tail
            let fix = suggest(w).and_then(|k| {
                let kw = first_word(&k).to_string();
                let single = matches!(kw.as_str(), "if" | "elseif" | "else" | "endif" | "endwhile" | "endswitch" | "switch" | "case" | "while" | "start" | "stop" | "end" | "fork" | "backward" | "repeat");
                if single && (k == kw || k.contains('(')) && !k.contains("repeat while") {
                    Some(format!("{}{}", kw, rest))
                } else {
                    None
                }
            });
            Err(LineErr { at: 0, msg, hint, fix })
        }
    }
}

fn short(s: &str) -> String {
    let s: String = s.chars().take(24).collect();
    s
}

// ── frames ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FK {
    Root,
    If,
    Switch,
    While,
    Repeat,
    Fork,
}

struct Frame {
    kind: FK,
    pos: Pos,
    cond: String,
    is_label: Option<String>,
    parts: Vec<Part>,
    cur: Vec<Stmt>,
    backward: Option<(String, Pos)>,
    seen_else: bool,
    cur_part: Option<(PartKind, Pos, String, Option<String>)>,
}

impl Frame {
    fn new(kind: FK, pos: Pos) -> Frame {
        Frame { kind, pos, cond: String::new(), is_label: None, parts: vec![], cur: vec![], backward: None, seen_else: false, cur_part: None }
    }
    fn finish_part(&mut self) {
        if let Some((kind, pos, cond, label)) = self.cur_part.take() {
            let body = std::mem::take(&mut self.cur);
            self.parts.push(Part { kind, pos, cond, label, body });
        }
    }
}

fn opener_name(k: FK) -> &'static str {
    match k {
        FK::Root => "diagram",
        FK::If => "if",
        FK::Switch => "switch",
        FK::While => "while",
        FK::Repeat => "repeat",
        FK::Fork => "fork",
    }
}

fn opener_for(k: FK) -> &'static str {
    match k {
        FK::Root => "@startuml",
        FK::If => "if (condition) then (label)",
        FK::Switch => "switch (question)",
        FK::While => "while (condition)",
        FK::Repeat => "repeat",
        FK::Fork => "fork",
    }
}

fn closer_for(k: FK) -> &'static str {
    match k {
        FK::Root => "@enduml",
        FK::If => "endif",
        FK::Switch => "endswitch",
        FK::While => "endwhile",
        FK::Repeat => "repeat while (condition)",
        FK::Fork => "end fork",
    }
}

pub struct Source<'a> {
    pub text: &'a str,
    /// Added to every reported line (for fences inside a markdown file).
    pub line_offset: usize,
}

pub struct Parsed {
    pub diagram: Option<Diagram>,
    pub errors: Vec<Diagnostic>,
}

/// Parse the first `@startuml ... @enduml` block of `src`.
pub fn parse(src: &Source) -> Parsed {
    let lines: Vec<&str> = src.text.lines().collect();
    let mut errors: Vec<Diagnostic> = vec![];
    let off = src.line_offset;
    let mk_err = |errors: &mut Vec<Diagnostic>, line_idx: usize, col: usize, msg: String, hint: Option<String>| {
        errors.push(Diagnostic {
            line: line_idx + 1 + off,
            col,
            message: msg,
            hint,
            source_line: lines.get(line_idx).map(|s| s.to_string()).unwrap_or_default(),
        });
    };

    // find @startuml
    let mut i = 0;
    let mut name = String::new();
    let mut found = false;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("@startuml") {
            name = t["@startuml".len()..].trim().to_string();
            found = true;
            i += 1;
            break;
        }
        if t.is_empty() {
            i += 1;
            continue;
        }
        let hint = if t.starts_with('@') && levenshtein(first_token(t), "@startuml") <= 3 { Some("did you mean `@startuml`?".to_string()) } else { Some("a diagram begins with the line `@startuml`".into()) };
        mk_err(&mut errors, i, indent_of(lines[i]) + 1, "expected `@startuml` at the start of the diagram".into(), hint);
        return Parsed { diagram: None, errors };
    }
    if !found {
        errors.push(Diagnostic { line: 1 + off, col: 1, message: "the file has no `@startuml`".into(), hint: Some("start the diagram with `@startuml`".into()), source_line: String::new() });
        return Parsed { diagram: None, errors };
    }

    // state mode?
    let mut is_state = false;
    for l in &lines[i..] {
        let t = l.trim();
        if t.starts_with("@enduml") {
            break;
        }
        if t.starts_with("state ") || t.contains(" --> ") || t.starts_with("hide empty") {
            is_state = true;
            break;
        }
        if t == "start" || t.starts_with(':') || t.starts_with("if ") {
            break;
        }
    }
    if is_state {
        return parse_state(src, &lines, i, name, errors);
    }

    let mut stack: Vec<Frame> = vec![Frame::new(FK::Root, Pos { line: i + off, col: 1 })];
    let mut ended = false;

    macro_rules! err {
        ($li:expr, $col:expr, $msg:expr, $hint:expr) => {
            mk_err(&mut errors, $li, $col, $msg, $hint)
        };
    }

    while i < lines.len() {
        let raw = lines[i];
        let ind = indent_of(raw);
        let t = raw.trim();
        let line_idx = i;
        let pos = Pos { line: i + 1 + off, col: ind + 1 };
        i += 1;
        if t.starts_with("@enduml") {
            ended = true;
            break;
        }
        // statements are skipped silently
        let col_of = |at: usize| -> usize { ind + t[..at.min(t.len())].chars().count() + 1 };
        let tok = match parse_line(t) {
            Ok(t) => t,
            Err(e) => {
                let fixed = e.fix.as_deref().and_then(|f| parse_line(f).ok());
                err!(line_idx, col_of(e.at), e.msg.clone(), e.hint.clone());
                if let Some(tok) = fixed {
                    tok
                } else {
                // a bad head still opens a block: no cascade of errors
                let fw = first_word(t).to_lowercase();
                let opener = match fw.as_str() {
                    "if" => Some(FK::If),
                    "switch" => Some(FK::Switch),
                    "while" => Some(FK::While),
                    "fork" => Some(FK::Fork),
                    _ => None,
                };
                if let Some(k) = opener {
                    let mut f = Frame::new(k, pos);
                    if k == FK::Switch {
                        f.cur_part = Some((PartKind::Case, pos, String::new(), None));
                    }
                    if k == FK::If || k == FK::Fork {
                        f.cur_part = Some((PartKind::Then, pos, String::new(), None));
                    }
                    stack.push(f);
                } else if fw == "repeat" && stack.last().map_or(false, |f| f.kind == FK::Repeat) {
                    let f = stack.pop().unwrap();
                    fold(&mut stack, f, pos, None);
                }
                continue;
                }
            }
        };

        match tok {
            Tok::Ignore => {}
            Tok::Action(first) | Tok::Backward(first) => {
                let is_backward = t.starts_with("backward");
                // text may span lines until one ends with `;`
                let mut text = first.clone();
                let start_idx = line_idx;
                let mut ok = text.trim_end().ends_with(';') && text.trim().len() > 1;
                while !ok {
                    if i >= lines.len() || lines[i].trim().starts_with("@enduml") {
                        break;
                    }
                    let nxt = lines[i].trim();
                    // a line that looks like a statement ends the guess
                    if looks_like_statement(nxt) {
                        break;
                    }
                    text.push('\n');
                    text.push_str(nxt);
                    i += 1;
                    if text.trim_end().ends_with(';') {
                        ok = true;
                    }
                }
                if !ok {
                    let mut hint = Some("add `;` at the end of the action text".to_string());
                    if i < lines.len() && !lines[i].trim().starts_with("@enduml") {
                        hint = Some(format!("add `;` at the end of the action; line {} looks like the next statement", i + 1 + off));
                    }
                    err!(start_idx, col_of(if is_backward { 9 } else { 0 }), "the action text is never closed: no `;` before the next statement".to_string(), hint);
                    continue;
                }
                let body = text.trim_start_matches(':').trim_end().trim_end_matches(';').trim().to_string();
                if body.is_empty() {
                    err!(line_idx, col_of(0), "the action has no text".to_string(), Some("write `:what happens;`".into()));
                    continue;
                }
                if is_backward {
                    let top = stack.last_mut().unwrap();
                    if top.kind != FK::Repeat {
                        err!(line_idx, ind + 1, "`backward` is only valid directly inside `repeat`".to_string(), Some("place it right after `repeat`".into()));
                    } else if top.backward.is_some() {
                        err!(line_idx, ind + 1, "a `repeat` can have one `backward` action".to_string(), None);
                    } else {
                        top.backward = Some((body, pos));
                    }
                } else {
                    push_stmt(&mut stack, &mut errors, &lines, off, line_idx, ind, Stmt::Action { text: body, pos });
                }
            }
            Tok::Start => push_stmt(&mut stack, &mut errors, &lines, off, line_idx, ind, Stmt::Start(pos)),
            Tok::Stop => push_stmt(&mut stack, &mut errors, &lines, off, line_idx, ind, Stmt::Stop(pos)),
            Tok::End => push_stmt(&mut stack, &mut errors, &lines, off, line_idx, ind, Stmt::End(pos)),
            Tok::Lane(n) => push_stmt(&mut stack, &mut errors, &lines, off, line_idx, ind, Stmt::Lane { name: n, pos }),
            Tok::Note { side, text } => {
                let mut text = text;
                if text.is_empty() {
                    // multi-line: until `end note`
                    let mut closed = false;
                    let mut parts = vec![];
                    while i < lines.len() {
                        let l = lines[i].trim();
                        i += 1;
                        if l == "end note" || l == "endnote" {
                            closed = true;
                            break;
                        }
                        parts.push(l.to_string());
                    }
                    if !closed {
                        err!(line_idx, ind + 1, "`note` is never closed".to_string(), Some("end the note with `end note`".into()));
                        continue;
                    }
                    text = parts.join("\n");
                }
                push_stmt(&mut stack, &mut errors, &lines, off, line_idx, ind, Stmt::Note { side, text, pos });
            }
            Tok::If { cond, label } => {
                let mut f = Frame::new(FK::If, pos);
                f.cur_part = Some((PartKind::Then, pos, cond, label));
                stack.push(f);
            }
            Tok::ElseIf { cond, label } => {
                let top = stack.last_mut().unwrap();
                if top.kind != FK::If {
                    err_misplaced(&mut errors, &lines, off, line_idx, ind, "elseif", &stack);
                } else if top.seen_else {
                    err!(line_idx, ind + 1, "`elseif` after `else`".to_string(), Some("put `elseif` before `else`".into()));
                } else {
                    top.finish_part();
                    top.cur_part = Some((PartKind::ElseIf, pos, cond, label));
                }
            }
            Tok::Else { label } => {
                let top = stack.last_mut().unwrap();
                if top.kind != FK::If {
                    err_misplaced(&mut errors, &lines, off, line_idx, ind, "else", &stack);
                } else if top.seen_else {
                    err!(line_idx, ind + 1, format!("second `else` in the `if` of line {}", top.pos.line), Some("an `if` has one `else`; use `elseif (condition) then` for more".into()));
                } else {
                    top.finish_part();
                    top.seen_else = true;
                    top.cur_part = Some((PartKind::Else, pos, String::new(), label));
                }
            }
            Tok::EndIf => close(&mut stack, &mut errors, &lines, off, line_idx, ind, FK::If, pos, None),
            Tok::Switch { cond } => {
                let mut f = Frame::new(FK::Switch, pos);
                f.cond = cond;
                stack.push(f);
            }
            Tok::Case { label } => {
                let top = stack.last_mut().unwrap();
                if top.kind != FK::Switch {
                    err_misplaced(&mut errors, &lines, off, line_idx, ind, "case", &stack);
                } else {
                    top.finish_part();
                    top.cur_part = Some((PartKind::Case, pos, String::new(), Some(label)));
                }
            }
            Tok::EndSwitch => close(&mut stack, &mut errors, &lines, off, line_idx, ind, FK::Switch, pos, None),
            Tok::While { cond, is } => {
                let mut f = Frame::new(FK::While, pos);
                f.cond = cond;
                f.is_label = is;
                stack.push(f);
            }
            Tok::EndWhile { label } => close(&mut stack, &mut errors, &lines, off, line_idx, ind, FK::While, pos, Some(label)),
            Tok::Repeat => stack.push(Frame::new(FK::Repeat, pos)),
            Tok::RepeatWhile { cond, is, not } => {
                close_repeat(&mut stack, &mut errors, &lines, off, line_idx, ind, pos, cond, is, not);
            }
            Tok::Fork => {
                let mut f = Frame::new(FK::Fork, pos);
                f.cur_part = Some((PartKind::Branch, pos, String::new(), None));
                stack.push(f);
            }
            Tok::ForkAgain => {
                let top = stack.last_mut().unwrap();
                if top.kind != FK::Fork {
                    err_misplaced(&mut errors, &lines, off, line_idx, ind, "fork again", &stack);
                } else {
                    top.finish_part();
                    top.cur_part = Some((PartKind::Branch, pos, String::new(), None));
                }
            }
            Tok::EndFork => close(&mut stack, &mut errors, &lines, off, line_idx, ind, FK::Fork, pos, None),
        }
    }

    if !ended {
        errors.push(Diagnostic {
            line: lines.len() + off,
            col: 1,
            message: "the diagram is never closed: `@enduml` is missing".into(),
            hint: Some("end the diagram with `@enduml`".into()),
            source_line: lines.last().map(|s| s.to_string()).unwrap_or_default(),
        });
    }
    // unclosed blocks
    while stack.len() > 1 {
        let f = stack.pop().unwrap();
        let at = f.pos.line - 1 - off;
        errors.push(Diagnostic {
            line: f.pos.line,
            col: f.pos.col,
            message: format!("`{}` opened here is never closed", opener_name(f.kind)),
            hint: Some(format!("add `{}` before the end of the diagram", closer_for(f.kind))),
            source_line: lines.get(at).map(|s| s.to_string()).unwrap_or_default(),
        });
    }
    let root = stack.pop().unwrap();
    errors.sort_by_key(|d| (d.line, d.col));
    let diagram = if errors.is_empty() {
        Some(Diagram { name, kind: Kind::Activity, body: root.cur, states: vec![], transitions: vec![] })
    } else {
        None
    };
    Parsed { diagram, errors }
}

fn first_token(t: &str) -> &str {
    t.split_whitespace().next().unwrap_or("")
}

fn indent_of(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// Does a line begin like a statement keyword (used to stop a runaway action)?
fn looks_like_statement(t: &str) -> bool {
    if t.starts_with(':') || t.starts_with('|') {
        return true;
    }
    let w = first_word(t);
    let lw = w.to_lowercase();
    matches!(lw.as_str(), "start" | "stop" | "end" | "endif" | "endswitch" | "endwhile" | "else" | "elseif" | "if" | "switch" | "case" | "while" | "repeat" | "fork" | "backward")
        && (t.len() == w.len() || t[w.len()..].starts_with(' ') || t[w.len()..].starts_with('('))
}

fn push_stmt(stack: &mut Vec<Frame>, errors: &mut Vec<Diagnostic>, lines: &[&str], off: usize, line_idx: usize, ind: usize, s: Stmt) {
    let top = stack.last_mut().unwrap();
    if top.kind == FK::Switch && top.cur_part.is_none() {
        errors.push(Diagnostic {
            line: line_idx + 1 + off,
            col: ind + 1,
            message: "a statement inside `switch` must follow a `case (label)`".into(),
            hint: Some("put `case (label)` before it".into()),
            source_line: lines[line_idx].to_string(),
        });
        return;
    }
    top.cur.push(s);
}

fn err_misplaced(errors: &mut Vec<Diagnostic>, lines: &[&str], off: usize, line_idx: usize, ind: usize, word: &str, stack: &[Frame]) {
    let top = stack.last().unwrap();
    let (msg, hint) = match word {
        "case" => (
            "`case` is only valid inside `switch`".to_string(),
            Some(if top.kind == FK::Root { "open the block with `switch (question)` first".to_string() } else { format!("the open block is `{}` from line {}", opener_name(top.kind), top.pos.line) }),
        ),
        "else" | "elseif" => (
            format!("`{}` has no `if` to belong to", word),
            Some(if top.kind == FK::Root { "open the block with `if (condition) then (label)` first".to_string() } else { format!("the open block is `{}` from line {}; close it first", opener_name(top.kind), top.pos.line) }),
        ),
        _ => (
            format!("`{}` is only valid inside `fork`", word),
            Some("open the block with `fork` first".to_string()),
        ),
    };
    errors.push(Diagnostic { line: line_idx + 1 + off, col: ind + 1, message: msg, hint, source_line: lines[line_idx].to_string() });
}

#[allow(clippy::too_many_arguments)]
fn close(stack: &mut Vec<Frame>, errors: &mut Vec<Diagnostic>, lines: &[&str], off: usize, line_idx: usize, ind: usize, want: FK, pos: Pos, end_label: Option<Option<String>>) {
    let top_kind = stack.last().unwrap().kind;
    let word = closer_for(want);
    if top_kind != want {
        // is there a matching frame lower in the stack?
        let lower = stack.iter().rposition(|f| f.kind == want);
        let top = stack.last().unwrap();
        let (msg, hint) = match lower {
            Some(_) if top_kind != FK::Root => (
                format!("found `{}` but the open `{}` from line {} must close first", word, opener_name(top_kind), top.pos.line),
                Some(format!("add `{}` first", closer_for(top_kind))),
            ),
            None if top_kind != FK::Root => (
                format!("expected `{}` to close the `{}` from line {}, found `{}`", closer_for(top_kind), opener_name(top_kind), top.pos.line, word),
                Some(format!("did you mean `{}`?", closer_for(top_kind))),
            ),
            _ => (
                format!("`{}` has no `{}` to close", word, opener_name(want)),
                Some(format!("remove it, or open the block earlier with `{}`", opener_for(want))),
            ),
        };
        errors.push(Diagnostic { line: line_idx + 1 + off, col: ind + 1, message: msg, hint, source_line: lines[line_idx].to_string() });
        if let Some(p) = lower {
            // recover: close the frames above it without more errors
            while stack.len() > p + 1 {
                let f = stack.pop().unwrap();
                fold(stack, f, pos, None);
            }
        } else if top_kind != FK::Root {
            // recover: the closer was meant for the open block
            let f = stack.pop().unwrap();
            fold(stack, f, pos, None);
            return;
        } else {
            return;
        }
    }
    let f = stack.pop().unwrap();
    fold(stack, f, pos, end_label.flatten());
}

fn fold(stack: &mut Vec<Frame>, mut f: Frame, pos: Pos, end_label: Option<String>) {
    let stmt = match f.kind {
        FK::If => {
            f.finish_part();
            Stmt::If { pos: f.pos, parts: f.parts }
        }
        FK::Switch => {
            f.finish_part();
            Stmt::Switch { pos: f.pos, cond: f.cond, parts: f.parts }
        }
        FK::Fork => {
            f.finish_part();
            Stmt::Fork { pos: f.pos, parts: f.parts }
        }
        FK::While => Stmt::While { pos: f.pos, cond: f.cond, is_label: f.is_label, end_label, body: f.cur },
        FK::Repeat => Stmt::Repeat { pos: f.pos, body: f.cur, backward: f.backward, cond: String::new(), is_label: None, not_label: None },
        FK::Root => return,
    };
    let _ = pos;
    if let Some(top) = stack.last_mut() {
        top.cur.push(stmt);
    }
}

#[allow(clippy::too_many_arguments)]
fn close_repeat(stack: &mut Vec<Frame>, errors: &mut Vec<Diagnostic>, lines: &[&str], off: usize, line_idx: usize, ind: usize, pos: Pos, cond: String, is: Option<String>, not: Option<String>) {
    let top_kind = stack.last().unwrap().kind;
    if top_kind != FK::Repeat {
        let lower = stack.iter().rposition(|f| f.kind == FK::Repeat);
        let (msg, hint) = match lower {
            Some(_) => (
                format!("found `repeat while` but the open `{}` from line {} must close first", opener_name(top_kind), stack.last().unwrap().pos.line),
                Some(format!("add `{}` first", closer_for(top_kind))),
            ),
            None if top_kind != FK::Root => (
                format!("expected `{}` to close the `{}` from line {}, found `repeat while`", closer_for(top_kind), opener_name(top_kind), stack.last().unwrap().pos.line),
                Some(format!("did you mean `{}`?", closer_for(top_kind))),
            ),
            None => ("`repeat while` has no `repeat` to close".to_string(), Some("remove it, or open the loop earlier with `repeat`".to_string())),
        };
        errors.push(Diagnostic { line: line_idx + 1 + off, col: ind + 1, message: msg, hint, source_line: lines[line_idx].to_string() });
        match lower {
            Some(p) => {
                while stack.len() > p + 1 {
                    let f = stack.pop().unwrap();
                    fold(stack, f, pos, None);
                }
            }
            None => {
                if top_kind != FK::Root {
                    let f = stack.pop().unwrap();
                    fold(stack, f, pos, None);
                }
                return;
            }
        }
    }
    let f = stack.pop().unwrap();
    let stmt = Stmt::Repeat { pos: f.pos, body: f.cur, backward: f.backward, cond, is_label: is, not_label: not };
    if let Some(top) = stack.last_mut() {
        top.cur.push(stmt);
    }
}

// ── state diagrams (the derived overview) ───────────────────────────────────

fn parse_state(src: &Source, lines: &[&str], mut i: usize, name: String, mut errors: Vec<Diagnostic>) -> Parsed {
    let off = src.line_offset;
    let mut states = vec![];
    let mut transitions = vec![];
    let mut ended = false;
    while i < lines.len() {
        let raw = lines[i];
        let t = raw.trim();
        let pos = Pos { line: i + 1 + off, col: indent_of(raw) + 1 };
        i += 1;
        if t.starts_with("@enduml") {
            ended = true;
            break;
        }
        if t.is_empty() || t.starts_with('\'') || t.starts_with("hide ") || t.starts_with("skinparam") {
            continue;
        }
        if let Some(rest) = t.strip_prefix("state ") {
            // state "title" as id
            let rest = rest.trim();
            if let Some(r) = rest.strip_prefix('"') {
                if let Some(q) = r.find('"') {
                    let title = r[..q].to_string();
                    let tail = r[q + 1..].trim();
                    if let Some(id) = tail.strip_prefix("as ") {
                        states.push(StateDef { id: id.trim().to_string(), title, pos });
                        continue;
                    }
                }
            }
            errors.push(Diagnostic { line: pos.line, col: pos.col, message: "expected `state \"title\" as id`".into(), hint: Some("state \"FL-1 · Title\" as FL1".into()), source_line: raw.to_string() });
            continue;
        }
        if let Some(p) = t.find("-->") {
            let a = t[..p].trim();
            let b = t[p + 3..].trim();
            if a.is_empty() || b.is_empty() {
                errors.push(Diagnostic { line: pos.line, col: pos.col, message: "a transition needs a state on both sides of `-->`".into(), hint: None, source_line: raw.to_string() });
            } else {
                transitions.push((a.to_string(), b.to_string(), pos));
            }
            continue;
        }
        errors.push(Diagnostic { line: pos.line, col: pos.col, message: format!("`{}` is not a state statement", short(t)), hint: Some("use `state \"title\" as id` or `a --> b`".into()), source_line: raw.to_string() });
    }
    if !ended {
        errors.push(Diagnostic { line: lines.len() + off, col: 1, message: "the diagram is never closed: `@enduml` is missing".into(), hint: Some("end the diagram with `@enduml`".into()), source_line: String::new() });
    }
    // transitions must name a known state
    let ids: std::collections::HashSet<&str> = states.iter().map(|s| s.id.as_str()).collect();
    for (a, b, p) in &transitions {
        for x in [a, b] {
            if !ids.contains(x.as_str()) {
                let near = states.iter().map(|s| s.id.as_str()).min_by_key(|s| levenshtein(s, x));
                errors.push(Diagnostic {
                    line: p.line,
                    col: p.col,
                    message: format!("`{}` is not a declared state", x),
                    hint: near.filter(|n| levenshtein(n, x) <= 2).map(|n| format!("did you mean `{}`?", n)),
                    source_line: lines.get(p.line - 1 - off).map(|s| s.to_string()).unwrap_or_default(),
                });
            }
        }
    }
    let diagram = if errors.is_empty() { Some(Diagram { name, kind: Kind::State, body: vec![], states, transitions }) } else { None };
    Parsed { diagram, errors }
}
