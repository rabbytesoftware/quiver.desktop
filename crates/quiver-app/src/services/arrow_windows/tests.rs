use std::sync::Mutex;

use async_trait::async_trait;
use quiver_arrow_host::{ArrowRequest, ArrowResponse, Headers, ProtocolRequest, WsRelay};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;

use super::*;
use crate::services::Error;
use quiver_arrow_host::{PageBounds, WindowToHost};

/// The test's end of one fake helper.
struct FakeHelper {
    args: HelperArgs,
    /// What the supervisor sent the helper.
    from_host: UnboundedReceiver<HostToWindow>,
    /// What the helper tells the supervisor.
    to_host: UnboundedSender<WindowToHost>,
    /// Resolves when the supervisor dropped the process handle (the helper "was killed").
    killed: oneshot::Receiver<()>,
}

impl FakeHelper {
    /// The next message that is not a show/hide or a position (those are checked on their own).
    async fn command(&mut self) -> Option<HostToWindow> {
        loop {
            match self.from_host.recv().await? {
                HostToWindow::Visible { .. } | HostToWindow::Bounds(_) => {}
                other => return Some(other),
            }
        }
    }

    /// Everything the supervisor sent so far, in order.
    fn drain(&mut self) -> Vec<HostToWindow> {
        let mut all = Vec::new();
        while let Ok(m) = self.from_host.try_recv() {
            all.push(m);
        }
        all
    }
}

struct KillSignal(Option<oneshot::Sender<()>>);

impl Drop for KillSignal {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

#[derive(Default)]
struct FakeLauncher {
    launched: Mutex<Vec<FakeHelper>>,
    fail: Mutex<bool>,
}

impl FakeLauncher {
    fn take(&self, index: usize) -> FakeHelper {
        self.launched.lock().unwrap().remove(index)
    }

    fn count(&self) -> usize {
        self.launched.lock().unwrap().len()
    }
}

impl HelperLauncher for FakeLauncher {
    fn launch(&self, args: &HelperArgs) -> Result<HelperLink> {
        if *self.fail.lock().unwrap() {
            return Err(Error::Helper("no helper binary".into()));
        }
        let (to_helper, from_host) = unbounded_channel();
        let (to_host, from_helper) = unbounded_channel();
        let (kill_tx, killed) = oneshot::channel();
        self.launched.lock().unwrap().push(FakeHelper {
            args: args.clone(),
            from_host,
            to_host,
            killed,
        });
        Ok(HelperLink {
            to_helper,
            from_helper,
            guard: Box::new(KillSignal(Some(kill_tx))),
        })
    }
}

struct EchoUpstream;

#[async_trait]
impl ArrowUpstream for EchoUpstream {
    async fn request(&self, req: ArrowRequest) -> quiver_arrow_host::Result<ArrowResponse> {
        Ok(ArrowResponse::new(200, req.uri.into_bytes()))
    }

