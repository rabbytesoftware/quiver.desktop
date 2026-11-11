//! `TrackedText`: one line of text with CSS `letter-spacing`. gpui 0.2.2 has no such property (neither
//! `TextStyle` nor `TextRun` carries one), so the line is shaped as usual, which keeps kerning, and
//! every glyph is painted `tracking` further along than the shaper placed it. A line wider than its box
//! ends in an ellipsis, as `text-overflow: ellipsis` does. With [`TrackedText::wrap`] the text instead
//! wraps at spaces onto as many lines as its box needs (CSS `white-space: normal`).

use gpui::prelude::*;
use gpui::{
    canvas, div, point, px, size, App, AvailableSpace, Bounds, Element, ElementId, FontId,
    FontWeight, GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels,
    ShapedGlyph, SharedString, Style, TextStyle, Window,
};

#[derive(IntoElement, Clone)]
pub struct TrackedText {
    text: SharedString,
    size: f32,
    line_height: f32,
    tracking: f32,
    color: Option<Hsla>,
    weight: FontWeight,
    family: Option<SharedString>,
    wrap: bool,
    max_lines: Option<usize>,
}

impl TrackedText {
    /// `size` and `line_height` in px; `tracking` in px (CSS `letter-spacing`, em values multiplied out).
    pub fn new(
        text: impl Into<SharedString>,
        size: f32,
        line_height: f32,
        tracking: f32,
        color: Hsla,
    ) -> Self {
        Self {
            text: text.into(),
            size,
            line_height,
            tracking,
            color: Some(color),
            weight: FontWeight::NORMAL,
            family: None,
            wrap: false,
            max_lines: None,
        }
    }

    /// Like [`TrackedText::new`], but painted in the inherited text colour, so a parent's hover colour
    /// applies (as CSS `color` inheritance does).
    pub fn inherit_color(
        text: impl Into<SharedString>,
        size: f32,
        line_height: f32,
        tracking: f32,
    ) -> Self {
        Self {
            text: text.into(),
            size,
            line_height,
            tracking,
            color: None,
            weight: FontWeight::NORMAL,
            family: None,
            wrap: false,
            max_lines: None,
        }
    }

    pub fn weight(
        mut self,
        weight: FontWeight,
    ) -> Self {
        self.weight = weight;
        self
    }

    pub fn family(
        mut self,
        family: impl Into<SharedString>,
    ) -> Self {
        self.family = Some(family.into());
        self
    }

    /// Wrap at spaces instead of ending in an ellipsis.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Wrap, keeping at most `lines` lines; a longer text ends its last one in "…" (CSS `line-clamp`).
    /// gpui's own `line_clamp` budgets the ellipsis against the lines' summed width, so a short first
    /// line lets the last one run past the box.
    pub fn max_lines(
        mut self,
        lines: usize,
    ) -> Self {
        self.wrap = true;
        self.max_lines = Some(lines.max(1));
        self
    }
}

impl RenderOnce for TrackedText {
    fn render(
        self,
        window: &mut Window,
        _: &mut App,
    ) -> impl IntoElement {
        if self.wrap {
            return Wrapped(self).into_any_element();
        }
        let height = px(self.line_height);
        // As wide as the tracked line (CSS adds the spacing after every glyph), shrinking to an ellipsis
        // when the parent is narrower.
        let width = self.natural_width(window);
        div()
            .relative()
            .flex_shrink()
            .min_w(px(0.))
            .w(width)
            .max_w_full()
            .h(height)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, cx| self.paint(bounds, window, cx),
                )
                .size_full(),
            )
            .into_any_element()
    }
}

impl TrackedText {
    fn style(
        &self,
        window: &Window,
    ) -> gpui::TextStyle {
        let mut style = window.text_style();
        if let Some(family) = &self.family {
            style.font_family = family.clone();
        }
        style.font_weight = self.weight;
        if let Some(color) = self.color {
            style.color = color;
        }
        style
    }

    fn natural_width(
        &self,
        window: &Window,
    ) -> Pixels {
        let style = self.style(window);
        let len = self.text.len();
        let line = window.text_system().shape_line(
            self.text.clone(),
            px(self.size),
            &[style.to_run(len)],
            None,
        );
        let glyphs: usize = line.runs.iter().map(|run| run.glyphs.len()).sum();
        let width = line.width + px(self.tracking) * glyphs as f32;
        if width > px(0.) {
            width.ceil()
        } else {
            px(0.)
        }
    }

