//! Route -> page view.

use gpui::{AnyView, AppContext, Context, Entity, Window};

use super::AppShell;
use crate::model::{AppModel, Route};
use crate::views::arrow_page::ArrowPageView;
use crate::views::collection::CollectionView;
use crate::views::collections::CollectionsView;
use crate::views::home::HomeView;
use crate::views::library::LibraryView;
use crate::views::recommended::RecommendedView;
use crate::views::remote::RemoteView;
use crate::views::search::SearchView;
use crate::views::settings::SettingsView;

pub(super) fn page_for(
    route: &Route,
    app: &Entity<AppModel>,
    _: &mut Window,
    cx: &mut Context<AppShell>,
) -> AnyView {
    match route {
        Route::Home => cx.new(|cx| HomeView::new(app.clone(), cx)).into(),
        Route::Arrow { namespace } => cx
            .new(|cx| ArrowPageView::new(app.clone(), namespace.clone(), cx))
            .into(),
        Route::Recommended => cx.new(|cx| RecommendedView::new(app.clone(), cx)).into(),
        Route::Search { query } => cx
            .new(|cx| SearchView::new(app.clone(), query.clone(), cx))
            .into(),
        Route::Library => cx.new(|cx| LibraryView::new(app.clone(), cx)).into(),
        Route::Collections => cx.new(|cx| CollectionsView::new(app.clone(), cx)).into(),
        Route::Collection { id } => cx
            .new(|cx| CollectionView::new(app.clone(), id.clone(), cx))
            .into(),
        Route::Remote => cx.new(|cx| RemoteView::new(app.clone(), cx)).into(),
        Route::Settings { tab } => cx.new(|cx| SettingsView::new(app.clone(), *tab, cx)).into(),
    }
}
