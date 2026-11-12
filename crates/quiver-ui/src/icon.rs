//! The glyphs the components need: Lucide (24 box, stroked) and Phosphor (256 box, filled), drawn from the
//! same path data the old app's icon packages ship (GPUI draws no SVG without assets, so they are tessellated).

use gpui::prelude::*;
use gpui::StrokeOptions;
use gpui::{
    canvas, px, App, Bounds, FillOptions, FillRule, Hsla, IntoElement, PathBuilder, PathStyle,
    Pixels, Window,
};
use lyon_tessellation::{LineCap, LineJoin};

use crate::error::{Error, Result};
use crate::icon_paths as paths;
use crate::svg_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Glyph {
    Check,
    ChevronDown,
    ChevronUp,
    ChevronsUpDown,
    Close,
    Search,
    Home,
    HomeFill,
    Server,
    ServerFill,
    Settings,
    SettingsFill,
    Play,
    Terminal,
    ArrowLeft,
    ArrowRight,
    Ellipsis,
    EllipsisVertical,
    Plus,
    Globe,
    TriangleAlert,
    ArrowRightLeft,
    Pencil,
    Trash,
    Info,
    RefreshCw,
    Square,
    Sparkles,
    Monitor,
    Code,
    Eye,
    EyeOff,
    Cpu,
    HardDrive,
    Link,
    MemoryStick,
    ArrowUp,
    Circle,
    Package,
    Radio,
    Box,
    CircleHelp,
    Download,
    ExternalLink,
    List,
    PackageOpen,
    Usb,
    Loader,
    PhArrowLeft,
    PhArrowCounterClockwise,
    PhMagnifyingGlass,
    PhPackage,
    /// The search bar's own 13px magnifier (13 box, stroke 1.68).
    SearchField,
    /// The old search card's 5-point star (its stars count), filled.
    Star,
    /// The Quiver arrow mark (an up chevron over a dot), as on the tile art.
    Mark,
    /// The same mark with the proportions of the arrow icon (a 512 box: chevron 144..368, dot at 392).
    ArrowIcon,
}

impl Glyph {
    /// Every glyph, for the gallery.
    pub const ALL: &[Glyph] = &[
        Glyph::Check,
        Glyph::ChevronDown,
        Glyph::ChevronUp,
        Glyph::ChevronsUpDown,
        Glyph::Close,
        Glyph::Search,
        Glyph::Home,
        Glyph::HomeFill,
        Glyph::Server,
        Glyph::ServerFill,
        Glyph::Settings,
        Glyph::SettingsFill,
        Glyph::Play,
        Glyph::Terminal,
        Glyph::ArrowLeft,
        Glyph::ArrowRight,
        Glyph::Ellipsis,
        Glyph::EllipsisVertical,
        Glyph::Plus,
        Glyph::Globe,
        Glyph::TriangleAlert,
        Glyph::ArrowRightLeft,
        Glyph::Pencil,
        Glyph::Trash,
        Glyph::Info,
        Glyph::RefreshCw,
        Glyph::Square,
        Glyph::Sparkles,
        Glyph::Monitor,
        Glyph::Code,
        Glyph::Eye,
        Glyph::EyeOff,
        Glyph::Cpu,
        Glyph::HardDrive,
        Glyph::Link,
        Glyph::MemoryStick,
        Glyph::ArrowUp,
        Glyph::Circle,
        Glyph::Package,
        Glyph::Radio,
        Glyph::Box,
        Glyph::CircleHelp,
        Glyph::Download,
        Glyph::ExternalLink,
        Glyph::List,
        Glyph::PackageOpen,
        Glyph::Usb,
        Glyph::Loader,
        Glyph::PhArrowLeft,
        Glyph::PhArrowCounterClockwise,
        Glyph::PhMagnifyingGlass,
        Glyph::PhPackage,
        Glyph::SearchField,
        Glyph::Star,
        Glyph::Mark,
        Glyph::ArrowIcon,
    ];
}

#[derive(Clone, Copy)]
enum Paint {
    Stroke,
    Fill,
    Stroke13,
}