    fn paint(
        &self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _: &mut App,
    ) {
        let style = self.style(window);
        let font_size = px(self.size);
        let shape = |window: &Window, text: SharedString| {
            let len = text.len();
            window
                .text_system()
                .shape_line(text, font_size, &[style.to_run(len)], None)
        };
        let line = shape(window, self.text.clone());
        let ellipsis = shape(window, "…".into());
        let tracking = px(self.tracking);
        let glyphs: Vec<_> = line
            .runs
            .iter()
            .flat_map(|run| run.glyphs.iter().map(move |g| (run.font_id, g)))
            .collect();
        let advance_to = |ix: usize| {
            glyphs.get(ix).map_or(line.width, |(_, g)| g.position.x) + tracking * ix as f32
        };
        let total = advance_to(glyphs.len());
        let keep = if total <= bounds.size.width {
            glyphs.len()
        } else {
            let room = bounds.size.width - ellipsis.width;
            (0..glyphs.len())
                .rev()
                .find(|&n| advance_to(n) <= room)
                .unwrap_or(0)
        };
        let baseline = (px(self.line_height) - line.ascent - line.descent) / 2. + line.ascent;
        let origin = bounds.origin;
        for (ix, (font_id, glyph)) in glyphs.iter().take(keep).enumerate() {
            if glyph.is_emoji {
                continue;
            }
            window
                .paint_glyph(
                    point(
                        origin.x + glyph.position.x + tracking * ix as f32,
                        origin.y + baseline,
                    ),
                    *font_id,
                    glyph.id,
                    font_size,
                    style.color,
                )
                .ok();
        }
        if keep < glyphs.len() {
            let x = origin.x + advance_to(keep);
            for run in &ellipsis.runs {
                for glyph in &run.glyphs {
                    window
                        .paint_glyph(
                            point(x + glyph.position.x, origin.y + baseline),
                            run.font_id,
                            glyph.id,
                            font_size,
                            style.color,
                        )
                        .ok();
                }
            }
        }
    }
}

/// One shaped line's glyphs with their tracked x positions, broken into lines at spaces.
struct Glyphs {
    glyphs: Vec<(FontId, ShapedGlyph)>,
    /// `advance[i]`: where glyph `i` starts once tracked; `advance[n]` is the tracked line's width.
    advance: Vec<Pixels>,
    /// Glyph ranges between spaces.
    words: Vec<(usize, usize)>,
    /// Which glyphs are spaces.
    spaces: Vec<bool>,
    ascent: Pixels,
    descent: Pixels,
}

impl Glyphs {
    fn shape(
        text: &TrackedText,
        style: &TextStyle,
        window: &Window,
    ) -> Self {
        let line = window.text_system().shape_line(
            text.text.clone(),
            px(text.size),
            &[style.to_run(text.text.len())],
            None,
        );
        let glyphs: Vec<_> = line
            .runs
            .iter()
            .flat_map(|run| run.glyphs.iter().map(move |g| (run.font_id, g.clone())))
            .collect();
        let tracking = px(text.tracking);
        let mut advance: Vec<Pixels> = glyphs
            .iter()
            .enumerate()
            .map(|(ix, (_, g))| g.position.x + tracking * ix as f32)
            .collect();
        advance.push(line.width + tracking * glyphs.len() as f32);
        let is_space =
            |g: &ShapedGlyph| text.text.get(g.index..).is_some_and(|t| t.starts_with(' '));
        let spaces: Vec<bool> = glyphs.iter().map(|(_, g)| is_space(g)).collect();
        let mut words = Vec::new();
        let mut start = None;
        for (ix, (_, g)) in glyphs.iter().enumerate() {
            match (is_space(g), start) {
                (true, Some(s)) => {
                    words.push((s, ix));
                    start = None;
                }
                (false, None) => start = Some(ix),
                _ => {}
            }
        }
        if let Some(s) = start {
            words.push((s, glyphs.len()));
        }
        Self {
            glyphs,
            advance,
            words,
            spaces,
            ascent: line.ascent,
            descent: line.descent,
        }
    }

    fn space(
        &self,
        ix: usize,
    ) -> bool {
        self.spaces.get(ix).copied().unwrap_or(false)
    }

