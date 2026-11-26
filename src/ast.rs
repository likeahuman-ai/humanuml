//! Syntax tree of a flow diagram, with a position on every node.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pos {
    /// 1-based line in the source file.
    pub line: usize,
    /// 1-based column (in characters).
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    Then,
    ElseIf,
    Else,
    Case,
    Branch,
}

/// One branch of an `if`, a `switch` or a `fork`.
#[derive(Debug, Clone)]
pub struct Part {
    pub kind: PartKind,
    pub pos: Pos,
    /// Condition of `if` / `elseif`; empty for the other kinds.
    pub cond: String,
    /// The branch label: `then (yes)`, `else (no)`, `case (label)`.
    pub label: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Lane { name: String, pos: Pos },
    Start(Pos),
    Stop(Pos),
    End(Pos),
    Action { text: String, pos: Pos },
    /// A note attached to the previous node.
    Note { side: String, text: String, pos: Pos },
    If { pos: Pos, parts: Vec<Part> },
    Switch { pos: Pos, cond: String, parts: Vec<Part> },
    While { pos: Pos, cond: String, is_label: Option<String>, end_label: Option<String>, body: Vec<Stmt> },
    Repeat {
        pos: Pos,
        body: Vec<Stmt>,
        backward: Option<(String, Pos)>,
        cond: String,
        is_label: Option<String>,
        not_label: Option<String>,
    },
    Fork { pos: Pos, parts: Vec<Part> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Activity,
    /// The derived overview: `state "title" as id` and `a --> b`.
    State,
}

#[derive(Debug, Clone)]
pub struct StateDef {
    pub id: String,
    pub title: String,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub name: String,
    pub kind: Kind,
    pub body: Vec<Stmt>,
    pub states: Vec<StateDef>,
    pub transitions: Vec<(String, String, Pos)>,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub message: String,
    pub hint: Option<String>,
    pub source_line: String,
}