const SEARCH_FIELD: &[&str] = &[
    "M9.48 5.35a4.13 4.13 0 1 0-8.26 0a4.13 4.13 0 1 0 8.26 0Z",
    "M8.56 8.56l3.37 3.37",
];
/// `M6 1l1.4 3.1 …` of the old card's 12px star, scaled to the 256 box of the filled glyphs.
const STAR: &[&str] = &["M128 21.33l29.87 66.13 72.53 8.53 -53.33 49.07 14.93 70.4L128 181.33 64 215.47l14.93 -70.4L25.6 96l72.53 -8.53z"];
const MARK: &[&str] = &[
    "M5.5 15.2L12 4.5L18.5 15.2",
    "M11.3 19.6a.7.7 0 1 0 1.4 0a.7.7 0 1 0-1.4 0Z",
];
const ARROW_ICON: &[&str] = &[
    "M6.75 17.25L12 7.125L17.25 17.25",
    "M11.2 18.375a.8.8 0 1 0 1.6 0a.8.8 0 1 0-1.6 0Z",
];

fn shape(glyph: Glyph) -> (Paint, &'static [&'static str]) {
    match glyph {
        Glyph::Check => (Paint::Stroke, paths::CHECK),
        Glyph::ChevronDown => (Paint::Stroke, paths::CHEVRONDOWN),
        Glyph::ChevronUp => (Paint::Stroke, paths::CHEVRONUP),
        Glyph::ChevronsUpDown => (Paint::Stroke, paths::CHEVRONSUPDOWN),
        Glyph::Close => (Paint::Stroke, paths::CLOSE),
        Glyph::Search => (Paint::Stroke, paths::SEARCH),
        Glyph::Home => (Paint::Fill, paths::HOME),
        Glyph::HomeFill => (Paint::Fill, paths::HOMEFILL),
        Glyph::Server => (Paint::Fill, paths::SERVER),
        Glyph::ServerFill => (Paint::Fill, paths::SERVERFILL),
        Glyph::Settings => (Paint::Fill, paths::SETTINGS),
        Glyph::SettingsFill => (Paint::Fill, paths::SETTINGSFILL),
        Glyph::Play => (Paint::Stroke, paths::PLAY),
        Glyph::Terminal => (Paint::Stroke, paths::TERMINAL),
        Glyph::ArrowLeft => (Paint::Stroke, paths::ARROWLEFT),
        Glyph::ArrowRight => (Paint::Stroke, paths::ARROWRIGHT),
        Glyph::Ellipsis => (Paint::Stroke, paths::ELLIPSIS),
        Glyph::EllipsisVertical => (Paint::Stroke, paths::ELLIPSISVERTICAL),
        Glyph::Plus => (Paint::Stroke, paths::PLUS),
        Glyph::Globe => (Paint::Stroke, paths::GLOBE),
        Glyph::TriangleAlert => (Paint::Stroke, paths::TRIANGLEALERT),
        Glyph::ArrowRightLeft => (Paint::Stroke, paths::ARROWRIGHTLEFT),
        Glyph::Pencil => (Paint::Stroke, paths::PENCIL),
        Glyph::Trash => (Paint::Stroke, paths::TRASH),
        Glyph::Info => (Paint::Stroke, paths::INFO),
        Glyph::RefreshCw => (Paint::Stroke, paths::REFRESHCW),
        Glyph::Square => (Paint::Stroke, paths::SQUARE),
        Glyph::Sparkles => (Paint::Stroke, paths::SPARKLES),
        Glyph::Monitor => (Paint::Stroke, paths::MONITOR),
        Glyph::Code => (Paint::Stroke, paths::CODE),
        Glyph::Eye => (Paint::Stroke, paths::EYE),
        Glyph::EyeOff => (Paint::Stroke, paths::EYEOFF),
        Glyph::Cpu => (Paint::Stroke, paths::CPU),
        Glyph::HardDrive => (Paint::Stroke, paths::HARDDRIVE),
        Glyph::Link => (Paint::Stroke, paths::LINK),
        Glyph::MemoryStick => (Paint::Stroke, paths::MEMORYSTICK),
        Glyph::ArrowUp => (Paint::Stroke, paths::ARROWUP),
        Glyph::Circle => (Paint::Stroke, paths::CIRCLE),
        Glyph::Package => (Paint::Stroke, paths::PACKAGE),
        Glyph::Radio => (Paint::Stroke, paths::RADIO),
        Glyph::Box => (Paint::Stroke, paths::BOX),
        Glyph::CircleHelp => (Paint::Stroke, paths::CIRCLEHELP),
        Glyph::Download => (Paint::Stroke, paths::DOWNLOAD),
        Glyph::ExternalLink => (Paint::Stroke, paths::EXTERNALLINK),
        Glyph::List => (Paint::Stroke, paths::LIST),
        Glyph::PackageOpen => (Paint::Stroke, paths::PACKAGEOPEN),
        Glyph::Usb => (Paint::Stroke, paths::USB),
        Glyph::Loader => (Paint::Stroke, paths::LOADER),
        Glyph::PhArrowLeft => (Paint::Fill, paths::PHARROWLEFT),
        Glyph::PhArrowCounterClockwise => (Paint::Fill, paths::PHARROWCOUNTERCLOCKWISE),
        Glyph::PhMagnifyingGlass => (Paint::Fill, paths::PHMAGNIFYINGGLASS),
        Glyph::PhPackage => (Paint::Fill, paths::PHPACKAGE),
        Glyph::SearchField => (Paint::Stroke13, SEARCH_FIELD),
        Glyph::Star => (Paint::Fill, STAR),
        Glyph::Mark => (Paint::Stroke, MARK),
        Glyph::ArrowIcon => (Paint::Stroke, ARROW_ICON),
    }
}