    fn span(
        &self,
        (from, to): (usize, usize),
    ) -> Pixels {
        self.advance[to] - self.advance[from]
    }

    /// Greedy lines within `width` (`None`: one line). A word wider than the box overflows it, as CSS does.
    fn lines(
        &self,
        width: Option<Pixels>,
    ) -> Vec<(usize, usize)> {
        let mut lines: Vec<(usize, usize)> = Vec::new();
        for &word in &self.words {
            match lines.last_mut() {
                Some(line) if width.is_none_or(|w| self.span((line.0, word.1)) <= w) => {
                    line.1 = word.1;
                }
                _ => lines.push(word),
            }
        }
        lines
    }

    fn widest(
        &self,
        lines: &[(usize, usize)],
    ) -> Pixels {
        lines
            .iter()
            .map(|&l| self.span(l))
            .fold(px(0.), |a, b| if b > a { b } else { a })
    }
}

/// [`TrackedText::wrap`]: measured by the layout engine like gpui's own text, so the box gets the height
/// its lines need.
struct Wrapped(TrackedText);

impl IntoElement for Wrapped {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Wrapped {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let text = self.0.clone();
        let style = text.style(window);
        let line_height = px(text.line_height);
        let layout_id =
            window.request_measured_layout(Style::default(), move |known, available, window, _| {
                let glyphs = Glyphs::shape(&text, &style, window);
                let lines = match (known.width, available.width) {
                    (Some(w), _) | (None, AvailableSpace::Definite(w)) => glyphs.lines(Some(w)),
                    (None, AvailableSpace::MinContent) => glyphs.lines(Some(px(0.))),
                    (None, AvailableSpace::MaxContent) => glyphs.lines(None),
                };
                let width = known.width.unwrap_or_else(|| glyphs.widest(&lines));
                let rows = text.max_lines.map_or(lines.len(), |n| lines.len().min(n));
                size(width, line_height * rows.max(1) as f32)
            });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        let text = &self.0;
        let style = text.style(window);
        let glyphs = Glyphs::shape(text, &style, window);
        let line_height = px(text.line_height);
        let baseline = (line_height - glyphs.ascent - glyphs.descent) / 2. + glyphs.ascent;
        // A hair of slack so rounding in the measured width never re-wraps the last word.
        let width = bounds.size.width + px(0.5);
        let mut lines = glyphs.lines(Some(width));
        let mut ellipsis_at = None;
        if let Some(max) = text.max_lines.filter(|&n| lines.len() > n) {
            lines.truncate(max);
            // The last kept line runs on into the rest of the text, cut where "…" still fits.
            let ellipsis = window.text_system().shape_line(
                "…".into(),
                px(text.size),
                &[style.to_run("…".len())],
                None,
            );
            let from = lines[max - 1].0;
            let room = width - ellipsis.width;
            let end = (from..=glyphs.glyphs.len())
                .rev()
                .find(|&e| glyphs.span((from, e)) <= room)
                .unwrap_or(from);
            // No trailing space before the ellipsis, as the browser draws it.
            let end = (from..end)
                .rev()
                .find(|&e| !glyphs.space(e))
                .map_or(from, |e| e + 1);
            lines[max - 1].1 = end;
            ellipsis_at = Some((max - 1, glyphs.span((from, end)), ellipsis));
        }
        for (row, (from, to)) in lines.into_iter().enumerate() {
            let y = bounds.origin.y + line_height * row as f32 + baseline;
            if let Some((_, x, ellipsis)) = ellipsis_at.as_ref().filter(|(r, _, _)| *r == row) {
                for run in &ellipsis.runs {
                    for glyph in &run.glyphs {
                        window
                            .paint_glyph(
                                point(bounds.origin.x + *x + glyph.position.x, y),
                                run.font_id,
                                glyph.id,
                                px(text.size),
                                style.color,
                            )
                            .ok();
                    }
                }
            }
            for ix in from..to {
                let (font_id, glyph) = &glyphs.glyphs[ix];
                if glyph.is_emoji {
                    continue;
                }
                let x = bounds.origin.x + glyphs.advance[ix] - glyphs.advance[from];
                window
                    .paint_glyph(point(x, y), *font_id, glyph.id, px(text.size), style.color)
                    .ok();
            }
        }
    }
}
