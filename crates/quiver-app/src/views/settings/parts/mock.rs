//! The old Developer tab's mock-server controls (scenario, chaos, fault injection), over the native
//! demo mode (`--demo`). The scenario re-seeds the arrow catalog and "Daemon unreachable" drops the
//! connection; latency, error rate and the fault rates are recorded for the session only, because the
//! seeded data is answered from memory (the Notice says so).

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Global, IntoElement, SharedString};
use quiver_state::domain::ConnectionStatus;
use quiver_state::i18n::en::settings as keys;
use quiver_ui::inspect::Kind;
use quiver_ui::{ActiveQuiver, ButtonVariant, NumberField, Select, SelectOption, Slider, Switch};

use super::notices::banner;
use super::rows::{probed, reset_button, row, row_with_reset, section, small_button};
use crate::views::settings::SettingsView;

const SCENARIOS: [(&str, &str, &str); 3] = [
    (
        "normal",
        "Normal",
        "17 arrows · every state · a failed install, a yanked ref, a rate-limited host",
    ),
    (
        "extreme",
        "Extreme",
        "200 arrows · 12 collections · the point where the rail scrolls and search stops being optional",
    ),
    (
        "empty",
        "Empty",
        "A first run — nothing installed, nothing followed, nothing to search",
    ),
];

const FAULTS: [(&str, quiver_state::i18n::Key); 12] = [
    ("arrows", keys::DEVELOPER_FAULTS_ARROWS),
    ("arrow-detail", keys::DEVELOPER_FAULTS_ARROW_DETAIL),
    ("search", keys::DEVELOPER_FAULTS_SEARCH),
    ("discover", keys::DEVELOPER_FAULTS_DISCOVER),
    ("collections", keys::DEVELOPER_FAULTS_COLLECTIONS),
    (
        "collection-detail",
        keys::DEVELOPER_FAULTS_COLLECTION_DETAIL,
    ),
    ("home", keys::DEVELOPER_FAULTS_HOME),
    ("runtime", keys::DEVELOPER_FAULTS_RUNTIME),
    ("health", keys::DEVELOPER_FAULTS_HEALTH),
    ("config", keys::DEVELOPER_FAULTS_CONFIG),
    ("path", keys::DEVELOPER_FAULTS_PATH),
    ("console", keys::DEVELOPER_FAULTS_CONSOLE),
];

/// What the Developer tab has been set to this session.
pub struct DevMock {
    applied: &'static str,
    pending: &'static str,
    latency: f64,
    error_rate: f64,
    unreachable: bool,
    faults: [f32; 12],
}

impl Default for DevMock {
    fn default() -> Self {
        Self {
            applied: "normal",
            pending: "normal",
            latency: 0.,
            error_rate: 0.,
            unreachable: false,
            faults: [0.; 12],
        }
    }
}

impl Global for DevMock {}

/// The applied scenario's label (`getScenario(mock.world.scenario).label`), for the mock footer.
pub fn applied_scenario_label(cx: &gpui::App) -> &'static str {
    let applied = cx.try_global::<DevMock>().map_or("normal", |m| m.applied);
    SCENARIOS
        .iter()
        .find(|(name, ..)| *name == applied)
        .map_or("Normal", |(_, label, _)| label)
}

fn scenario_name(value: &str) -> &'static str {
    SCENARIOS
        .iter()
        .find(|(name, ..)| *name == value)
        .map_or("normal", |(name, ..)| name)
}

