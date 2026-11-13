//! The mock footer (port of `src/components/mock-indicator.tsx`, the `AppShell` footer): a 22px primary bar
//! under the page whenever the app runs in demo mode, naming the applied scenario, with a link to the
//! Developer tab where the mock controls live.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement};
use quiver_state::i18n::en::mock as keys;
use quiver_state::i18n::Params;
use quiver_state::settings::SettingsTab;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{ActiveQuiver, TrackedText};

use crate::model::{navigate, AppModel, Route};
use crate::views::settings::applied_scenario_label;

pub(super) fn mock_indicator(
    app: &AppModel,
    cx: &App,
) -> Option<impl IntoElement> {
    if !app.demo {
        return None;
    }
    let p = cx.quiver().palette;
    let label = applied_scenario_label(cx);
    let (badge, status, turn_off) = (
        app.t(keys::BADGE),
        app.translator
            .t_with(keys::STATUS, &Params::new().with("scenario", label)),
        app.t(keys::TURN_OFF),
    );
    let fg = p.primary_foreground;
    Some(
        div()
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .h(px(22.))
            .px(px(12.))
            .bg(p.primary)
            .text_color(fg)
            .text_size(px(11.))
            .line_height(px(16.5))
            // `text-[11px] font-medium uppercase tracking-[0.12em]`: 0.12em is 1.32px at 11px.
            .child(
                TrackedText::new(badge.to_uppercase(), 11., 16.5, 1.32, fg)
                    .weight(FontWeight::MEDIUM),
            )
            .child(div().opacity(0.7).child(status.clone()))
            .child(
                div()
                    .id("mock-turn-off")
                    .relative()
                    .ml(px(4.))
                    .underline()
                    .opacity(0.7)
                    .cursor_pointer()
                    .hover(|s| s.opacity(1.))
                    .on_click(|_, _, cx| {
                        navigate(
                            cx,
                            Route::Settings {
                                tab: SettingsTab::Developer,
                            },
                        );
                    })
                    .child(turn_off.clone())
                    .child(probe("mock.turnOff", Kind::Button, turn_off, true, "")),
            )
            .child(probe("mock.indicator", Kind::Text, badge, true, status)),
    )
}
