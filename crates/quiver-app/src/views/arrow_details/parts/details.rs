//! The Details card (`ArrowRunningAfter`): requirements, dependencies, network, people and links,
//! each section only when the manifest has something for it (`details::visible_sections`).

use crate::model::Route;
use gpui::prelude::*;
use gpui::{black, div, px, AnyElement, Context, FontWeight, IntoElement, SharedString};
use quiver_state::arrow_details::dependency_rows::{
    build_dependency_rows, CatalogEntryFacts, DependencyRow,
};
use quiver_state::arrow_details::details::{
    credit_initial, credit_key, visible_sections, CreditFacts, DetailsSection,
};
use quiver_state::arrow_details::{compute_status, RunState, StatusGlyph};
use quiver_state::i18n::en::arrow;
use quiver_state::i18n::{Params, Translator};
use quiver_theme::FontFamily;
use quiver_ui::inspect::{probe, Kind};
use quiver_ui::{
    ActiveQuiver, Avatar, Badge, BadgeSize, BadgeVariant, CardArt, GlyphKind, Palette,
    StatusGlyph as GlyphWidget,
};

use super::banner::app_icon_sized;
use crate::views::arrow_details::page::Page;
use crate::views::arrow_details::ArrowDetailsView;

fn label(
    text: String,
    p: &Palette,
) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .line_height(px(15.))
        .text_color(p.muted_foreground)
        .child(text)
}

fn mono_value(
    text: String,
    p: &Palette,
) -> impl IntoElement {
    div()
        .font_family(FontFamily::Mono.family())
        .text_size(px(13.))
        .text_color(p.muted_foreground)
        .child(text)
}

fn row(
    id: String,
    left: impl IntoElement,
    right: Option<AnyElement>,
) -> impl IntoElement {
    div()
        .relative()
        .h(px(18.))
        .flex()
        .items_center()
        .justify_between()
        .child(left)
        .children(right)
        .child(probe(id, Kind::Text, "", true, ""))
}

/// The board's requirement mark: one outlined square (12 of 24 units, stroke 1.6) for every row.
fn requirement_mark(p: &Palette) -> impl IntoElement {
    div()
        .flex_none()
        .size(px(16.))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(px(7.5))
                .rounded(px(1.25))
                .border_1()
                .border_color(p.muted_foreground),
        )
}

fn requirements(
    page: &Page,
    tr: &Translator,
    p: &Palette,
) -> AnyElement {
    let r = page.details.requirement.unwrap_or_default();
    let item = |id: &str, name: gpui::SharedString, key, value: f64| {
        let text = tr.t_with(key, &Params::new().with("count", value));
        row(
            format!("arrow.requirement.{id}"),
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(requirement_mark(p))
                .child(div().text_size(px(14.)).child(name)),
            Some(mono_value(text, p).into_any_element()),
        )
    };
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(label(tr.t(arrow::DETAILS_REQUIREMENTS), p))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(item(
                    "cpu",
                    tr.t(arrow::DETAILS_REQUIREMENTS_CPU).into(),
                    arrow::DETAILS_REQUIREMENTS_CPU_VALUE,
                    r.cpu_cores,
                ))
                .child(item(
                    "memory",
                    tr.t(arrow::DETAILS_REQUIREMENTS_MEMORY).into(),
                    arrow::DETAILS_REQUIREMENTS_MEMORY_VALUE,
                    r.memory_gb,
                ))
                .child(item(
                    "disk",
                    tr.t(arrow::DETAILS_REQUIREMENTS_DISK).into(),
                    arrow::DETAILS_REQUIREMENTS_DISK_VALUE,
                    r.disk_gb,
                )),
        )
        .into_any_element()
}

