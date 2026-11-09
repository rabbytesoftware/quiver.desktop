use super::*;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Records every announce the manager asks for.
struct RecordingAnnounce(mpsc::UnboundedSender<Arc<dyn Transport>>);

#[async_trait]
impl SelfAnnounce for RecordingAnnounce {
    async fn announce(&self, transport: Arc<dyn Transport>) {
        let _ = self.0.send(transport);
    }
}

fn recorder() -> (
    Arc<dyn SelfAnnounce>,
    mpsc::UnboundedReceiver<Arc<dyn Transport>>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    (Arc::new(RecordingAnnounce(tx)), rx)
}

/// The bug this guards: quiver.desktop used to register itself in the Core
/// catalog of EVERY connection that became ready, remote ones included --
/// filing this desktop in a catalog on somebody else's machine.
#[tokio::test]
async fn a_remote_connection_becoming_ready_is_never_announced() {
    let (hook, mut announced) = recorder();
    let h = harness(
        StubConnection::boxed("local", Arc::new(AtomicBool::new(false))),
        Some(hook),
    );

    let handle = h
        .manager
        .switch_to(async {
            Ok((
                StubConnection::boxed_of_kind("home-lab", ConnectionKind::Remote),
                CoreStatus::Ready,
            ))
        })
        .await
        .expect("the switch must succeed");

    assert!(
        handle.is_none(),
        "no announce may even be started for a remote"
    );
    drop(h);
    assert!(
        announced.recv().await.is_none(),
        "the hook must never have been called for a remote connection"
    );
}

#[tokio::test]
async fn the_local_connection_becoming_ready_is_announced_over_its_own_transport() {
    let (hook, mut announced) = recorder();
    let h = harness(
        StubConnection::boxed_of_kind("home-lab", ConnectionKind::Remote),
        Some(hook),
    );
    let local = StubConnection::boxed_of_kind("local", ConnectionKind::Local);
    let local_transport = local.transport();

    let handle = h
        .manager
        .switch_to(async { Ok((local, CoreStatus::Ready)) })
        .await
        .expect("the switch must succeed")
        .expect("a ready local connection must start an announce");
    handle.await.expect("the announce task must not panic");

    let transport = announced.recv().await.expect("the hook must be called");
    assert!(Arc::ptr_eq(&transport, &local_transport));
}

#[tokio::test]
async fn a_local_connection_that_did_not_become_ready_is_not_announced() {
    let (hook, _announced) = recorder();
    let h = harness(
        StubConnection::boxed_of_kind("home-lab", ConnectionKind::Remote),
        Some(hook),
    );

    let handle = h
        .manager
        .switch_to(async {
            Ok((
                StubConnection::boxed_of_kind("local", ConnectionKind::Local),
                CoreStatus::Disconnected,
            ))
        })
        .await
        .expect("the switch must succeed");

    assert!(handle.is_none());
}

/// The startup path announces too: the local connection is active from the
/// first moment, and becomes ready under `start`.
#[tokio::test]
async fn starting_the_local_connection_announces_it() {
    let (hook, mut announced) = recorder();
    let h = harness(
        StubConnection::boxed_of_kind("local", ConnectionKind::Local),
        Some(hook),
    );

    let handle = h
        .manager
        .start_active()
        .await
        .expect("a ready local connection must start an announce");
    handle.await.expect("the announce task must not panic");

    assert!(announced.recv().await.is_some());
}

#[tokio::test]
async fn starting_an_active_remote_connection_announces_nothing() {
    let (hook, _announced) = recorder();
    let h = harness(
        StubConnection::boxed_of_kind("home-lab", ConnectionKind::Remote),
        Some(hook),
    );

    assert!(h.manager.start_active().await.is_none());
}
