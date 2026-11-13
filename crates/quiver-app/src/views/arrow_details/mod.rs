//! The arrow page (`ArrowRunningAfter`, `ArrowActivityAfter`, `ArrowNoBannerAfter`,
//! `ArrowMethodsAfter`): header with the one action row, README (or an "At a glance" summary) on the
//! left, the Details card on the right; Settings and Methods are modals. The decisions come from
//! `quiver_state::arrow_details`; this view holds the scroll, the open popovers and the form values.

mod actions;
pub(crate) mod demo;
mod load;
mod methods_modal;
mod page;
mod parts;
mod settings_modal;
mod switch;
mod values;

use gpui::prelude::*;
use gpui::{div, Context, Entity, KeyDownEvent, MouseDownEvent, Render, Subscription, Window};
use quiver_state::arrow_details::channel::ChannelSelection;
use quiver_state::arrow_details::invoke::InvokeState;
use quiver_state::arrow_details::ArrowActionKind;
use quiver_state::domain::ArrowDetail;
use quiver_state::namespace::bare_namespace;
use quiver_state::observable::Store;
use quiver_ui::{ActiveQuiver, ScrollArea};

use crate::bridge::observe_store;
use crate::model::AppModel;
use crate::views::card_dismiss::CardDismiss;
use crate::views::screen_probe;
use load::{Load, ReadTask};
use values::FormValues;

/// The library row a route's namespace names: the identity itself, or the bare namespace of one.
fn is_row_of(
    row: &str,
    route: &str,
) -> bool {
    row == route || (!route.contains('@') && bare_namespace(row) == route)
}

/// The popover card under the action row that is showing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Card {
    Steps,
    Info(ArrowActionKind),
}

pub struct ArrowDetailsView {
    pub(super) app: Entity<AppModel>,
    /// The route's namespace (bare).
    pub(super) namespace: String,
    pub(super) load: Store<Load>,
    pub(super) values: Entity<FormValues>,
    pub(super) seeded: bool,
    pub(super) invoke: InvokeState,
    /// The Steps popover of the progress control.
    pub(super) steps_open: bool,
    /// Which step the popover explains (default: the running one).
    pub(super) inspected_step: Option<usize>,
    /// The ⓘ popover of an action button.
    pub(super) info_open: Option<ArrowActionKind>,
    pub(super) problem_open: bool,
    /// Outside press and Escape close the Steps and ⓘ cards; see [`CardDismiss`].
    pub(super) card: CardDismiss<Card>,
    /// The Channel and Version pickers of an arrow that is not in the library yet: the identity Add registers.
    pub(super) selection: ChannelSelection,
    /// The selector and channel count the picker was last seeded from: either changing re-seeds it (the
    /// channels can land before the detail says which selector the page follows).
    pub(super) selection_seen: Option<(String, usize)>,
    /// The Switch dialog's own pick, while it is open.
    pub(super) switch_selection: Option<ChannelSelection>,
    pub(super) switch_pending: bool,
    pub(super) register_error: Option<String>,
    /// The run's interface was already open when the page was entered (or has been seen): only a surface that
    /// opens while the person is here takes them to the app pane, so Details in the pane does not bounce back.
    /// Rounded corners when the page is shown inside a dialog (the app pane's Details): its background would
    /// square the dialog's corners off, GPUI clipping to rectangles.
    pub(crate) corner: Option<f32>,
    /// `--demo` only: the modal the route asked for, opened once the page has its detail.
    demo_modal: Option<&'static str>,
    _read: Option<ReadTask>,
    _subscriptions: Vec<Subscription>,
}

impl ArrowDetailsView {
    pub fn new(
        app: Entity<AppModel>,
        namespace: String,
        cx: &mut Context<Self>,
    ) -> Self {
        let model = app.read(cx);
        let services = model.services.clone();
        let demo = model.demo;
        let load: Store<Load> = Store::default();
        observe_store(cx, &load);
        observe_store(cx, services.arrows.store());
        observe_store(cx, services.status.store());
        let entry = services
            .arrows
            .entries()
            .into_iter()
            .find(|e| is_row_of(&e.namespace, &namespace));
        let mut read = None;
        if demo {
            load.update(|l| match &entry {
                Some(entry) => l.detail = Some(demo::detail(entry)),
                None if namespace == "error" => {
                    l.error = Some("The Quiver Core did not answer (demo)".into())
                }
                None => l.not_found = true,
            });
        } else {
            // A library row is read at its identity (`ns@selector`): each selector has its own manifest.
            let identity = entry.map_or_else(|| namespace.clone(), |e| e.namespace);
            read = Some(load::start(&services, identity, load.clone()));
        }
        let values = cx.new(|_| FormValues::default());
        let subscriptions = vec![cx.observe(&values, |_, _, cx| cx.notify())];
        let mut view = Self {
            app,
            namespace,
            load,
            values,
            seeded: false,
            invoke: InvokeState::default(),
            steps_open: false,
            card: CardDismiss::new(cx.focus_handle()),
            inspected_step: None,
            info_open: None,
            problem_open: false,
            selection: ChannelSelection::default(),
            selection_seen: None,
            switch_selection: None,
            switch_pending: false,
            register_error: None,
            corner: None,
            demo_modal: demo
                .then(crate::demo::route::arrow_modal_from_environment)
                .flatten(),
            _read: read,
            _subscriptions: subscriptions,
        };
        view.watch_live(cx);
        view
    }

