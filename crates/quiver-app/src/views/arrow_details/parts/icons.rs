//! The arrow page's pictures by role. Each one is a `quiver_ui::Glyph` carrying the exact Lucide path
//! the old arrow page (and the AFTER board) draws, so this enum only names what each spot shows.

use gpui::{Hsla, IntoElement};
use quiver_ui::{icon_with_stroke, Glyph};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pict {
    Play,
    Stop,
    Refresh,
    Info,
    Methods,
    Download,
    Trash,
    Plus,
    External,
    Eye,
    EyeOff,
    Alert,
}

impl Pict {
    fn glyph(self) -> Glyph {
        match self {
            Pict::Play => Glyph::Play,
            Pict::Stop => Glyph::Square,
            Pict::Refresh => Glyph::RefreshCw,
            Pict::Info => Glyph::Info,
            Pict::Methods => Glyph::List,
            Pict::Download => Glyph::Download,
            Pict::Trash => Glyph::Trash,
            Pict::Plus => Glyph::Plus,
            Pict::External => Glyph::ExternalLink,
            Pict::Eye => Glyph::Eye,
            Pict::EyeOff => Glyph::EyeOff,
            Pict::Alert => Glyph::TriangleAlert,
        }
    }
}

/// A `size`px square picture in `color`; stroke 1.8 of 24 unless the caller passes its own.
pub fn pict(
    pict: Pict,
    size: f32,
    color: Hsla,
) -> impl IntoElement {
    pict_with_stroke(pict, size, color, 1.8)
}

pub fn pict_with_stroke(
    pict: Pict,
    size: f32,
    color: Hsla,
    stroke: f32,
) -> impl IntoElement {
    icon_with_stroke(pict.glyph(), size, color, stroke)
}
