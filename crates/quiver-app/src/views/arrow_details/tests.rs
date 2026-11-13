use gpui::{TestAppContext, VisualTestContext};
use quiver_state::arrow_details::{
    build_action_row, facts_from_detail, ActionSlot, ArrowActionKind, RunState,
};
use quiver_state::domain::ArrowState;
use quiver_theme::Mode;

use super::*;
use crate::model::Route;

const PLATFORM: &str = "linux/amd64";

/// The model under test: demo services (a scratch dir, no daemon) with the demo library seeded.
fn boot(cx: &mut TestAppContext) -> Entity<AppModel> {
    let runtime: &'static tokio::runtime::Runtime = Box::leak(Box::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap(),
    ));
    let services = crate::demo::services(runtime).unwrap();
    cx.update(|cx| {
        quiver_ui::install(cx, Mode::Light);
        let model = AppModel::install(cx, services, true);
        crate::demo::seed(model.read(cx));
        model
    })
}

fn entry(
    model: &Entity<AppModel>,
    cx: &mut TestAppContext,
    slug: &str,
) -> quiver_state::domain::ArrowEntry {
    cx.update(|cx| {
        model
            .read(cx)
            .services
            .arrows
            .entries()
            .into_iter()
            .find(|e| e.namespace.ends_with(slug))
            .unwrap()
    })
}

#[gpui::test]
fn demo_arrows_map_to_the_actions_of_their_state(cx: &mut TestAppContext) {
    let model = boot(cx);
    let slots = |slug: &str, cx: &mut TestAppContext| {
        let mut detail = demo::detail(&entry(&model, cx, slug));
        detail.state = entry(&model, cx, slug).state;
        let facts = facts_from_detail(&detail, PLATFORM);
        build_action_row(&facts, None, false)
    };
    let running = slots("minecraft", cx);
    assert!(running.button(ArrowActionKind::Stop).is_some());
    assert!(running.show_settings && running.show_methods);
    let updating = slots("caddy", cx);
    let control = updating
        .progress()
        .expect("an update in flight is a progress control");
    assert_eq!(control.total, 4);
    assert_eq!(control.index, 2);
    let outdated = slots("terraria", cx);
    assert!(outdated.button(ArrowActionKind::Update).is_some());
    assert!(matches!(outdated.slots[0], ActionSlot::Button(_)));
    let installing = slots("redis", cx);
    assert!(installing.progress().is_some());
}

#[gpui::test]
fn the_page_renders_for_every_demo_state(cx: &mut TestAppContext) {
    let model = boot(cx);
    for slug in ["minecraft", "caddy", "terraria", "redis", "valheim"] {
        let namespace = format!("github.com/rabbyte/{slug}");
        // Read once, before the page exists: the store has background writers, and reading it a second time
        // inside the page's closure raced them on a loaded CI box (None between two reads of one closure).
        let expected = cx.update(|cx| entry_state(&model, cx, slug));
        let (m, ns) = (model.clone(), namespace.clone());
        let window = cx.add_window(move |_, cx| ArrowDetailsView::new(m, ns, cx));
        let mut vcx = VisualTestContext::from_window(*window, cx);
        vcx.run_until_parked();
        vcx.draw(
            gpui::point(gpui::px(0.), gpui::px(0.)),
            gpui::size(gpui::px(954.), gpui::px(800.)),
            |_, _| gpui::div(),
        );
        window
            .update(&mut vcx, |view, _, cx| {
                let detail = view.merged(cx).expect("a demo arrow has a detail");
                assert_eq!(detail.namespace, namespace);
                assert_eq!(RunState::from(detail.state), RunState::from(expected));
            })
            .unwrap();
    }
}

fn entry_state(
    model: &Entity<AppModel>,
    cx: &gpui::App,
    slug: &str,
) -> ArrowState {
    model
        .read(cx)
        .services
        .arrows
        .entries()
        .into_iter()
        .find(|e| e.namespace.ends_with(slug))
        .map(|e| e.state)
        .unwrap()
}

#[gpui::test]
fn an_unknown_arrow_is_a_404_not_a_blank_page(cx: &mut TestAppContext) {
    let model = boot(cx);
    let m = model.clone();
    let window = cx
        .add_window(move |_, cx| ArrowDetailsView::new(m, "github.com/nobody/nothing".into(), cx));
    let mut vcx = VisualTestContext::from_window(*window, cx);
    vcx.run_until_parked();
    window
        .update(&mut vcx, |view, _, cx| {
            assert!(view.merged(cx).is_none());
            assert!(view.load.read(|l| l.not_found));
        })
        .unwrap();
    let _ = Route::Home;
}
