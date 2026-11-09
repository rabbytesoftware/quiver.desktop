//! Gallery page: input, number field, switch, slider, select, label.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context};

use super::layout::{row, section};
use super::state::Gallery;
use crate::{
    Input, Label, NumberField, Select, SelectOption, SelectSize, Slider, Switch, SwitchSize,
};

fn options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("stable", "Stable"),
        SelectOption::new("beta", "Beta"),
        SelectOption::new("nightly", "Nightly"),
        SelectOption::new("old", "Deprecated").disabled(true),
    ]
}

pub(super) fn page(g: &mut Gallery, cx: &mut Context<Gallery>) -> Vec<AnyElement> {
    let on = g.switch_on;
    let switches = vec![
        Switch::new("g-switch")
            .checked(on)
            .on_change(cx.processor(|this, v, _, cx| {
                this.switch_on = v;
                cx.notify();
            }))
            .into_any_element(),
        Switch::new("g-switch-sm")
            .size(SwitchSize::Sm)
            .checked(on)
            .on_change(cx.processor(|this, v, _, cx| {
                this.switch_on = v;
                cx.notify();
            }))
            .into_any_element(),
        Switch::new("g-switch-off")
            .checked(false)
            .into_any_element(),
        Switch::new("g-switch-dis")
            .checked(true)
            .disabled(true)
            .into_any_element(),
        Label::new("Show build indicator").into_any_element(),
    ];
    let inputs = vec![
        div()
            .w(px(260.))
            .child(Input::new("g-input", &g.text))
            .into_any_element(),
        div()
            .w(px(200.))
            .child(Input::new("g-input-invalid", &g.text).invalid(true))
            .into_any_element(),
        div()
            .w(px(200.))
            .child(Input::new("g-input-dis", &g.text).disabled(true))
            .into_any_element(),
    ];
    let numbers = vec![
        NumberField::new("g-latency")
            .value(g.latency)
            .range(0., 10000.)
            .step(50.)
            .suffix("ms")
            .on_change(cx.processor(|this, v, _, cx| {
                this.latency = v;
                cx.notify();
            }))
            .into_any_element(),
        NumberField::new("g-rate")
            .value(g.rate)
            .range(0., 100.)
            .step(5.)
            .suffix("%")
            .on_change(cx.processor(|this, v, _, cx| {
                this.rate = v;
                cx.notify();
            }))
            .into_any_element(),
        div()
            .child(format!("latency = {:?}, rate = {:?}", g.latency, g.rate))
            .into_any_element(),
    ];
    let sliders = vec![
        div()
            .w(px(140.))
            .child(
                Slider::new("g-slider")
                    .value(g.slider)
                    .step(5.)
                    .on_change(cx.processor(|this, v, _, cx| {
                        this.slider = v;
                        cx.notify();
                    })),
            )
            .into_any_element(),
        div()
            .w(px(40.))
            .child(format!("{}", g.slider))
            .into_any_element(),
        div()
            .w(px(140.))
            .child(Slider::new("g-slider-dis").value(60.).disabled(true))
            .into_any_element(),
    ];
    let selects = vec![
        div()
            .w(px(180.))
            .child(
                Select::new("g-select")
                    .options(options())
                    .value(g.select.clone())
                    .on_change(cx.processor(|this, v, _, cx| {
                        this.select = Some(v);
                        cx.notify();
                    })),
            )
            .into_any_element(),
        div()
            .w(px(150.))
            .child(
                Select::new("g-select-sm")
                    .size(SelectSize::Sm)
                    .options(options())
                    .placeholder("Channel")
                    .on_change(cx.processor(|_, _, _, _| {})),
            )
            .into_any_element(),
        div()
            .w(px(150.))
            .child(
                Select::new("g-select-lg")
                    .size(SelectSize::Lg)
                    .options(options())
                    .value(Some("beta".into())),
            )
            .into_any_element(),
        div()
            .w(px(150.))
            .child(
                Select::new("g-select-dis")
                    .options(options())
                    .disabled(true)
                    .value(Some("beta".into())),
            )
            .into_any_element(),
    ];
    vec![
        section("Input", vec![row("states", inputs, cx)], cx),
        section("NumberField", vec![row("min/max/step", numbers, cx)], cx),
        section(
            "Switch / Label",
            vec![row("sizes, states", switches, cx)],
            cx,
        ),
        section("Slider", vec![row("step 5", sliders, cx)], cx),
        section(
            "Select",
            vec![row("sm / default / lg / disabled", selects, cx)],
            cx,
        ),
    ]
}
