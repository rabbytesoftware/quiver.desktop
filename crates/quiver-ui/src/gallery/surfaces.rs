//! Gallery page: frame, avatar, scroll area, progress button, wavy line.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context};

use super::layout::{row, section};
use super::state::Gallery;
use crate::theme::ActiveQuiver;
use crate::{
    Avatar, Button, ButtonVariant, Frame, FrameDescription, FrameFooter, FrameHeader, FramePanel,
    FrameTitle, ProgressButton, ScrollArea, WavyLine,
};

pub(super) fn page(
    g: &mut Gallery,
    cx: &mut Context<Gallery>,
) -> Vec<AnyElement> {
    let p = cx.quiver().palette;
    let frame = Frame::new()
        .w(px(360.))
        .child(
            FrameHeader::new()
                .child(FrameTitle::new().child("Engine"))
                .child(FrameDescription::new().child("Core connection and logging.")),
        )
        .child(FramePanel::new().child("Panel content"))
        .child(FrameFooter::new().child(FrameDescription::new().child("Footer")));
    let avatars = vec![
        Avatar::new("g-avatar").fallback("QV").into_any_element(),
        Avatar::new("g-avatar-lg")
            .size(48.)
            .fallback("RB")
            .into_any_element(),
        Avatar::new("g-avatar-bad")
            .fallback("??")
            .src("does-not-exist.png")
            .into_any_element(),
    ];
    let lines: Vec<AnyElement> = (1..=40)
        .map(|i| {
            div()
                .py(px(2.))
                .child(format!("Scrollable line {i}"))
                .into_any_element()
        })
        .collect();
    let scroll = div()
        .w(px(260.))
        .h(px(140.))
        .rounded(px(10.))
        .border_1()
        .border_color(p.border)
        .bg(p.popover)
        .child(
            ScrollArea::new("g-scroll-area")
                .fade_color(p.popover)
                .child(div().p(px(12.)).children(lines)),
        );
    let steps = cx.listener(|this, _, _, cx| {
        this.steps_open = !this.steps_open;
        cx.notify();
    });
    let progress = ProgressButton::new("g-progress")
        .title("Updating")
        .detail("Verify checksum \u{b7} 2 of 4")
        .progress(0.36)
        .steps_open(g.steps_open)
        .on_steps(steps);
    let wavy = div()
        .w(px(420.))
        .flex()
        .flex_col()
        .items_center()
        .child(WavyLine::new())
        .child(
            div().mt(px(-18.)).bg(p.background).px(px(8.)).child(
                Button::new("g-load-more")
                    .variant(ButtonVariant::Outline)
                    .label("Load more"),
            ),
        );
    vec![
        section(
            "Frame",
            vec![row("frame", vec![frame.into_any_element()], cx)],
            cx,
        ),
        section(
            "Avatar",
            vec![row("fallback / size / broken src", avatars, cx)],
            cx,
        ),
        section(
            "ScrollArea",
            vec![row(
                "fade + thumb (hover)",
                vec![scroll.into_any_element()],
                cx,
            )],
            cx,
        ),
        section(
            "ProgressButton",
            vec![row(
                "36% (Steps toggle)",
                vec![progress.into_any_element()],
                cx,
            )],
            cx,
        ),
        section(
            "WavyLine",
            vec![row("load more", vec![wavy.into_any_element()], cx)],
            cx,
        ),
    ]
}
