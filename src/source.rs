//! Find the diagrams in a file: a `.puml` file is one diagram, a `.md` file
//! holds its diagrams in ```plantuml fences.

pub struct Chunk {
    pub text: String,
    pub line_offset: usize,
}

pub fn chunks(path: &str, text: &str) -> Vec<Chunk> {
    if !path.ends_with(".md") {
        return vec![Chunk { text: text.to_string(), line_offset: 0 }];
    }
    let mut out = vec![];
    let mut cur: Option<(usize, Vec<&str>)> = None;
    for (i, l) in text.lines().enumerate() {
        match &mut cur {
            None => {
                if l.trim_start().starts_with("```plantuml") {
                    cur = Some((i + 1, vec![]));
                }
            }
            Some((start, buf)) => {
                if l.trim_start().starts_with("```") {
                    out.push(Chunk { text: buf.join("\n"), line_offset: *start });
                    cur = None;
                } else {
                    buf.push(l);
                }
            }
        }
    }
    out
}
