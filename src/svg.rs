//! Turn a layout into SVG. No font or graphics library: text width comes from
//! the embedded metrics in `font.rs`. Every node is a `<g data-node="id">`.

use crate::ast::*;
use crate::font::text_width;
use crate::layout::*;
use crate::model::Model;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c => o.push(c),
        }
    }
    o
}

const MARGIN: f64 = 24.0;
const LANE_PAD: f64 = 22.0;
const HEADER_H: f64 = 30.0;

#[derive(Clone, Copy, PartialEq)]
pub enum DecisionShape {
    Hex,
    Pill,
}

/// Every colour and shape choice of a drawing. See `Theme::named`.
#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,
    pub font: &'static str,
    pub bg: &'static str,
    pub lane_a: &'static str,
    pub lane_b: &'static str,
    pub lane_line: &'static str,
    pub lane_radius: f64,
    pub header_fill: &'static str,
    pub header_text: &'static str,
    pub card_fill: &'static str,
    pub card_line: &'static str,
    pub card_w: f64,
    pub card_r: f64,
    pub shadow: bool,
    pub text: &'static str,
    pub key: &'static str,
    pub key_bold: bool,
    pub dec_fill: &'static str,
    pub dec_line: &'static str,
    pub dec_text: &'static str,
    pub dec_shape: DecisionShape,
    pub back_fill: &'static str,
    pub line: &'static str,
    pub line_w: f64,
    pub corner: f64,
    pub term: &'static str,
    pub label: &'static str,
    pub merge_fill: &'static str,
    pub bar: &'static str,
    pub dark: bool,
    pub link: &'static str,
}

impl Theme {
    pub fn names() -> &'static [&'static str] {
        &["classic", "soft", "navy", "ink"]
    }

    pub fn named(n: &str) -> Option<Theme> {
        let base = Theme {
            name: "classic",
            font: "Helvetica, Arial, sans-serif",
            bg: "#ffffff",
            lane_a: "#fbfbfb",
            lane_b: "#f5f7fa",
            lane_line: "#c9c9c9",
            lane_radius: 0.0,
            header_fill: "#ececec",
            header_text: "#333333",
            card_fill: "#f6f2e7",
            card_line: "#8a7f66",
            card_w: 1.2,
            card_r: 7.0,
            shadow: false,
            text: "#1f1f1f",
            key: "#1f1f1f",
            key_bold: false,
            dec_fill: "#e7eef6",
            dec_line: "#56708c",
            dec_text: "#1f1f1f",
            dec_shape: DecisionShape::Hex,
            back_fill: "#f6f2e7",
            line: "#5b5b5b",
            line_w: 1.2,
            corner: 0.0,
            term: "#222222",
            label: "#444444",
            merge_fill: "#ffffff",
            bar: "#333333",
            dark: false,
            link: "#12345A",
        };
        Some(match n {
            "classic" => base,
            // brand soft UI: cream ground, white cards that float, navy line, lime key chips
            "soft" => Theme {
                name: "soft",
                font: "'Libre Franklin', 'Helvetica Neue', Helvetica, Arial, sans-serif",
                bg: "#FFF8E7",
                lane_a: "#FFFDF6",
                lane_b: "#F8F0DD",
                lane_line: "#E8DCC8",
                lane_radius: 18.0,
                header_fill: "#12345A",
                header_text: "#FFF8E7",
                card_fill: "#FFFFFF",
                card_line: "none",
                card_w: 0.0,
                card_r: 14.0,
                shadow: true,
                text: "#1C2B3A",
                key: "#12345A",
                key_bold: true,
                dec_fill: "#DEFEB3",
                dec_line: "none",
                dec_text: "#12345A",
                dec_shape: DecisionShape::Pill,
                back_fill: "#F0E8D8",
                line: "#6A87AD",
                line_w: 1.6,
                corner: 10.0,
                term: "#12345A",
                label: "#46576A",
                merge_fill: "#12345A",
                bar: "#12345A",
                ..base
            },
            // the dark mode of the brand: deep navy ladder, lime where it matters
            "navy" => Theme {
                name: "navy",
                font: "'Libre Franklin', 'Helvetica Neue', Helvetica, Arial, sans-serif",
                bg: "#08142A",
                lane_a: "#0B1A33",
                lane_b: "#0F1F38",
                lane_line: "#1F3358",
                lane_radius: 18.0,
                header_fill: "#15294A",
                header_text: "#EAF1F8",
                card_fill: "#15294A",
                card_line: "#2A4470",
                card_w: 1.0,
                card_r: 12.0,
                shadow: false,
                text: "#EAF1F8",
                key: "#DEFEB3",
                key_bold: true,
                dec_fill: "#08142A",
                dec_line: "#DEFEB3",
                dec_text: "#EAF1F8",
                dec_shape: DecisionShape::Pill,
                back_fill: "#0F1F38",
                line: "#6A87AD",
                line_w: 1.5,
                corner: 10.0,
                term: "#DEFEB3",
                label: "#9FB3C8",
                merge_fill: "#DEFEB3",
                bar: "#DEFEB3",
                dark: true,
                link: "#A3D2FF",
                ..base
            },
            // editorial: white page, hairlines, no fill, one lime mark per decision
            "ink" => Theme {
                name: "ink",
                font: "'Libre Franklin', 'Helvetica Neue', Helvetica, Arial, sans-serif",
                bg: "#FFFFFF",
                lane_a: "#FFFFFF",
                lane_b: "#FAFAF7",
                lane_line: "#E8E4DA",
                lane_radius: 0.0,
                header_fill: "#FFFFFF",
                header_text: "#12345A",
                card_fill: "#FFFFFF",
                card_line: "#12345A",
                card_w: 1.0,
                card_r: 3.0,
                shadow: false,
                text: "#1C2B3A",
                key: "#12345A",
                key_bold: true,
                dec_fill: "#FFFFFF",
                dec_line: "#12345A",
                dec_text: "#1C2B3A",
                dec_shape: DecisionShape::Hex,
                back_fill: "#FFFFFF",
                line: "#12345A",
                line_w: 1.0,
                corner: 0.0,
                term: "#12345A",
                label: "#46576A",
                merge_fill: "#DEFEB3",
                bar: "#12345A",
                ..base
            },
            _ => return None,
        })
    }
}