fn dependency_row(
    row: &DependencyRow,
    id: String,
    tr: &Translator,
    p: &Palette,
    cx: &mut Context<ArrowDetailsView>,
) -> AnyElement {
    let status = compute_status(row.state, row.user_installed, false);
    let glyph = match status.glyph {
        StatusGlyph::Busy => Some(GlyphKind::Busy),
        StatusGlyph::Running => Some(GlyphKind::Running),
        StatusGlyph::NeedsAttention => Some(GlyphKind::NeedsAttention),
        StatusGlyph::UpdateAvailable => Some(GlyphKind::UpdateAvailable),
        StatusGlyph::Ready | StatusGlyph::Inactive => None,
    };
    let variant = match status.glyph {
        StatusGlyph::Running => BadgeVariant::Default,
        StatusGlyph::Busy | StatusGlyph::Ready => BadgeVariant::Secondary,
        StatusGlyph::NeedsAttention => BadgeVariant::Error,
        StatusGlyph::UpdateAvailable | StatusGlyph::Inactive => BadgeVariant::Outline,
    };
    let label = tr.t(status.label);
    let namespace = row.namespace.clone();
    let app = view_app(cx);
    let card = CardArt {
        initial: row
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect::<String>())
            .unwrap_or_else(|| "?".into())
            .into(),
        name: row.name.clone().into(),
        publisher: SharedString::default(),
        hue: quiver_state::tile::hue_for(quiver_state::namespace::bare_namespace(&row.namespace)),
    };
    div()
        .id(SharedString::from(id.replace('.', "-")))
        .relative()
        .h(px(30.))
        .mx(px(-4.))
        .px(px(4.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .gap(px(10.))
        .cursor_pointer()
        .hover(|s| s.bg(p.muted))
        .on_click(move |_, _, cx| {
            let route = Route::Arrow {
                namespace: namespace.clone(),
            };
            app.update(cx, |m, cx| m.navigate(route, cx));
        })
        .child(app_icon_sized(&card, 20., row.icon.as_deref()))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(14.))
                .child(row.name.clone()),
        )
        .child(
            Badge::new()
                .variant(variant)
                .size(BadgeSize::Sm)
                .children(
                    glyph.map(|kind| GlyphWidget::new("arrow.dependency.glyph", kind).size(12.)),
                )
                .child(label.clone()),
        )
        .child(probe(id, Kind::Button, row.name.clone(), true, label))
        .into_any_element()
}

fn view_app(cx: &Context<ArrowDetailsView>) -> gpui::Entity<crate::model::AppModel> {
    crate::model::app_model(cx)
}

fn dependency_list(
    heading: String,
    id: &str,
    rows: &[DependencyRow],
    tr: &Translator,
    p: &Palette,
    cx: &mut Context<ArrowDetailsView>,
) -> AnyElement {
    let items: Vec<_> = rows
        .iter()
        .map(|row| {
            let bare = quiver_state::namespace::bare_namespace(&row.namespace);
            dependency_row(row, format!("arrow.{id}.{bare}"), tr, p, cx)
        })
        .collect();
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(label(heading, p))
        .child(div().flex().flex_col().children(items))
        .into_any_element()
}

fn catalog(
    view: &ArrowDetailsView,
    cx: &Context<ArrowDetailsView>,
) -> Vec<CatalogEntryFacts> {
    view.app
        .read(cx)
        .services
        .arrows
        .entries()
        .into_iter()
        .map(|e| CatalogEntryFacts {
            namespace: e.namespace,
            name: e.name,
            icon: e.icon,
            state: RunState::from(e.state),
        })
        .collect()
}

fn network(
    page: &Page,
    tr: &Translator,
    p: &Palette,
) -> AnyElement {
    let ports: Vec<_> = page
        .details
        .ports
        .iter()
        .map(|port| {
            row(
                format!("arrow.port.{}", port.name),
                // Board `ArrowRunningAfter`: the port's name and its number, nothing between.
                div()
                    .font_family(FontFamily::Mono.family())
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(port.name.clone()),
                Some(mono_value(port.default.to_string(), p).into_any_element()),
            )
        })
        .collect();
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .child(label(tr.t(arrow::DETAILS_NETWORK), p))
        .child(div().flex().flex_col().gap(px(10.)).children(ports))
        .into_any_element()
}