    /// The detail with the live runtime (state, active run) laid over the one-time read.
    pub(super) fn merged(
        &self,
        cx: &gpui::App,
    ) -> Option<ArrowDetail> {
        let model = self.app.read(cx);
        let mut detail = self.load.read(|l| l.detail.clone())?;
        let entry = model.services.arrows.store().read(|s| {
            s.arrows
                .values()
                .find(|e| is_row_of(&e.namespace, &self.namespace))
                .cloned()
        });
        if let Some(entry) = entry {
            detail.state = entry.state;
            // The demo has no runtime stream: its detail carries the run it wants to show.
            detail.active_run = if model.demo {
                entry.active_run.or(detail.active_run)
            } else {
                entry.active_run
            };
        }
        Some(detail)
    }

    /// Defaults of the manifest seed the form once per arrow.
    fn seed_values(
        &mut self,
        detail: &ArrowDetail,
        cx: &mut Context<Self>,
    ) {
        if self.seeded {
            return;
        }
        self.seeded = true;
        let initial = quiver_state::arrow_details::variable_form::initial_values(&detail.variables);
        self.values.update(cx, |v, _| v.values = initial);
    }
}

impl ArrowDetailsView {
    /// Which popover card is showing (the Steps card wins if both flags were ever set).
    pub(super) fn open_card(&self) -> Option<Card> {
        if self.steps_open {
            Some(Card::Steps)
        } else {
            self.info_open.map(Card::Info)
        }
    }

    pub(super) fn set_card(
        &mut self,
        card: Option<Card>,
        cx: &mut Context<Self>,
    ) {
        self.steps_open = card == Some(Card::Steps);
        self.info_open = match card {
            Some(Card::Info(kind)) => Some(kind),
            _ => None,
        };
        cx.notify();
    }

    /// The trigger of `card` was clicked: close it if showing, open it if not, and never reopen what the
    /// press that began this click just closed.
    pub(super) fn toggle_card(
        &mut self,
        card: Card,
        cx: &mut Context<Self>,
    ) {
        let next = self.card.next(card, self.open_card());
        self.set_card(next, cx);
    }
}

impl Render for ArrowDetailsView {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.quiver().palette;
        let load = self.load.snapshot();
        let mut problem = None;
        let body = match self.merged(cx) {
            Some(detail) => {
                self.seed_values(&detail, cx);
                self.sync_selection(&detail.selector, &detail.channels);
                if let Some(modal) = self.demo_modal.take() {
                    let this = cx.entity();
                    cx.defer(move |cx| {
                        this.update(cx, |view, cx| match modal {
                            "methods" => view.open_methods(cx),
                            _ => view.open_settings(cx),
                        })
                    });
                }
                let page = page::Page::build(&detail, self);
                problem = page.problem_text(self.app.read(cx).translator);
                parts::page(self, &page, window, cx).into_any_element()
            }
            None if load.not_found => parts::not_found(self, cx).into_any_element(),
            None if load.error.is_some() => parts::load_error(self, cx).into_any_element(),
            None => parts::skeleton(cx).into_any_element(),
        };
        let dialogs = parts::dialogs(self, problem, cx);
        // A card up takes the keyboard so Escape reaches this root (the card is drawn where there is no window).
        if self.open_card().is_some() && !self.card.focus.contains_focused(window, cx) {
            let focus = self.card.focus.clone();
            window.defer(cx, move |window, _| window.focus(&focus));
        }
        div()
            .id("arrow")
            .relative()
            .track_focus(&self.card.focus)
            .capture_any_mouse_down(
                cx.listener(|this, _: &MouseDownEvent, _, _| this.card.press(this.open_card())),
            )
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && this.open_card().is_some() {
                    this.set_card(None, cx);
                }
            }))
            .size_full()
            .when_some(self.corner, |d, r| d.rounded(gpui::px(r)))
            .bg(p.background)
            .child(
                ScrollArea::new("arrow.scroll")
                    .fade_color(p.background)
                    .child(body),
            )
            .children(dialogs)
            .child(screen_probe("arrow"))
    }
}

#[cfg(test)]
mod tests;