fn f(x: f64) -> String {
    let s = format!("{:.1}", x);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A polyline as an SVG path, with rounded corners of radius `r`.
fn path_d(pts: &[(f64, f64)], r: f64) -> String {
    let mut p: Vec<(f64, f64)> = vec![];
    for q in pts {
        if p.last().map_or(true, |l| (l.0 - q.0).abs() > 0.01 || (l.1 - q.1).abs() > 0.01) {
            p.push(*q);
        }
    }
    let mut d = format!("M{} {}", f(p[0].0), f(p[0].1));
    for i in 1..p.len() {
        if r > 0.0 && i + 1 < p.len() {
            let (a, b, c) = (p[i - 1], p[i], p[i + 1]);
            let l1 = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let l2 = ((c.0 - b.0).powi(2) + (c.1 - b.1).powi(2)).sqrt();
            let rr = r.min(l1 / 2.0).min(l2 / 2.0);
            let p1 = (b.0 - (b.0 - a.0) / l1 * rr, b.1 - (b.1 - a.1) / l1 * rr);
            let p2 = (b.0 + (c.0 - b.0) / l2 * rr, b.1 + (c.1 - b.1) / l2 * rr);
            d.push_str(&format!(" L{} {} Q{} {} {} {}", f(p1.0), f(p1.1), f(b.0), f(b.1), f(p2.0), f(p2.1)));
        } else {
            d.push_str(&format!(" L{} {}", f(p[i].0), f(p[i].1)));
        }
    }
    d
}

pub fn render(model: &Model, d: &Diagram) -> String {
    render_with(model, d, &Theme::named("soft").unwrap())
}

pub fn render_with(model: &Model, d: &Diagram, th: &Theme) -> String {
    if d.kind == Kind::State {
        return render_state(model);
    }
    let laid = layout(model, d);
    let root = &laid.root;
    let nl = root.ext.len();
    // lane columns from the extents of the whole diagram
    let mut centers = vec![0.0; nl];
    let mut bands = vec![(0.0, 0.0); nl];
    let mut x = MARGIN;
    let lane_gap = if th.lane_radius > 0.0 { 10.0 } else { 0.0 };
    for l in 0..nl {
        let (el, er) = root.ext[l].unwrap_or((40.0, 40.0));
        let (el, er) = (el.max(20.0), er.max(20.0));
        let start = x;
        centers[l] = x + LANE_PAD + el;
        x = centers[l] + er + LANE_PAD;
        bands[l] = (start, x);
        x += lane_gap;
    }
    x -= lane_gap;
    let has_lanes = !laid.lanes.is_empty();
    let width = x + MARGIN;
    let top = MARGIN + if has_lanes { HEADER_H } else { 0.0 } + 14.0;
    let height = top + root.h + MARGIN + 6.0;
    let px = |p: &Port| centers[p.lane] + p.dx;
    let py = |y: f64| top + y;

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\" font-family=\"{}\" data-diagram=\"{}\" data-theme=\"{}\">\n",
        f(width), f(height), f(width), f(height), th.font.replace('\'', "&apos;"), esc(&model.name), th.name
    ));
    s.push_str(&format!(
        "<defs><marker id=\"arr\" viewBox=\"0 0 10 10\" refX=\"8\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M1 1.5 L9 5 L1 8.5\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></marker>",
        th.line
    ));
    if th.shadow {
        s.push_str("<filter id=\"lift\" x=\"-10%\" y=\"-10%\" width=\"120%\" height=\"140%\"><feDropShadow dx=\"0\" dy=\"3\" stdDeviation=\"4\" flood-color=\"#12345A\" flood-opacity=\"0.13\"/></filter>");
    }
    s.push_str("</defs>\n");
    s.push_str(&format!("<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>\n", f(width), f(height), th.bg));
    if has_lanes {
        for (i, name) in laid.lanes.iter().enumerate() {
            let (a, b) = bands[i];
            let fill = if i % 2 == 0 { th.lane_a } else { th.lane_b };
            let h = height - 2.0 * MARGIN + 6.0;
            if th.lane_radius > 0.0 {
                s.push_str(&format!(
                    "<g class=\"lane\" data-lane=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\"/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" font-weight=\"700\" letter-spacing=\"1.2\" fill=\"{}\">{}</text></g>\n",
                    esc(name), f(a), f(MARGIN), f(b - a), f(h), f(th.lane_radius), fill, th.lane_line,
                    f(a + 10.0), f(MARGIN + 8.0), f(b - a - 20.0), f(HEADER_H - 12.0), f((HEADER_H - 12.0) / 2.0), th.header_fill,
                    f((a + b) / 2.0), f(MARGIN + 8.0 + (HEADER_H - 12.0) / 2.0 + 4.0), th.header_text, esc(&name.to_uppercase())
                ));
            } else {
                s.push_str(&format!(
                    "<g class=\"lane\" data-lane=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\"/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\">{}</text></g>\n",
                    esc(name), f(a), f(MARGIN), f(b - a), f(h), fill, th.lane_line,
                    f(a), f(MARGIN), f(b - a), f(HEADER_H), th.header_fill, th.lane_line, f((a + b) / 2.0), f(MARGIN + 20.0), th.header_text, esc(name)
                ));
            }
        }
    }
    // edges
    s.push_str(&format!("<g fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n", th.line, th.line_w));
    let mk = |arrow: bool| if arrow { " marker-end=\"url(#arr)\"" } else { "" };
    let c = th.corner;
    for e in &root.edges {
        match e {
            Edge::Flow { from, to, ymid, arrow } => {
                let (x1, y1, x2, y2) = (px(from), py(from.y), px(to), py(to.y));
                let pts = if (x1 - x2).abs() < 0.5 {
                    vec![(x1, y1), (x2, y2)]
                } else {
                    let ym = ymid.map(py).unwrap_or((y1 + y2) / 2.0);
                    vec![(x1, y1), (x1, ym), (x2, ym), (x2, y2)]
                };
                s.push_str(&format!("<path d=\"{}\"{}/>\n", path_d(&pts, c), mk(*arrow)));
            }
            Edge::Fan { dec, dw, dh, to, bus_y, arrow } => {
                let (dx, dy) = (px(dec), py(dec.y));
                let (tx, ty) = (px(to), py(to.y));
                let pts = if let Some(by) = bus_y {
                    let by = py(*by);
                    if (tx - dx).abs() < 0.5 { vec![(dx, dy + dh), (tx, ty)] } else { vec![(dx, dy + dh), (dx, by), (tx, by), (tx, ty)] }
                } else if (tx - dx).abs() < 0.5 {
                    vec![(dx, dy + dh), (tx, ty)]
                } else if tx > dx {
                    vec![(dx + dw / 2.0, dy + dh / 2.0), (tx, dy + dh / 2.0), (tx, ty)]
                } else {
                    vec![(dx - dw / 2.0, dy + dh / 2.0), (tx, dy + dh / 2.0), (tx, ty)]
                };
                s.push_str(&format!("<path d=\"{}\"{}/>\n", path_d(&pts, c), mk(*arrow)));
            }
            Edge::Poly { pts, arrow } => {
                let v: Vec<(f64, f64)> = pts.iter().map(|p| (px(p), py(p.y))).collect();
                s.push_str(&format!("<path d=\"{}\"{}/>\n", path_d(&v, c), mk(*arrow)));
            }
            Edge::Bar { .. } => {}
        }
    }
    s.push_str("</g>\n");
    // bars
    for e in &root.edges {
        if let Edge::Bar { pts, id } = e {
            let xs: Vec<f64> = pts.iter().map(px).collect();
            let a = xs.iter().cloned().fold(f64::INFINITY, f64::min) - 12.0;
            let b = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 12.0;
            let y = py(pts[0].y);
            let attr = id.as_ref().map(|i| format!(" data-node=\"{}\"", esc(i))).unwrap_or_default();
            s.push_str(&format!("<g class=\"node bar\"{}><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"4\" rx=\"{}\" fill=\"{}\"/></g>\n", attr, f(a), f(y - 2.0), f(b - a), if th.corner > 0.0 { "2" } else { "0" }, th.bar));
        }
    }
    // items
    let filt = if th.shadow { " filter=\"url(#lift)\"" } else { "" };
    let stroke_of = |col: &str, w: f64| if col == "none" || w == 0.0 { "stroke=\"none\"".to_string() } else { format!("stroke=\"{}\" stroke-width=\"{}\"", col, f(w)) };
    for it in &root.items {
        let cx = centers[it.lane] + it.dx;
        let y = py(it.y);
        let attr = it.id.as_ref().map(|i| format!(" data-node=\"{}\"", esc(i))).unwrap_or_default();
        match it.shape {
            Shape::Start => s.push_str(&format!("<g class=\"node start\"{}><circle cx=\"{}\" cy=\"{}\" r=\"8\" fill=\"{}\"/></g>\n", attr, f(cx), f(y + 8.0), th.term)),
            Shape::Stop => s.push_str(&format!("<g class=\"node stop\"{}><circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.8\"/><circle cx=\"{}\" cy=\"{}\" r=\"5.5\" fill=\"{}\"/></g>\n", attr, f(cx), f(y + 11.0), th.term, f(cx), f(y + 11.0), th.term)),
            Shape::End => s.push_str(&format!("<g class=\"node end\"{}><circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.8\"/><path d=\"M{} {} l8 8 M{} {} l-8 8\" stroke=\"{}\" stroke-width=\"1.8\" stroke-linecap=\"round\"/></g>\n", attr, f(cx), f(y + 11.0), th.term, f(cx - 4.0), f(y + 7.0), f(cx + 4.0), f(y + 7.0), th.term)),
            Shape::Merge => {
                if th.corner > 0.0 {
                    s.push_str(&format!("<circle class=\"merge\" cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"{}\"/>\n", f(cx), f(y + 6.0), th.merge_fill));
                } else {
                    s.push_str(&format!("<polygon class=\"merge\" points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\"/>\n", f(cx), f(y), f(cx + 6.0), f(y + 6.0), f(cx), f(y + 12.0), f(cx - 6.0), f(y + 6.0), th.merge_fill, th.line));
                }
            }
            Shape::Action | Shape::BackAction => {
                let back = it.shape == Shape::BackAction;
                let dash = if back { " stroke-dasharray=\"4 3\"" } else { "" };
                let cls = if back { "node action backward" } else { "node action" };
                let fill = if back { th.back_fill } else { th.card_fill };
                let (lc, lw) = if back && th.card_line == "none" { ("#6A87AD", 1.2) } else { (th.card_line, th.card_w) };
                s.push_str(&format!("<g class=\"{}\"{}><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" {}{}{}/>", cls, attr, f(cx - it.w / 2.0), f(y), f(it.w), f(it.h), f(th.card_r), fill, stroke_of(lc, lw), dash, if back { "" } else { filt }));
                text_block(&mut s, it, cx, y, it.h, th, th.text);
                s.push_str("</g>\n");
            }
            Shape::Decision => {
                let (w, h) = (it.w, it.h);
                let (x0, x1) = (cx - w / 2.0, cx + w / 2.0);
                let shape = match th.dec_shape {
                    DecisionShape::Pill => format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" {}{}/>", f(x0), f(y), f(w), f(h), f((h / 2.0).min(22.0)), th.dec_fill, stroke_of(th.dec_line, 1.4), filt),
                    DecisionShape::Hex => {
                        let sl = 22.0f64.min(w / 3.0);
                        format!(
                            "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" {}/>",
                            f(x0 + sl), f(y), f(x1 - sl), f(y), f(x1), f(y + h / 2.0), f(x1 - sl), f(y + h), f(x0 + sl), f(y + h), f(x0), f(y + h / 2.0), th.dec_fill, stroke_of(th.dec_line, 1.2)
                        )
                    }
                };
                s.push_str(&format!("<g class=\"node decision\"{}>{}", attr, shape));
                text_block(&mut s, it, cx, y, h, th, th.dec_text);
                s.push_str("</g>\n");
            }
            Shape::Label => {
                s.push_str(&format!("<text class=\"label\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">", f(LFS), th.label));
                for (i, l) in it.lines.iter().enumerate() {
                    s.push_str(&format!("<tspan x=\"{}\" y=\"{}\">{}</tspan>", f(cx), f(y + i as f64 * LLH + 10.0), esc(l)));
                }
                s.push_str("</text>\n");
            }
        }
    }
    s.push_str("</svg>\n");
    s
}

/// `g · text` at the start of a line: the key is drawn in its own colour.
fn split_key_line(l: &str) -> Option<(&str, &str)> {
    let p = l.find(" · ")?;
    let k = &l[..p];
    if !k.is_empty() && k.len() <= 3 && k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        Some((k, &l[p + 1..]))
    } else {
        None
    }
}

/// Colours of a status chip: (fill, text).
fn chip_colors(st: &str, dark: bool) -> (&'static str, &'static str) {
    let light = match st {
        "new" => ("#DEFEB3", "#12345A"),
        "fix" => ("#FFEBE8", "#7E2924"),
        "v2" | "live" => ("#D8FBDC", "#165927"),
        "copy" => ("#E6F1FF", "#274C76"),
        "human" => ("#F0E8D8", "#12345A"),
        s if s.starts_with("wk") => ("#FFEED0", "#614502"),
        _ => ("#F0E8D8", "#46576A"),
    };
    if !dark {
        return light;
    }
    match st {
        "new" => ("#DEFEB3", "#12345A"),
        "fix" => ("#5A2424", "#E88F8F"),
        "v2" | "live" => ("#1D3D2C", "#7FC79A"),
        "copy" => ("#1B3352", "#8FBCE4"),
        s if s.starts_with("wk") => ("#3D321A", "#E4C57E"),
        _ => ("#12345A", "#C7D4E2"),
    }
}

/// The FL ids in a line become links to the drawing of that record.
fn line_spans(l: &str, th: &Theme) -> String {
    let mut out = String::new();
    let mut last = 0;
    for (a, b) in crate::marks::refs_in(l) {
        out.push_str(&esc(&l[last..a]));
        let id = &l[a..b];
        out.push_str(&format!(
            "<a href=\"{}.svg\" class=\"ref\" data-ref=\"{}\"><tspan fill=\"{}\" text-decoration=\"underline\">{}</tspan></a>",
            id.replace('.', "_"), id, th.link, esc(id)
        ));
        last = b;
    }
    out.push_str(&esc(&l[last..]));
    out
}

fn text_block(s: &mut String, it: &Item, cx: f64, y: f64, h: f64, th: &Theme, fill: &str) {
    let lines = &it.lines;
    let total = content_h(lines.len(), it.extras.len(), !it.chips.is_empty());
    let mut yy = y + (h - total) / 2.0;
    if !it.chips.is_empty() {
        let cw: f64 = it.chips.iter().map(|c| chip_w(c) + 4.0).sum::<f64>() - 4.0;
        let mut x = cx - cw / 2.0;
        for c in &it.chips {
            let w = chip_w(c);
            let (bg, fg) = chip_colors(c, th.dark);
            s.push_str(&format!(
                "<g class=\"chip\" data-status=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"700\" letter-spacing=\"0.6\" fill=\"{}\">{}</text></g>",
                esc(c), f(x), f(yy), f(w), f(CHIP_H), f(CHIP_H / 2.0), bg, f(x + w / 2.0), f(yy + 10.5), f(CHIP_FS), fg, esc(&c.to_uppercase())
            ));
            x += w + 4.0;
        }
        yy += CHIP_H + 5.0;
    }
    s.push_str(&format!("<text text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">", f(FS), fill));
    for (i, l) in lines.iter().enumerate() {
        let ly = f(yy + i as f64 * LH + 11.0);
        match (i, split_key_line(l)) {
            (0, Some((k, rest))) if th.key_bold => {
                s.push_str(&format!("<tspan x=\"{}\" y=\"{}\"><tspan font-weight=\"700\" fill=\"{}\">{}</tspan>{}</tspan>", f(cx), ly, th.key, esc(k), line_spans(&format!(" {}", rest), th)));
            }
            _ => s.push_str(&format!("<tspan x=\"{}\" y=\"{}\">{}</tspan>", f(cx), ly, line_spans(l, th))),
        }
    }
    s.push_str("</text>");
    let base = yy + lines.len() as f64 * LH;
    for (j, (kind, l)) in it.extras.iter().enumerate() {
        let ly = f(base + j as f64 * LH + 11.0);
        match kind {
            Extra::Pitfall => s.push_str(&format!(
                "<text class=\"pitfall\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{}</text>",
                f(cx), ly, f(FS), if th.dark { "#E4C57E" } else { "#8A5A00" }, line_spans(l, th)
            )),
            Extra::Cite | Extra::Event => s.push_str(&format!(
                "<text class=\"{}\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"'Space Mono', Menlo, monospace\" font-size=\"{}\" fill=\"{}\">{}</text>",
                if *kind == Extra::Cite { "cite" } else { "event" }, f(cx), ly, f(MONO_FS), th.label, esc(l)
            )),
        }
    }
}

/// The derived overview: boxes in columns by the first number of the id, lines between them.
fn render_state(model: &Model) -> String {
    let fs = 11.0;
    let col_w = 230.0;
    let row_h = 26.0;
    let n_of = |t: &str| -> usize { t.trim_start_matches("FL-").split(|c: char| !c.is_ascii_digit()).next().and_then(|x| x.parse().ok()).unwrap_or(0) };
    let mut cols: Vec<usize> = model.nodes.iter().map(|n| n_of(&n.text)).collect();
    cols.sort();
    cols.dedup();
    let mut pos = std::collections::HashMap::new();
    let mut maxrows = 0;
    for (ci, c) in cols.iter().enumerate() {
        let mut r = 0;
        for n in model.nodes.iter().filter(|n| n_of(&n.text) == *c) {
            pos.insert(n.id.clone(), (MARGIN + ci as f64 * col_w, MARGIN + r as f64 * row_h));
            r += 1;
        }
        maxrows = maxrows.max(r);
    }
    let width = MARGIN * 2.0 + cols.len() as f64 * col_w;
    let height = MARGIN * 2.0 + maxrows as f64 * row_h;
    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\" font-family=\"{}\" data-diagram=\"{}\">\n<rect width=\"{}\" height=\"{}\" fill=\"#fff\"/>\n", f(width), f(height), f(width), f(height), "Helvetica, Arial, sans-serif", esc(&model.name), f(width), f(height));
    s.push_str("<g fill=\"none\" stroke=\"#9aa7b5\" stroke-width=\"0.7\" opacity=\"0.7\">\n");
    for e in &model.edges {
        if let (Some(a), Some(b)) = (pos.get(&e.from), pos.get(&e.to)) {
            let (x1, y1) = (a.0 + col_w - 30.0, a.1 + 10.0);
            let (x2, y2) = (b.0 + 4.0, b.1 + 10.0);
            let mx = (x1 + x2) / 2.0;
            s.push_str(&format!("<path d=\"M{} {} C{} {} {} {} {} {}\"/>\n", f(x1), f(y1), f(mx), f(y1), f(mx), f(y2), f(x2), f(y2)));
        }
    }
    s.push_str("</g>\n");
    for n in &model.nodes {
        let (x, y) = pos[&n.id];
        let w = col_w - 40.0;
        let mut t = n.text.clone();
        while text_width(&t, fs) > w - 12.0 && t.chars().count() > 4 {
            t.pop();
        }
        if t != n.text {
            t.push('…');
        }
        s.push_str(&format!("<g class=\"node state\" data-node=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"20\" rx=\"4\" fill=\"#eef2f6\" stroke=\"#8798ab\"/><text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#222\">{}<title>{}</title></text></g>\n", esc(&n.id), f(x), f(y), f(w), f(x + 6.0), f(y + 14.0), f(fs), esc(&t), esc(&n.text)));
    }
    s.push_str("</svg>\n");
    s
}
