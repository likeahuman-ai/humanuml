//! Text width without a browser: Helvetica advance widths (1000 units per em),
//! from the public Adobe core-14 font metrics. The SVG names the family
//! `Helvetica, Arial, sans-serif`; Arial has the same advance widths.

const ASCII: [u16; 95] = [278,278,355,556,556,889,667,191,333,333,389,584,278,333,278,278,556,556,556,556,556,556,556,556,556,556,278,278,584,584,584,556,1015,667,667,722,722,667,611,778,722,278,500,667,556,833,722,778,667,778,722,667,611,722,667,944,667,667,611,278,278,278,469,556,333,556,556,500,556,556,278,556,556,222,222,500,222,833,556,556,556,556,333,500,278,556,500,722,500,500,500,334,260,334,584];

/// Width of one character, in 1000ths of the font size.
pub fn char_width(c: char) -> f64 {
    let n = c as u32;
    if (32..=126).contains(&n) {
        return ASCII[(n - 32) as usize] as f64;
    }
    match c {
        '\u{00b7}' => 278.0,                       // · periodcentered
        '\u{203a}' | '\u{2039}' => 333.0,          // › ‹
        '\u{2014}' => 1000.0,                      // — emdash
        '\u{2013}' => 556.0,                       // – endash
        '\u{2018}' | '\u{2019}' => 222.0,          // ‘ ’
        '\u{201c}' | '\u{201d}' => 333.0,          // “ ”
        '\u{2026}' => 1000.0,                      // …
        '\u{2192}' | '\u{2190}' => 1000.0,         // → ←
        '\u{00a0}' => 278.0,
        '\u{00e0}'..='\u{00ff}' | '\u{00c0}'..='\u{00df}' => 556.0, // Latin-1 letters
        _ => 700.0,
    }
}

/// Width of `s` at `size` pixels.
pub fn text_width(s: &str, size: f64) -> f64 {
    s.chars().map(char_width).sum::<f64>() * size / 1000.0
}

/// Break `s` into lines no wider than `max` pixels. Long words are cut.
pub fn wrap(s: &str, size: f64, max: f64) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    for para in s.split('\n') {
        let mut cur = String::new();
        for word in para.split_whitespace() {
            let mut word = word.to_string();
            loop {
                let trial = if cur.is_empty() { word.clone() } else { format!("{} {}", cur, word) };
                if text_width(&trial, size) <= max {
                    cur = trial;
                    break;
                }
                if !cur.is_empty() {
                    lines.push(std::mem::take(&mut cur));
                    continue;
                }
                // one word wider than the line: cut it
                let chars: Vec<char> = word.chars().collect();
                let mut n = 1;
                while n < chars.len() && text_width(&chars[..n + 1].iter().collect::<String>(), size) <= max {
                    n += 1;
                }
                let cut: String = chars[..n].iter().collect();
                let rest: String = chars[n..].iter().collect();
                lines.push(cut);
                word = rest;
                if word.is_empty() {
                    break;
                }
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
