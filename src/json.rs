//! A tiny JSON value and writer, so the crate has no dependencies.

#[derive(Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }
    pub fn opt(s: &Option<String>) -> Json {
        match s {
            Some(s) => Json::Str(s.clone()),
            None => Json::Null,
        }
    }
    pub fn int(n: usize) -> Json {
        Json::Num(n as f64)
    }

    pub fn to_string_pretty(&self) -> String {
        let mut s = String::new();
        self.write(&mut s, 0);
        s.push('\n');
        s
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64))
                } else {
                    out.push_str(&format!("{}", n))
                }
            }
            Json::Str(s) => escape(s, out),
            Json::Arr(a) => {
                if a.is_empty() {
                    out.push_str("[]");
                } else if a.iter().all(|x| matches!(x, Json::Str(_) | Json::Num(_) | Json::Bool(_) | Json::Null)) {
                    out.push('[');
                    for (i, x) in a.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        x.write(out, depth);
                    }
                    out.push(']');
                } else {
                    out.push_str("[\n");
                    for (i, x) in a.iter().enumerate() {
                        indent(out, depth + 1);
                        x.write(out, depth + 1);
                        if i + 1 < a.len() {
                            out.push(',');
                        }
                        out.push('\n');
                    }
                    indent(out, depth);
                    out.push(']');
                }
            }
            Json::Obj(o) => {
                // small flat objects stay on one line
                let flat = o.iter().all(|(_, v)| matches!(v, Json::Str(_) | Json::Num(_) | Json::Bool(_) | Json::Null)) && o.len() <= 12;
                if flat {
                    out.push('{');
                    for (i, (k, v)) in o.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        escape(k, out);
                        out.push_str(": ");
                        v.write(out, depth);
                    }
                    out.push('}');
                } else {
                    out.push_str("{\n");
                    for (i, (k, v)) in o.iter().enumerate() {
                        indent(out, depth + 1);
                        escape(k, out);
                        out.push_str(": ");
                        v.write(out, depth + 1);
                        if i + 1 < o.len() {
                            out.push(',');
                        }
                        out.push('\n');
                    }
                    indent(out, depth);
                    out.push('}');
                }
            }
        }
    }
}

fn indent(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push_str("  ");
    }
}

fn escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
