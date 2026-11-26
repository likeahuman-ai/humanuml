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

const FONT: &str = "Helvetica, Arial, sans-serif";
const MARGIN: f64 = 24.0;
const LANE_PAD: f64 = 22.0;
const HEADER_H: f64 = 30.0;

fn f(x: f64) -> String {
    let s = format!("{:.1}", x);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn render(model: &Model, d: &Diagram) -> String {
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
    for l in 0..nl {
        let (el, er) = root.ext[l].unwrap_or((40.0, 40.0));
        let (el, er) = (el.max(20.0), er.max(20.0));
        let start = x;
        centers[l] = x + LANE_PAD + el;
        x = centers[l] + er + LANE_PAD;
        bands[l] = (start, x);
    }
    let has_lanes = !laid.lanes.is_empty();
    let width = x + MARGIN;
    let top = MARGIN + if has_lanes { HEADER_H } else { 0.0 } + 14.0;
    let height = top + root.h + MARGIN + 6.0;
    let px = |p: &Port| centers[p.lane] + p.dx;
    let py = |y: f64| top + y;

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\" font-family=\"{}\" data-diagram=\"{}\">\n",
        f(width), f(height), f(width), f(height), FONT, esc(&model.name)
    ));
    s.push_str("<defs><marker id=\"arr\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M0 1 L10 5 L0 9 z\" fill=\"#5b5b5b\"/></marker></defs>\n");
    s.push_str(&format!("<rect width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>\n", f(width), f(height)));
    if has_lanes {
        for (i, name) in laid.lanes.iter().enumerate() {
            let (a, b) = bands[i];
            s.push_str(&format!(
                "<g class=\"lane\" data-lane=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#c9c9c9\"/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ececec\" stroke=\"#c9c9c9\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"13\" font-weight=\"bold\" fill=\"#333\">{}</text></g>\n",
                esc(name), f(a), f(MARGIN), f(b - a), f(height - 2.0 * MARGIN + 6.0), if i % 2 == 0 { "#fbfbfb" } else { "#f5f7fa" },
                f(a), f(MARGIN), f(b - a), f(HEADER_H), f((a + b) / 2.0), f(MARGIN + 20.0), esc(name)
            ));
        }
    }
    // edges
    s.push_str("<g fill=\"none\" stroke=\"#5b5b5b\" stroke-width=\"1.2\">\n");
    let mk = |arrow: bool| if arrow { " marker-end=\"url(#arr)\"" } else { "" };
    for e in &root.edges {
        match e {
            Edge::Flow { from, to, ymid, arrow } => {
                let (x1, y1, x2, y2) = (px(from), py(from.y), px(to), py(to.y));
                if (x1 - x2).abs() < 0.5 {
                    s.push_str(&format!("<path d=\"M{} {} V{}\"{}/>\n", f(x1), f(y1), f(y2), mk(*arrow)));
                } else {
                    let ym = ymid.map(py).unwrap_or((y1 + y2) / 2.0);
                    s.push_str(&format!("<path d=\"M{} {} V{} H{} V{}\"{}/>\n", f(x1), f(y1), f(ym), f(x2), f(y2), mk(*arrow)));
                }
            }
            Edge::Fan { dec, dw, dh, to, bus_y, arrow } => {
                let (dx, dy) = (px(dec), py(dec.y));
                let (tx, ty) = (px(to), py(to.y));
                let d = if let Some(by) = bus_y {
                    let by = py(*by);
                    if (tx - dx).abs() < 0.5 {
                        format!("M{} {} V{}", f(dx), f(dy + dh), f(ty))
                    } else {
                        format!("M{} {} V{} H{} V{}", f(dx), f(dy + dh), f(by), f(tx), f(ty))
                    }
                } else if (tx - dx).abs() < 0.5 {
                    format!("M{} {} V{}", f(dx), f(dy + dh), f(ty))
                } else if tx > dx {
                    format!("M{} {} H{} V{}", f(dx + dw / 2.0), f(dy + dh / 2.0), f(tx), f(ty))
                } else {
                    format!("M{} {} H{} V{}", f(dx - dw / 2.0), f(dy + dh / 2.0), f(tx), f(ty))
                };
                s.push_str(&format!("<path d=\"{}\"{}/>\n", d, mk(*arrow)));
            }
            Edge::Poly { pts, arrow } => {
                let mut d = String::new();
                for (i, p) in pts.iter().enumerate() {
                    d.push_str(&format!("{}{} {} ", if i == 0 { "M" } else { "L" }, f(px(p)), f(py(p.y))));
                }
                s.push_str(&format!("<path d=\"{}\"{}/>\n", d.trim_end(), mk(*arrow)));
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
            s.push_str(&format!("<g class=\"node bar\"{}><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"4\" fill=\"#333\"/></g>\n", attr, f(a), f(y - 2.0), f(b - a)));
        }
    }
    // items
    for it in &root.items {
        let cx = centers[it.lane] + it.dx;
        let y = py(it.y);
        let attr = it.id.as_ref().map(|i| format!(" data-node=\"{}\"", esc(i))).unwrap_or_default();
        match it.shape {
            Shape::Start => s.push_str(&format!("<g class=\"node start\"{}><circle cx=\"{}\" cy=\"{}\" r=\"8\" fill=\"#222\"/></g>\n", attr, f(cx), f(y + 8.0))),
            Shape::Stop => s.push_str(&format!("<g class=\"node stop\"{}><circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"#fff\" stroke=\"#222\" stroke-width=\"1.5\"/><circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"#222\"/></g>\n", attr, f(cx), f(y + 11.0), f(cx), f(y + 11.0))),
            Shape::End => s.push_str(&format!("<g class=\"node end\"{}><circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"#fff\" stroke=\"#222\" stroke-width=\"1.5\"/><path d=\"M{} {} l8 8 M{} {} l-8 8\" stroke=\"#222\" stroke-width=\"1.8\"/></g>\n", attr, f(cx), f(y + 11.0), f(cx - 4.0), f(y + 7.0), f(cx + 4.0), f(y + 7.0))),
            Shape::Merge => s.push_str(&format!("<polygon class=\"merge\" points=\"{},{} {},{} {},{} {},{}\" fill=\"#fff\" stroke=\"#5b5b5b\" stroke-width=\"1.2\"/>\n", f(cx), f(y), f(cx + 6.0), f(y + 6.0), f(cx), f(y + 12.0), f(cx - 6.0), f(y + 6.0))),
            Shape::Action | Shape::BackAction => {
                let dash = if it.shape == Shape::BackAction { " stroke-dasharray=\"4 3\"" } else { "" };
                let cls = if it.shape == Shape::BackAction { "node action backward" } else { "node action" };
                s.push_str(&format!("<g class=\"{}\"{}><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"7\" fill=\"#f6f2e7\" stroke=\"#8a7f66\" stroke-width=\"1.2\"{}/>", cls, attr, f(cx - it.w / 2.0), f(y), f(it.w), f(it.h), dash));
                text_block(&mut s, &it.lines, cx, y, it.h);
                s.push_str("</g>\n");
            }
            Shape::Decision => {
                let (w, h) = (it.w, it.h);
                let sl = 22.0f64.min(w / 3.0);
                let (x0, x1) = (cx - w / 2.0, cx + w / 2.0);
                s.push_str(&format!(
                    "<g class=\"node decision\"{}><polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"#e7eef6\" stroke=\"#56708c\" stroke-width=\"1.2\"/>",
                    attr, f(x0 + sl), f(y), f(x1 - sl), f(y), f(x1), f(y + h / 2.0), f(x1 - sl), f(y + h), f(x0 + sl), f(y + h), f(x0), f(y + h / 2.0)
                ));
                text_block(&mut s, &it.lines, cx, y, h);
                s.push_str("</g>\n");
            }
            Shape::Label => {
                s.push_str(&format!("<text class=\"label\" font-size=\"{}\" fill=\"#444\">", f(LFS)));
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

fn text_block(s: &mut String, lines: &[String], cx: f64, y: f64, h: f64) {
    let n = lines.len() as f64;
    let y0 = y + (h - n * LH) / 2.0;
    s.push_str(&format!("<text text-anchor=\"middle\" font-size=\"{}\" fill=\"#1f1f1f\">", f(FS)));
    for (i, l) in lines.iter().enumerate() {
        s.push_str(&format!("<tspan x=\"{}\" y=\"{}\">{}</tspan>", f(cx), f(y0 + i as f64 * LH + 11.0), esc(l)));
    }
    s.push_str("</text>");
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
    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\" font-family=\"{}\" data-diagram=\"{}\">\n<rect width=\"{}\" height=\"{}\" fill=\"#fff\"/>\n", f(width), f(height), f(width), f(height), FONT, esc(&model.name), f(width), f(height));
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
