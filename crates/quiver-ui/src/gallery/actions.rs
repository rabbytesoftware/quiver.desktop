//! Gallery page: buttons, badges, spinners and status glyphs.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App};

use super::layout::{gid, row, section};
use crate::{
    icon, ActiveQuiver, Badge, BadgeSize, BadgeVariant, Button, ButtonSize, ButtonVariant,
    FlickerSpinner, Glyph, GlyphKind, Spinner, StatusGlyph,
};

const VARIANTS: [(&str, ButtonVariant); 7] = [
    ("default", ButtonVariant::Default),
    ("destructive", ButtonVariant::Destructive),
    ("destr.-outline", ButtonVariant::DestructiveOutline),
    ("ghost", ButtonVariant::Ghost),
    ("link", ButtonVariant::Link),
    ("outline", ButtonVariant::Outline),
    ("secondary", ButtonVariant::Secondary),
];

const SIZES: [(&str, ButtonSize); 7] = [
    ("xs", ButtonSize::Xs),
    ("sm", ButtonSize::Sm),
    ("default", ButtonSize::Default),
    ("lg", ButtonSize::Lg),
    ("xl", ButtonSize::Xl),
    ("icon-xs", ButtonSize::IconXs),
    ("icon", ButtonSize::Icon),
];

const BADGES: [(&str, BadgeVariant); 8] = [
    ("default", BadgeVariant::Default),
    ("secondary", BadgeVariant::Secondary),
    ("outline", BadgeVariant::Outline),
    ("destructive", BadgeVariant::Destructive),
    ("error", BadgeVariant::Error),
    ("info", BadgeVariant::Info),
    ("success", BadgeVariant::Success),
    ("warning", BadgeVariant::Warning),
];

pub fn page(cx: &App) -> Vec<AnyElement> {
    let p = cx.quiver().palette;
    let variants = VARIANTS
        .iter()
        .map(|(n, v)| {
            Button::new(gid(format!("g-btn-{n}")))
                .variant(*v)
                .label(*n)
                .into_any_element()
        })
        .collect();
    let disabled = VARIANTS
        .iter()
        .map(|(n, v)| {
            Button::new(gid(format!("g-btn-dis-{n}")))
                .variant(*v)
                .disabled(true)
                .label(*n)
                .into_any_element()
        })
        .collect();
    let loading = VARIANTS
        .iter()
        .map(|(n, v)| {
            Button::new(gid(format!("g-btn-load-{n}")))
                .variant(*v)
                .loading(true)
                .label(*n)
                .into_any_element()
        })
        .collect();
    let sizes = SIZES
        .iter()
        .map(|(n, s)| {
            let b = Button::new(gid(format!("g-btn-size-{n}")))
                .variant(ButtonVariant::Outline)
                .size(*s);
            (if n.starts_with("icon") {
                b.child(icon(Glyph::Close, 14., p.foreground))
            } else {
                b.label(*n)
            })
            .into_any_element()
        })
        .collect();
    let badges = BADGES
        .iter()
        .map(|(n, v)| Badge::new().variant(*v).label(*n).into_any_element())
        .collect();
    let badge_sizes = [
        ("sm", BadgeSize::Sm),
        ("default", BadgeSize::Default),
        ("lg", BadgeSize::Lg),
    ]
    .iter()
    .map(|(n, s)| {
        Badge::new()
            .variant(BadgeVariant::Secondary)
            .size(*s)
            .label(*n)
            .into_any_element()
    })
    .collect();
    let glyphs = [
        ("running", GlyphKind::Running),
        ("attention", GlyphKind::NeedsAttention),
        ("update", GlyphKind::UpdateAvailable),
        ("busy", GlyphKind::Busy),
    ]
    .iter()
    .flat_map(|(n, k)| {
        [
            StatusGlyph::new(gid(format!("g-glyph16-{n}")), *k).into_any_element(),
            StatusGlyph::new(gid(format!("g-glyph12-{n}")), *k)
                .size(12.)
                .into_any_element(),
            div().w(px(12.)).into_any_element(),
        ]
    })
    .collect();
    let icon_rows = |size: f32| -> Vec<Vec<AnyElement>> {
        Glyph::ALL
            .chunks(20)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|g| icon(*g, size, p.foreground).into_any_element())
                    .collect()
            })
            .collect()
    };
    let icons_24 = icon_rows(24.);
    let icons_14 = icon_rows(14.);
    let spinners = vec![
        Spinner::new("g-spin").into_any_element(),
        Spinner::new("g-spin-24").size(24.).into_any_element(),
        FlickerSpinner::new("g-flicker").into_any_element(),
        FlickerSpinner::new("g-flicker-24")
            .size(24.)
            .into_any_element(),
        FlickerSpinner::new("g-flicker-48")
            .size(48.)
            .into_any_element(),
    ];
    vec![
        section(
            "Button",
            vec![
                row("variants", variants, cx),
                row("disabled", disabled, cx),
                row("loading", loading, cx),
                row("sizes", sizes, cx),
            ],
            cx,
        ),
        section(
            "Badge",
            vec![row("variants", badges, cx), row("sizes", badge_sizes, cx)],
            cx,
        ),
        section(
            "Spinner / FlickerSpinner",
            vec![row("spinners", spinners, cx)],
            cx,
        ),
        section(
            "Icons (24px, 14px)",
            icons_24
                .into_iter()
                .map(|r| row("24", r, cx))
                .chain(icons_14.into_iter().map(|r| row("14", r, cx)))
                .collect(),
            cx,
        ),
        section(
            "Status glyphs (16px, 12px)",
            vec![row("glyphs", glyphs, cx)],
            cx,
        ),
    ]
}
