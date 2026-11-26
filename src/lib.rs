//! humanuml: flow diagrams as plain text.
//!
//! `parse` reads the PlantUML activity subset, `model` builds the graph,
//! `render` draws it as SVG.

pub mod ast;
pub mod font;
pub mod layout;
pub mod marks;
pub mod svg;
pub mod tree;
pub mod json;
pub mod model;
pub mod parser;
pub mod source;

pub use ast::*;
pub use parser::{parse, Parsed, Source};

/// Render one diagnostic in the compiler style:
/// `file:line:col: error: message`, the source line, a caret and the hint.
pub fn format_diagnostic(file: &str, d: &Diagnostic) -> String {
    let mut s = format!("{}:{}:{}: error: {}\n", file, d.line, d.col, d.message);
    let num = d.line.to_string();
    let pad = " ".repeat(num.len());
    if !d.source_line.is_empty() {
        s.push_str(&format!(" {} |\n {} | {}\n {} | {}^\n", pad, num, d.source_line, pad, " ".repeat(d.col.saturating_sub(1))));
    }
    if let Some(h) = &d.hint {
        s.push_str(&format!(" {} = hint: {}\n", pad, h));
    }
    s
}