fn view_box(paint: Paint) -> f32 {
    match paint {
        Paint::Stroke => 24.0,
        Paint::Fill => 256.0,
        Paint::Stroke13 => 13.0,
    }
}

/// A stroke builder with the round caps and joins every Lucide icon is drawn with.
pub(crate) fn round_stroke(width: f32) -> PathBuilder {
    PathBuilder::stroke(px(width)).with_style(PathStyle::Stroke(
        StrokeOptions::default()
            .with_line_width(width)
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round),
    ))
}

fn paint_shape(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    glyph: Glyph,
    size: f32,
    stroke: f32,
    color: Hsla,
) -> Result<()> {
    // The tessellation is the expensive part and depends on everything but the colour: build it once per
    // (glyph, size, stroke, origin) and repaint the stored paths on every later frame.
    let key = (
        glyph,
        size.to_bits(),
        stroke.to_bits(),
        f32::from(bounds.origin.x).to_bits(),
        f32::from(bounds.origin.y).to_bits(),
    );
    let paths = crate::path_cache::cached(key, || {
        let (paint, ds) = shape(glyph);
        let k = size / view_box(paint);
        ds.iter()
            .map(|d| {
                let mut b = match paint {
                    Paint::Stroke | Paint::Stroke13 => round_stroke(stroke * k),
                    Paint::Fill => PathBuilder::fill().with_style(PathStyle::Fill(
                        FillOptions::default().with_fill_rule(FillRule::NonZero),
                    )),
                };
                svg_path::append(&mut b, d, bounds.origin, k);
                b.build().map_err(|e| Error::Path(e.to_string()))
            })
            .collect()
    })?;
    for path in paths.iter() {
        window.paint_path(path.clone(), color);
    }
    Ok(())
}

/// A `size`px square glyph, stroke 2 (scaled), in `color`.
pub fn icon(
    glyph: Glyph,
    size: f32,
    color: Hsla,
) -> impl IntoElement {
    icon_with_stroke(glyph, size, color, 2.0)
}

/// [`icon`] with an explicit stroke width in the 24px view box (ignored by the filled Phosphor glyphs).
pub fn icon_with_stroke(
    glyph: Glyph,
    size: f32,
    color: Hsla,
    stroke: f32,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window: &mut Window, _: &mut App| {
            // A failed tessellation only drops one glyph for a frame; nothing to recover.
            paint_shape(window, bounds, glyph, size, stroke, color).ok();
        },
    )
    .size(px(size))
    .flex_none()
}