    async fn open_ws(&self, _: &str) -> quiver_arrow_host::Result<Box<dyn WsRelay>> {
        Err(quiver_arrow_host::Error::Upstream("unused".into()))
    }
}

fn windows() -> (ArrowWindows, Arc<FakeLauncher>) {
    let launcher = Arc::new(FakeLauncher::default());
    let windows = ArrowWindows::new(
        launcher.clone(),
        Arc::new(ArrowHosts::new()),
        Arc::new(EchoUpstream),
    );
    (windows, launcher)
}

fn ns(n: usize) -> String {
    format!("github.com/user/app{n}")
}

#[tokio::test]
async fn showing_an_app_starts_one_helper_on_its_registered_url_and_title() {
    let (windows, launcher) = windows();
    windows.show(&ns(1), "App 1").await.unwrap();
    windows.show(&ns(1), "App 1").await.unwrap();

    assert_eq!(launcher.count(), 1);
    let helper = launcher.take(0);
    assert_eq!(helper.args.title, "App 1");
    assert!(
        helper.args.url.starts_with("arrow-app://"),
        "{}",
        helper.args.url
    );
    assert!(
        helper.args.url.ends_with(".localhost/"),
        "{}",
        helper.args.url
    );
    assert_eq!(windows.visible(), Some(ns(1)));
}

#[tokio::test]
async fn protocol_requests_are_answered_through_the_arrow_host_with_the_same_id() {
    let (windows, launcher) = windows();
    windows.show(&ns(1), "App").await.unwrap();
    let mut helper = launcher.take(0);

    let url = helper.args.url.clone();
    helper
        .to_host
        .send(WindowToHost::ProtocolRequest(ProtocolRequest {
            id: 41,
            method: "GET".into(),
            uri: format!("{url}page?x=1"),
            headers: Headers::new(),
            body: Vec::new(),
        }))
        .unwrap();

    let Some(HostToWindow::ProtocolResponse(resp)) = helper.command().await else {
        panic!("expected a protocol response");
    };
    assert_eq!((resp.id, resp.status), (41, 200));
    assert_eq!(resp.body, b"/v0/ui/github.com%2Fuser%2Fapp1/page?x=1");
    assert!(resp.headers.contains("content-security-policy"));
}

#[tokio::test]
async fn the_fifth_app_closes_and_kills_the_least_recently_used_helper() {
    let (windows, launcher) = windows();
    for n in 1..=4 {
        windows.show(&ns(n), "App").await.unwrap();
    }
    let mut events = windows.subscribe();
    let mut oldest = launcher.take(0);

    windows.show(&ns(5), "App").await.unwrap();

    assert_eq!(oldest.command().await, Some(HostToWindow::Close));
    oldest.killed.await.unwrap();
    assert_eq!(
        events.recv().await,
        Some(WindowEvent::Evicted { namespace: ns(1) })
    );
    assert_eq!(windows.open_apps(), vec![ns(2), ns(3), ns(4), ns(5)]);
    // The three survivors are untouched.
    assert_eq!(launcher.count(), 4);
    for helper in launcher.launched.lock().unwrap().iter_mut() {
        assert!(helper.killed.try_recv().is_err());
    }
}

#[tokio::test]
async fn reload_tells_only_that_helper_and_bumps_its_key() {
    let (windows, launcher) = windows();
    windows.show(&ns(1), "A").await.unwrap();
    windows.show(&ns(2), "B").await.unwrap();
    let mut first = launcher.take(0);
    let mut second = launcher.take(0);

    windows.reload(&ns(1));

    assert_eq!(first.command().await, Some(HostToWindow::Reload));
    assert!(
        second
            .drain()
            .iter()
            .all(|m| matches!(m, HostToWindow::Visible { .. } | HostToWindow::Bounds(_))),
        "the other app was not told to reload"
    );
    assert_eq!(
        (windows.reload_key(&ns(1)), windows.reload_key(&ns(2))),
        (1, 0)
    );
}

#[tokio::test]
async fn stop_closes_and_kills_the_helper_and_forgets_the_app() {
    let (windows, launcher) = windows();
    windows.show(&ns(1), "A").await.unwrap();
    let mut helper = launcher.take(0);

    assert!(windows.stop(&ns(1)));
    assert_eq!(helper.command().await, Some(HostToWindow::Close));
    helper.killed.await.unwrap();
    assert!(windows.open_apps().is_empty());
    assert_eq!(windows.visible(), None);
    assert!(!windows.stop(&ns(1)));
}

#[tokio::test]
async fn an_app_stopped_by_the_daemon_loses_its_window_but_live_ones_keep_theirs() {
    let (windows, launcher) = windows();
    windows.show(&ns(1), "A").await.unwrap();
    windows.show(&ns(2), "B").await.unwrap();
    let first = launcher.take(0);
    let mut second = launcher.take(0);

    windows.prune(&HashSet::from([ns(1)]));

    let mut first = first;
    assert!(
        first.killed.try_recv().is_err(),
        "a live app's helper must not be killed"
    );
    assert_eq!(second.command().await, Some(HostToWindow::Close));
    second.killed.await.unwrap();
    assert_eq!(windows.open_apps(), vec![ns(1)]);
}

#[tokio::test]
async fn the_user_closing_the_window_forgets_the_app_and_says_so() {
    let (windows, launcher) = windows();
    let mut events = windows.subscribe();
    windows.show(&ns(1), "A").await.unwrap();
    let helper = launcher.take(0);

    helper.to_host.send(WindowToHost::Closed).unwrap();

    assert_eq!(
        events.recv().await,
        Some(WindowEvent::Closed { namespace: ns(1) })
    );
    helper.killed.await.unwrap();
    assert!(windows.open_apps().is_empty());

    // Opening it again makes a fresh helper.
    windows.show(&ns(1), "A").await.unwrap();
    assert_eq!(launcher.count(), 1);
}

#[tokio::test]
async fn a_helper_that_dies_is_treated_as_closed() {
    let (windows, launcher) = windows();
    let mut events = windows.subscribe();
    windows.show(&ns(1), "A").await.unwrap();
    let helper = launcher.take(0);
    drop(helper.to_host);

    assert_eq!(
        events.recv().await,
        Some(WindowEvent::Closed { namespace: ns(1) })
    );
    assert!(windows.open_apps().is_empty());
}

#[tokio::test]
async fn a_launch_failure_leaves_nothing_open() {
    let (windows, launcher) = windows();
    *launcher.fail.lock().unwrap() = true;
    assert!(windows.show(&ns(1), "A").await.is_err());
    assert!(windows.open_apps().is_empty());
    assert_eq!(windows.visible(), None);
}

#[tokio::test]
async fn shutdown_kills_every_helper_is_idempotent_and_opens_nothing_after() {
    let (windows, launcher) = windows();
    windows.show(&ns(1), "A").await.unwrap();
    windows.show(&ns(2), "B").await.unwrap();
    let mut a = launcher.take(0);
    let mut b = launcher.take(0);

    windows.shutdown();
    windows.shutdown();

    for helper in [&mut a, &mut b] {
        assert_eq!(helper.command().await, Some(HostToWindow::Close));
    }
    a.killed.await.unwrap();
    b.killed.await.unwrap();
    assert!(windows.open_apps().is_empty());

    windows.show(&ns(3), "C").await.unwrap();
    assert_eq!(launcher.count(), 0);
}

#[tokio::test]
async fn only_the_visible_apps_page_is_shown_and_it_follows_the_reported_rectangle() {
    let (windows, launcher) = windows();
    let bounds = PageBounds {
        x: 240.0,
        y: 56.0,
        width: 960.0,
        height: 744.0,
    };
    windows.set_bounds("a@x", bounds);
    windows.show("a@x", "A").await.unwrap();
    windows.show("b@x", "B").await.unwrap();
    let mut a = launcher.take(0);
    let mut b = launcher.take(0);
    assert_eq!(
        a.drain(),
        vec![
            HostToWindow::Bounds(bounds),
            HostToWindow::Visible { visible: true },
            HostToWindow::Visible { visible: false },
        ]
    );
    assert_eq!(b.drain(), vec![HostToWindow::Visible { visible: true }]);

    let moved = PageBounds {
        width: 800.0,
        ..bounds
    };
    windows.set_bounds("b@x", moved);
    windows.set_bounds("b@x", moved);
    assert_eq!(
        b.drain(),
        vec![HostToWindow::Bounds(moved)],
        "a repeat is not resent"
    );

    windows.hide();
    assert_eq!(b.drain(), vec![HostToWindow::Visible { visible: false }]);
}
