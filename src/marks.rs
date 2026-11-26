//! Marks inside the text of a node: the notation of the first Like A Human
//! flows (Sep 2026), kept as words so that PlantUML still reads the fence.
//!
//! | mark | example | meaning |
//! |---|---|---|
//! | status | `[v2]` `[new]` `[fix]` `[copy]` `[human]` `[old]` `[open]` `[outside]` `[wk 47]` | where the step stands |
//! | citation | `⤳ convex/commands/invites.ts:681` | where the behaviour lives; a node without one is an intention |
//! | pitfall | `⚠ the address is never shown` | where it goes wrong if you do not pay attention |
//! | event | `event: invite.claimed` | the domain event that proves the step |
//! | reference | `FL-1.3.1` | another flow record |
//!
//! Dutch status words of the first flows are read too: `nieuw`, `kopie`, `mens`, `oud`, `buiten`.

#[derive(Debug, Clone, Default)]
pub struct Cite {
    pub path: String,
    pub line: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Marks {
    /// The text without status, citations, pitfalls and events.
    pub text: String,
    pub status: Vec<String>,
    pub cites: Vec<Cite>,
    pub pitfalls: Vec<String>,
    pub events: Vec<String>,
    pub refs: Vec<String>,
}

const STATUS: &[(&str, &str)] = &[
    ("v2", "v2"),
    ("live", "live"),
    ("new", "new"),
    ("nieuw", "new"),
    ("copy", "copy"),
    ("kopie", "copy"),
    ("fix", "fix"),
    ("human", "human"),
    ("mens", "human"),
    ("old", "old"),
    ("oud", "old"),
    ("open", "open"),
    ("outside", "outside"),
    ("buiten", "outside"),
    ("intent", "intent"),
];

fn status_of(inner: &str) -> Option<String> {
    let t = inner.trim().to_lowercase();
    for (k, v) in STATUS {
        if t == *k {
            return Some(v.to_string());
        }
    }
    if let Some(n) = t.strip_prefix("wk ") {
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("wk {}", n));
        }
    }
    None
}

/// `FL-1.3.1`, `FL-10.2`: the ids that a text cites.
pub fn refs_in(text: &str) -> Vec<(usize, usize)> {
    let b = text.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i + 4 <= b.len() {
        if &b[i..i + 3] == b"FL-" && b[i + 3].is_ascii_digit() && (i == 0 || !b[i - 1].is_ascii_alphanumeric()) {
            let mut j = i + 3;
            while j < b.len() && (b[j].is_ascii_digit() || (b[j] == b'.' && j + 1 < b.len() && b[j + 1].is_ascii_digit())) {
                j += 1;
            }
            out.push((i, j));
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

pub fn parse(text: &str) -> Marks {
    let mut m = Marks::default();
    // split on the interpunct that the flows use between parts
    let parts: Vec<&str> = text.split(" · ").collect();
    let mut keep: Vec<String> = vec![];
    for part in parts {
        let mut words: Vec<String> = vec![];
        let p = part.trim();
        if let Some(pit) = p.strip_prefix('⚠') {
            m.pitfalls.push(pit.trim().to_string());
            continue;
        }
        let mut rest = p;
        // tokens inside a part
        loop {
            if rest.is_empty() {
                break;
            }
            // status [..]
            if let Some(r) = rest.strip_prefix('[') {
                if let Some(e) = r.find(']') {
                    if let Some(st) = status_of(&r[..e]) {
                        m.status.push(st);
                        rest = r[e + 1..].trim_start();
                        continue;
                    }
                }
            }
            if let Some(r) = rest.strip_prefix('⤳') {
                let r = r.trim_start();
                let end = r.find(char::is_whitespace).unwrap_or(r.len());
                let tok = r[..end].trim_end_matches([',', ';']);
                let (path, line) = match tok.rfind(':') {
                    Some(c) if tok[c + 1..].chars().next().map_or(false, |ch| ch.is_ascii_digit()) => (tok[..c].to_string(), Some(tok[c + 1..].to_string())),
                    _ => (tok.to_string(), None),
                };
                if !path.is_empty() {
                    m.cites.push(Cite { path, line });
                }
                rest = r[end..].trim_start();
                continue;
            }
            if let Some(r) = rest.strip_prefix("event:") {
                let r = r.trim_start();
                let end = r.find(char::is_whitespace).unwrap_or(r.len());
                m.events.push(r[..end].trim_end_matches([',', ';']).to_string());
                rest = r[end..].trim_start();
                continue;
            }
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            words.push(rest[..end].to_string());
            rest = rest[end..].trim_start();
        }
        let w = words.join(" ");
        if !w.is_empty() {
            keep.push(w);
        }
    }
    m.text = keep.join(" · ");
    for (a, b) in refs_in(&m.text) {
        let r = m.text[a..b].to_string();
        if !m.refs.contains(&r) {
            m.refs.push(r);
        }
    }
    for c in &m.pitfalls {
        for (a, b) in refs_in(c) {
            let r = c[a..b].to_string();
            if !m.refs.contains(&r) {
                m.refs.push(r);
            }
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_mark() {
        let m = parse("c · preview before sign-in [new] · ⤳ convex/queries/invites.ts:75 · ⚠ the address is never shown · event: invite.previewed · FL-1.3.1");
        assert_eq!(m.text, "c · preview before sign-in · FL-1.3.1");
        assert_eq!(m.status, vec!["new"]);
        assert_eq!(m.cites[0].path, "convex/queries/invites.ts");
        assert_eq!(m.cites[0].line.as_deref(), Some("75"));
        assert_eq!(m.pitfalls, vec!["the address is never shown"]);
        assert_eq!(m.events, vec!["invite.previewed"]);
        assert_eq!(m.refs, vec!["FL-1.3.1"]);
    }

    #[test]
    fn leaves_plain_brackets() {
        let m = parse("a notice Pinned by [first name] above the bar");
        assert_eq!(m.text, "a notice Pinned by [first name] above the bar");
        assert!(m.status.is_empty());
    }
}
