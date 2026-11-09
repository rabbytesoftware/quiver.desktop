use std::future::Future;

use gpui::Context;
use tokio::runtime::Handle;

/// Runs `work` on the tokio runtime and calls `done` with its result on the UI thread, inside the view
/// (so it can set state and `cx.notify()`). `done` does not run if the view is gone first.
pub fn spawn_action<V, R>(
    cx: &mut Context<V>,
    handle: &Handle,
    work: impl Future<Output = R> + Send + 'static,
    done: impl FnOnce(&mut V, R, &mut Context<V>) + 'static,
) where
    V: 'static,
    R: Send + 'static,
{
    let join = handle.spawn(work);
    cx.spawn(async move |view, cx| {
        if let Ok(result) = join.await {
            view.update(cx, |view, cx| done(view, result, cx)).ok();
        }
    })
    .detach();
}

/// Fire and forget: the outcome reaches the views through the stores.
pub fn spawn_detached(handle: &Handle, work: impl Future<Output = ()> + Send + 'static) {
    handle.spawn(work);
}
