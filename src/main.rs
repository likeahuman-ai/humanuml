use humanuml::source::chunks;
use humanuml::{format_diagnostic, parse, Source};
use std::process::ExitCode;

const USAGE: &str = "humanuml: flow diagrams as plain text

usage:
  humanuml check <files>...      parse; errors by line and column, with hints
  humanuml json <file>           the parsed flow as JSON
  humanuml render <file> -o out.svg [--theme classic|soft|navy|ink]

A file is a .puml file or a .md file with ```plantuml fences.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("check") => check(&args[1..]),
        Some("json") => json(&args[1..]),
        Some("render") => render(&args[1..]),
        _ => {
            eprintln!("{}", USAGE);
            ExitCode::from(2)
        }
    }
}

fn check(files: &[String]) -> ExitCode {
    if files.is_empty() {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    }
    let mut diagrams = 0;
    let mut bad = 0;
    for f in files {
        let text = match std::fs::read_to_string(f) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}: cannot read: {}", f, e);
                bad += 1;
                continue;
            }
        };
        for c in chunks(f, &text) {
            diagrams += 1;
            let p = parse(&Source { text: &c.text, line_offset: c.line_offset });
            if !p.errors.is_empty() {
                bad += 1;
            }
            for e in &p.errors {
                eprint!("{}", format_diagnostic(f, e));
            }
        }
    }
    if bad > 0 {
        eprintln!("{} of {} diagrams have errors", bad, diagrams);
        ExitCode::from(1)
    } else {
        println!("ok: {} diagrams", diagrams);
        ExitCode::SUCCESS
    }
}

fn json(files: &[String]) -> ExitCode {
    let Some(f) = files.first() else {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    };
    let text = match std::fs::read_to_string(f) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: cannot read: {}", f, e);
            return ExitCode::from(1);
        }
    };
    let mut out = vec![];
    for c in chunks(f, &text) {
        let p = parse(&Source { text: &c.text, line_offset: c.line_offset });
        for e in &p.errors {
            eprint!("{}", format_diagnostic(f, e));
        }
        match p.diagram {
            Some(d) => out.push(humanuml::model::build(&d).to_json()),
            None => return ExitCode::from(1),
        }
    }
    let j = if out.len() == 1 { out.pop().unwrap() } else { humanuml::json::Json::Arr(out) };
    print!("{}", j.to_string_pretty());
    ExitCode::SUCCESS
}

fn render(args: &[String]) -> ExitCode {
    let mut files = vec![];
    let mut out = None;
    let mut theme = String::from("classic");
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-o" {
            out = args.get(i + 1).cloned();
            i += 2;
        } else if args[i] == "--theme" {
            theme = args.get(i + 1).cloned().unwrap_or_default();
            i += 2;
        } else {
            files.push(args[i].clone());
            i += 1;
        }
    }
    let Some(out) = out else {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    };
    let Some(th) = humanuml::svg::Theme::named(&theme) else {
        eprintln!("unknown theme `{}`; themes: {}", theme, humanuml::svg::Theme::names().join(", "));
        return ExitCode::from(2);
    };
    if files.is_empty() {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    }
    // one diagram: -o is a file; several diagrams: -o is a folder
    let mut jobs = vec![];
    for f in &files {
        let text = match std::fs::read_to_string(f) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}: cannot read: {}", f, e);
                return ExitCode::from(1);
            }
        };
        for c in chunks(f, &text) {
            jobs.push((f.clone(), c));
        }
    }
    let many = jobs.len() > 1;
    if many {
        let _ = std::fs::create_dir_all(&out);
    }
    let mut failed = 0;
    for (f, c) in jobs {
        let p = parse(&Source { text: &c.text, line_offset: c.line_offset });
        for e in &p.errors {
            eprint!("{}", format_diagnostic(&f, e));
        }
        let Some(d) = p.diagram else {
            failed += 1;
            continue;
        };
        let m = humanuml::model::build(&d);
        let svg = humanuml::svg::render_with(&m, &d, &th);
        let path = if many { format!("{}/{}.svg", out, d.name) } else { out.clone() };
        if let Err(e) = std::fs::write(&path, svg) {
            eprintln!("{}: cannot write: {}", path, e);
            failed += 1;
        }
    }
    if failed > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
}