fn people(
    heading: String,
    id: &str,
    list: &[CreditFacts],
    p: &Palette,
) -> AnyElement {
    let rows: Vec<_> = list
        .iter()
        .map(|c| {
            let key = credit_key(c);
            row(
                format!("arrow.{id}.{}", c.name.to_lowercase()),
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        Avatar::new(SharedString::from(format!("arrow-{id}-{key}")))
                            .size(24.)
                            .fallback(credit_initial(c)),
                    )
                    .child(div().text_size(px(14.)).child(c.name.clone())),
                None,
            )
            .into_any_element()
        })
        .collect();
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(label(heading, p))
        .child(div().flex().flex_col().gap(px(10.)).children(rows))
        .into_any_element()
}

fn links(
    page: &Page,
    tr: &Translator,
    p: &Palette,
    cx: &mut Context<ArrowDetailsView>,
) -> AnyElement {
    let url = page.details.url.clone();
    let target = format!("https://{url}");
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(label(tr.t(arrow::DETAILS_LINKS), p))
        .child(
            div()
                .id("arrow.link")
                .relative()
                .cursor_pointer()
                .text_size(px(13.))
                .underline()
                .child(url)
                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&target)))
                .child(probe("arrow.link", Kind::Button, "link", true, "")),
        )
        .into_any_element()
}

pub fn details(
    view: &ArrowDetailsView,
    page: &Page,
    cx: &mut Context<ArrowDetailsView>,
) -> Option<impl IntoElement> {
    let p = cx.quiver().palette;
    let tr = view.app.read(cx).translator;
    let entries = catalog(view, cx);
    let namespaces = |list: Vec<String>| build_dependency_rows(&list, &entries);
    let depends_on = namespaces(
        page.detail
            .dependencies
            .iter()
            .map(|d| d.namespace.clone())
            .collect(),
    );
    let required_by = namespaces(page.detail.dependents.clone());
    let mut sections: Vec<AnyElement> = Vec::new();
    for section in visible_sections(&page.details) {
        sections.push(match section {
            DetailsSection::Requirements => requirements(page, &tr, &p),
            DetailsSection::Dependencies => dependency_list(
                tr.t(arrow::DETAILS_DEPENDENCIES),
                "dependency",
                &depends_on,
                &tr,
                &p,
                cx,
            ),
            DetailsSection::RequiredBy => dependency_list(
                tr.t(arrow::DETAILS_REQUIRED_BY),
                "required-by",
                &required_by,
                &tr,
                &p,
                cx,
            ),
            DetailsSection::Network => network(page, &tr, &p),
            DetailsSection::Maintainers => people(
                tr.t(arrow::DETAILS_MAINTAINERS),
                "maintainer",
                &page.details.maintainers,
                &p,
            ),
            DetailsSection::Credits => people(
                tr.t(arrow::DETAILS_CREDITS),
                "credit",
                &page.details.credits,
                &p,
            ),
            DetailsSection::Links => links(page, &tr, &p, cx),
        });
    }
    let count = sections.len();
    // Nothing the manifest says: no card at all, rather than a "Details" title over an empty panel.
    if count == 0 {
        return None;
    }
    let sections: Vec<_> = sections
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            div()
                .py(px(16.))
                .when(i == 0, |d| d.pt(px(19.)).pb(px(24.)))
                .when(i + 1 < count, |d| d.border_b_1().border_color(p.border))
                .child(s)
        })
        .collect();
    div()
        .relative()
        .flex_none()
        .w(px(339.))
        .p(px(5.))
        .pt(px(19.))
        .rounded(px(16.))
        .bg(p.muted)
        .child(
            div()
                .px(px(17.))
                .pb(px(16.))
                .text_size(px(14.))
                .line_height(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(tr.t(arrow::DETAILS_TITLE)),
        )
        .child(
            div()
                .px(px(17.))
                .rounded(px(12.))
                .bg(p.card)
                .shadow(vec![gpui::BoxShadow {
                    color: black().opacity(0.08),
                    offset: gpui::point(px(0.), px(1.)),
                    blur_radius: px(3.),
                    spread_radius: px(0.),
                }])
                .children(sections),
        )
        .child(probe("arrow.details", Kind::Other, "Details", true, ""))
        .into()
}
