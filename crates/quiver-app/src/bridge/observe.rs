use gpui::Context;
use quiver_state::observable::Store;

/// Re-renders the view whenever `store` changes.
///
/// The watch receiver is runtime-agnostic, so it is awaited on GPUI's own executor: no thread hop.
/// The task ends by itself when the view is dropped or the store goes away.
pub fn observe_store<V: 'static, T: 'static>(cx: &mut Context<V>, store: &Store<T>) {
    let mut revisions = store.subscribe();
    cx.spawn(async move |view, cx| {
        while revisions.changed().await.is_ok() {
            if view.update(cx, |_, cx| cx.notify()).is_err() {
                break;
            }
        }
    })
    .detach();
}
