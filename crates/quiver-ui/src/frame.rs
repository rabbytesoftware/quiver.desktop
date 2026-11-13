//! `Frame`, `FramePanel`, `FrameHeader`, `FrameTitle`, `FrameDescription`, `FrameFooter`:
//! port of `src/components/ui/frame.tsx`.

use gpui::prelude::*;
use gpui::px;
use gpui::FontWeight;
use quiver_theme::Radius;

use crate::parts::part;

part!(
    /// `relative flex flex-col rounded-2xl bg-muted p-1`.
    Frame,
    |p, d| d
        .relative()
        .flex()
        .flex_col()
        .rounded(px(Radius::XXL))
        .bg(p.muted)
        .p(px(4.))
);

part!(
    /// `rounded-xl border bg-card p-5 shadow-xs`. Use [`FramePanel::table`] for the `has-[table]` look.
    FramePanel,
    |p, d, q| d
        .child(crate::edge::control_edge(&q, Radius::XL, 0.08))
        .relative()
        .rounded(px(Radius::XL))
        .border_1()
        .border_color(p.border)
        .bg(p.card)
        .p(px(20.))
        .shadow(crate::elevation::shadow_xs(gpui::black().opacity(0.05)))
);

part!(
    /// `flex flex-col px-5 py-4`.
    FrameHeader,
    |_p, d| d.flex().flex_col().px(px(20.)).py(px(16.))
);

part!(
    /// `font-semibold text-sm`.
    FrameTitle,
    |p, d| d
        .text_size(px(14.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.foreground)
);

part!(
    /// `text-muted-foreground text-sm`.
    FrameDescription,
    |p, d| d.text_size(px(14.)).text_color(p.muted_foreground)
);

part!(
    /// `flex flex-col gap-1 px-5 py-4`.
    FrameFooter,
    |_p, d| d.flex().flex_col().gap(px(4.)).px(px(20.)).py(px(16.))
);