pub fn mock_sections(
    view: &SettingsView,
    cx: &mut Context<SettingsView>,
) -> Vec<AnyElement> {
    if !cx.has_global::<DevMock>() {
        cx.set_global(DevMock::default());
    }
    let p = cx.quiver().palette;
    let this = cx.entity();
    let demo = view.app.read(cx).demo;
    let (applied, pending, latency, error_rate, unreachable, faults) = {
        let m = cx.global::<DevMock>();
        (
            m.applied,
            m.pending,
            m.latency,
            m.error_rate,
            m.unreachable,
            m.faults,
        )
    };
    let m = view.app.read(cx);
    let t = |key| m.t(key);
    let update = {
        let this = this.clone();
        move |cx: &mut gpui::App, f: &dyn Fn(&mut DevMock)| {
            cx.update_global::<DevMock, _>(|m, _| f(m));
            this.update(cx, |_, cx| cx.notify());
        }
    };

    let mock_toggle = probed(
        Switch::new("settings-mock-enabled")
            .checked(demo)
            .disabled(true),
        "settings.mock.enabled".into(),
        Kind::Switch,
        t(keys::DEVELOPER_MOCK_TOGGLE).into(),
        if demo { "checked" } else { "" }.into(),
    );
    let summary = SCENARIOS
        .iter()
        .find(|(name, ..)| *name == pending)
        .map_or("", |(.., summary)| *summary);
    let options: Vec<SelectOption> = SCENARIOS
        .iter()
        .map(|(name, label, _)| SelectOption::new(*name, *label))
        .collect();
    let scenario_select = probed(
        div().w(px(144.)).child(
            Select::new("settings-mock-scenario")
                .options(options)
                .value(Some(pending.into()))
                .on_change({
                    let update = update.clone();
                    move |value, _, cx| {
                        let next = scenario_name(&value);
                        update(cx, &|m| m.pending = next);
                    }
                }),
        ),
        "settings.mock.scenario".into(),
        Kind::Select,
        t(keys::DEVELOPER_MOCK_SCENARIO_LABEL).into(),
        format!("value={pending}"),
    );
    let apply = {
        let this = this.clone();
        small_button(
            "settings.mock.apply",
            t(keys::DEVELOPER_MOCK_APPLY),
            ButtonVariant::Default,
            demo && pending != applied,
            move |_, cx| {
                cx.update_global::<DevMock, _>(|m, _| m.applied = m.pending);
                this.update(cx, |v, cx| {
                    let scenario = cx.global::<DevMock>().applied;
                    crate::demo::apply_scenario(v.app.read(cx), scenario);
                    cx.notify();
                });
            },
        )
    };
    let mock = section(
        &p,
        t(keys::DEVELOPER_MOCK_TITLE),
        true,
        vec![
            row(
                &p,
                "settings.mock.toggle",
                t(keys::DEVELOPER_MOCK_TOGGLE),
                Some(
                    t(if demo {
                        keys::DEVELOPER_NATIVE_FORCED
                    } else {
                        keys::DEVELOPER_NATIVE_OFF
                    })
                    .into(),
                ),
                vec![mock_toggle],
            ),
            row(
                &p,
                "settings.mock.scenario-row",
                t(keys::DEVELOPER_MOCK_SCENARIO),
                Some(summary.into()),
                vec![scenario_select, apply],
            ),
        ],
    );

    let reset_label = |label: &str| t(keys::ROW_RESET).replace("{setting}", label);
    let latency_label = t(keys::DEVELOPER_CHAOS_LATENCY);
    let rate_label = t(keys::DEVELOPER_CHAOS_ERROR_RATE);
    let unreachable_label = t(keys::DEVELOPER_CHAOS_UNREACHABLE);
    let latency_row = row_with_reset(
        &p,
        "settings.mock.latency",
        latency_label.clone(),
        Some(t(keys::DEVELOPER_CHAOS_LATENCY_DESCRIPTION).into()),
        vec![probed(
            NumberField::new("settings-mock-latency")
                .value(Some(latency))
                .range(0., 10_000.)
                .step(50.)
                .suffix("ms")
                .on_change({
                    let update = update.clone();
                    move |v, _, cx| update(cx, &|m| m.latency = v.unwrap_or(0.))
                }),
            "settings.mock.latency.field".into(),
            Kind::Input,
            t(keys::DEVELOPER_CHAOS_LATENCY_LABEL).into(),
            String::new(),
        )],
        Some(reset_button(
            &p,
            "settings.mock.latency.reset",
            reset_label(&latency_label),
            latency != 0.,
            {
                let update = update.clone();
                move |_, cx| update(cx, &|m| m.latency = 0.)
            },
        )),
    );
    let rate_row = row_with_reset(
        &p,
        "settings.mock.error-rate",
        rate_label.clone(),
        Some(t(keys::DEVELOPER_CHAOS_ERROR_RATE_DESCRIPTION).into()),
        vec![probed(
            NumberField::new("settings-mock-error-rate")
                .value(Some(error_rate))
                .range(0., 100.)
                .step(5.)
                .suffix("%")
                .on_change({
                    let update = update.clone();
                    move |v, _, cx| update(cx, &|m| m.error_rate = v.unwrap_or(0.))
                }),
            "settings.mock.error-rate.field".into(),
            Kind::Input,
            t(keys::DEVELOPER_CHAOS_ERROR_RATE_LABEL).into(),
            String::new(),
        )],
        Some(reset_button(
            &p,
            "settings.mock.error-rate.reset",
            reset_label(&rate_label),
            error_rate != 0.,
            {
                let update = update.clone();
                move |_, cx| update(cx, &|m| m.error_rate = 0.)
            },
        )),
    );
    let set_unreachable = {
        let this = this.clone();
        move |on: bool, cx: &mut gpui::App| {
            cx.update_global::<DevMock, _>(|m, _| m.unreachable = on);
            this.update(cx, |v, cx| {
                if v.app.read(cx).demo {
                    v.app.read(cx).services.status.set_status(if on {
                        ConnectionStatus::Disconnected
                    } else {
                        ConnectionStatus::Ready
                    });
                }
                cx.notify();
            });
        }
    };
    let unreachable_row = row_with_reset(
        &p,
        "settings.mock.unreachable",
        unreachable_label.clone(),
        Some(t(keys::DEVELOPER_CHAOS_UNREACHABLE_DESCRIPTION).into()),
        vec![probed(
            Switch::new("settings-mock-unreachable")
                .checked(unreachable)
                .on_change({
                    let set = set_unreachable.clone();
                    move |on, _, cx| set(on, cx)
                }),
            "settings.mock.unreachable.switch".into(),
            Kind::Switch,
            unreachable_label.clone().into(),
            if unreachable { "checked" } else { "" }.into(),
        )],
        Some(reset_button(
            &p,
            "settings.mock.unreachable.reset",
            reset_label(&unreachable_label),
            unreachable,
            move |_, cx| set_unreachable(false, cx),
        )),
    );
    let chaos = section(
        &p,
        t(keys::DEVELOPER_CHAOS_TITLE),
        false,
        vec![latency_row, rate_row, unreachable_row],
    );

    let fault_rows = FAULTS
        .iter()
        .enumerate()
        .map(|(i, (id, key))| {
            let label = t(*key);
            let rate_aria = m.translator.t_with(
                keys::DEVELOPER_FAULTS_RATE_LABEL,
                &quiver_state::i18n::Params::new().with("family", label.as_str()),
            );
            let percent = format!("{}%", faults[i] as i64);
            row_with_reset(
                &p,
                &format!("settings.mock.fault.{id}"),
                label.clone(),
                None,
                vec![
                    probed(
                        div().w(px(140.)).child(
                            Slider::new(SharedString::from(format!("settings-mock-fault-{id}")))
                                .value(faults[i])
                                .range(0., 100.)
                                .step(5.)
                                .on_change({
                                    let update = update.clone();
                                    move |v, _, cx| update(cx, &|m| m.faults[i] = v)
                                }),
                        ),
                        format!("settings.mock.fault.{id}.slider"),
                        Kind::Other,
                        rate_aria.into(),
                        percent.clone(),
                    ),
                    div()
                        .w(px(34.))
                        .text_right()
                        .text_size(px(12.))
                        .text_color(p.muted_foreground)
                        .child(percent)
                        .into_any_element(),
                ],
                Some(reset_button(
                    &p,
                    &format!("settings.mock.fault.{id}.reset"),
                    reset_label(&label),
                    faults[i] > 0.,
                    {
                        let update = update.clone();
                        move |_, cx| update(cx, &|m| m.faults[i] = 0.)
                    },
                )),
            )
        })
        .collect();
    let injection = section(&p, t(keys::DEVELOPER_FAULTS_TITLE), false, fault_rows);

    vec![
        mock.into_any_element(),
        div()
            .child(banner(&p, t(keys::DEVELOPER_NATIVE_CHAOS_NOTE)))
            .into_any_element(),
        chaos.into_any_element(),
        injection.into_any_element(),
    ]
}
