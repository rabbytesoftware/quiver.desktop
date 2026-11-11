//! Plays SVG path data (`M L H V C S Q T A Z`, absolute and relative) into a GPUI [`PathBuilder`].

use gpui::{point, px, PathBuilder, Pixels, Point};

struct Scanner<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Scanner<'_> {
    fn skip_separators(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b',')
        {
            self.at += 1;
        }
    }

    fn command(&mut self) -> Option<u8> {
        self.skip_separators();
        let b = *self.bytes.get(self.at)?;
        b.is_ascii_alphabetic().then(|| {
            self.at += 1;
            b
        })
    }

    fn has_number(&mut self) -> bool {
        self.skip_separators();
        self.bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_digit() || matches!(b, b'-' | b'+' | b'.'))
    }

    fn number(&mut self) -> Option<f32> {
        self.skip_separators();
        let start = self.at;
        if matches!(self.bytes.get(self.at), Some(b'-' | b'+')) {
            self.at += 1;
        }
        let mut seen_dot = false;
        while let Some(b) = self.bytes.get(self.at) {
            match b {
                b'0'..=b'9' => self.at += 1,
                b'.' if !seen_dot => {
                    seen_dot = true;
                    self.at += 1;
                }
                b'e' | b'E' => {
                    self.at += 1;
                    if matches!(self.bytes.get(self.at), Some(b'-' | b'+')) {
                        self.at += 1;
                    }
                }
                _ => break,
            }
        }
        std::str::from_utf8(&self.bytes[start..self.at])
            .ok()?
            .parse()
            .ok()
    }

    fn flag(&mut self) -> Option<bool> {
        self.skip_separators();
        let b = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(b == b'1')
    }
}

/// Appends `d` to `builder`, mapping view-box units to pixels with `scale` and `origin`.
pub(crate) fn append(
    builder: &mut PathBuilder,
    d: &str,
    origin: Point<Pixels>,
    scale: f32,
) {
    let map = |x: f32, y: f32| point(origin.x + px(x * scale), origin.y + px(y * scale));
    let mut s = Scanner {
        bytes: d.as_bytes(),
        at: 0,
    };
    let (mut cx, mut cy) = (0.0_f32, 0.0_f32);
    let (mut sx, mut sy) = (0.0_f32, 0.0_f32);
    let mut last_cubic: Option<(f32, f32)> = None;
    let mut last_quad: Option<(f32, f32)> = None;
    let mut cmd = b'M';
    loop {
        if let Some(c) = s.command() {
            cmd = c;
            if matches!(cmd, b'Z' | b'z') {
                builder.close();
                cx = sx;
                cy = sy;
                last_cubic = None;
                last_quad = None;
                continue;
            }
        } else if !s.has_number() {
            break;
        }
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { (cx, cy) } else { (0.0, 0.0) };
        let mut next_cubic = None;
        let mut next_quad = None;
        match cmd.to_ascii_uppercase() {
            b'M' => {
                let (Some(x), Some(y)) = (s.number(), s.number()) else {
                    break;
                };
                (cx, cy) = (ox + x, oy + y);
                (sx, sy) = (cx, cy);
                builder.move_to(map(cx, cy));
                cmd = if rel { b'l' } else { b'L' };
            }
            b'L' => {
                let (Some(x), Some(y)) = (s.number(), s.number()) else {
                    break;
                };
                (cx, cy) = (ox + x, oy + y);
                builder.line_to(map(cx, cy));
            }
            b'H' => {
                let Some(x) = s.number() else { break };
                cx = ox + x;
                builder.line_to(map(cx, cy));
            }
            b'V' => {
                let Some(y) = s.number() else { break };
                cy = oy + y;
                builder.line_to(map(cx, cy));
            }
            b'C' | b'S' => {
                let (x1, y1) = if cmd.eq_ignore_ascii_case(&b'C') {
                    let (Some(a), Some(b)) = (s.number(), s.number()) else {
                        break;
                    };
                    (ox + a, oy + b)
                } else {
                    last_cubic.map_or((cx, cy), |(lx, ly)| (2.0 * cx - lx, 2.0 * cy - ly))
                };
                let (Some(a), Some(b), Some(c), Some(e)) =
                    (s.number(), s.number(), s.number(), s.number())
                else {
                    break;
                };
                let (x2, y2, x, y) = (ox + a, oy + b, ox + c, oy + e);
                builder.cubic_bezier_to(map(x, y), map(x1, y1), map(x2, y2));
                next_cubic = Some((x2, y2));
                (cx, cy) = (x, y);
            }
            b'Q' | b'T' => {
                let (x1, y1) = if cmd.eq_ignore_ascii_case(&b'Q') {
                    let (Some(a), Some(b)) = (s.number(), s.number()) else {
                        break;
                    };
                    (ox + a, oy + b)
                } else {
                    last_quad.map_or((cx, cy), |(lx, ly)| (2.0 * cx - lx, 2.0 * cy - ly))
                };
                let (Some(a), Some(b)) = (s.number(), s.number()) else {
                    break;
                };
                let (x, y) = (ox + a, oy + b);
                builder.curve_to(map(x, y), map(x1, y1));
                next_quad = Some((x1, y1));
                (cx, cy) = (x, y);
            }
            b'A' => {
                let (Some(rx), Some(ry), Some(rot)) = (s.number(), s.number(), s.number()) else {
                    break;
                };
                let (Some(large), Some(sweep)) = (s.flag(), s.flag()) else {
                    break;
                };
                let (Some(x), Some(y)) = (s.number(), s.number()) else {
                    break;
                };
                (cx, cy) = (ox + x, oy + y);
                builder.arc_to(
                    point(px(rx * scale), px(ry * scale)),
                    px(rot),
                    large,
                    sweep,
                    map(cx, cy),
                );
            }
            _ => break,
        }
        last_cubic = next_cubic;
        last_quad = next_quad;
    }
}
