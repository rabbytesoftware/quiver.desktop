//! Forgetting only the rows this app's own earlier announces registered.

use super::*;

fn nightly() -> String {
    id("nightly-rolling")
}

fn stable() -> String {
    id("stable")
}

#[tokio::test]
async fn records_the_identity_it_registered() {
    let daemon = Daemon::new();
    let record = MemoryAnnouncedRows::new();
    announce(&daemon, &config(None, "nightly-rolling"), Some(&record)).await;
    assert_eq!(record.snapshot(), [nightly()]);
}

#[tokio::test]
async fn registers_its_recorded_identity_again_after_the_user_removed_it_recording_it_once() {
    let daemon = Daemon::new();
    let record = MemoryAnnouncedRows::with(&[&nightly()]);
    announce(&daemon, &config(None, "nightly-rolling"), Some(&record)).await;
    assert_eq!(daemon.calls(), [post(&nightly())]);
    assert_eq!(record.snapshot(), [nightly()]);
}

#[tokio::test]
async fn records_nothing_for_an_identity_already_in_the_library() {
    let daemon = Daemon::new().with_row("nightly-rolling");
    let record = MemoryAnnouncedRows::new();
    announce(&daemon, &config(None, "nightly-rolling"), Some(&record)).await;
    assert!(record.snapshot().is_empty(), "that row is not its own");
}

#[tokio::test]
async fn removes_the_channel_row_an_earlier_build_of_another_channel_registered() {
    let daemon = Daemon::new()
        .with_row("nightly-rolling")
        .with_channels(&["stable", "nightly-rolling"]);
    let record = MemoryAnnouncedRows::with(&[&nightly()]);

    announce(
        &daemon,
        &config(Some("stable-26.5.0"), "stable"),
        Some(&record),
    )
    .await;

    assert_eq!(
        daemon.calls(),
        [post(&stable()), adopt(&stable()), delete(&nightly())]
    );
    assert_eq!(record.snapshot(), [stable()]);
}

#[tokio::test]
async fn never_removes_a_pin_or_channel_the_user_added_even_one_of_its_own_build_tag() {
    let daemon = Daemon::new()
        .with_row("stable-26.6")
        .with_row("nightly-rolling")
        .with_row("stable-26.4");
    announce(
        &daemon,
        &config(Some("stable-26.6"), ""),
        Some(&MemoryAnnouncedRows::new()),
    )
    .await;
    assert!(daemon.deletes().is_empty());
}

#[tokio::test]
async fn never_removes_the_identity_this_build_files_itself_under() {
    let daemon = Daemon::new().with_resolved_row("stable", "stable-26.5.0");
    let record = MemoryAnnouncedRows::with(&[&stable()]);
    announce(&daemon, &config(Some("stable-26.5.0"), ""), Some(&record)).await;
    assert!(daemon.deletes().is_empty());
    assert_eq!(record.snapshot(), [stable()]);
}

#[tokio::test]
async fn drops_a_recorded_identity_the_user_already_removed_without_a_call() {
    let daemon = Daemon::new().with_resolved_row("stable", "stable-26.5.0");
    let record = MemoryAnnouncedRows::with(&[&nightly(), &stable()]);
    announce(&daemon, &config(Some("stable-26.5.0"), ""), Some(&record)).await;
    assert!(daemon.deletes().is_empty());
    assert_eq!(record.snapshot(), [stable()]);
}

#[tokio::test]
async fn keeps_a_row_whose_removal_failed_on_record_and_tries_again_on_the_next_announce() {
    let daemon = Daemon::new()
        .with_resolved_row("stable", "stable-26.5.0")
        .with_row("nightly-rolling")
        .failing(delete(&nightly()), Failure::Status(422));
    let record = MemoryAnnouncedRows::with(&[&nightly()]);
    let cfg = config(Some("stable-26.5.0"), "");

    announce(&daemon, &cfg, Some(&record)).await;
    assert_eq!(record.snapshot(), [nightly()]);
    announce(&daemon, &cfg, Some(&record)).await;

    assert_eq!(daemon.deletes(), [delete(&nightly()), delete(&nightly())]);
}

#[tokio::test]
async fn drops_a_row_core_no_longer_has_404_from_the_record() {
    let daemon = Daemon::new()
        .with_resolved_row("stable", "stable-26.5.0")
        .with_row("nightly-rolling")
        .failing(delete(&nightly()), Failure::Status(404));
    let record = MemoryAnnouncedRows::with(&[&nightly()]);
    announce(&daemon, &config(Some("stable-26.5.0"), ""), Some(&record)).await;
    assert!(record.snapshot().is_empty());
}

#[tokio::test]
async fn still_announces_and_records_and_removes_nothing_when_there_is_no_record() {
    let daemon = Daemon::new()
        .with_row("nightly-rolling")
        .with_channels(&["stable", "nightly-rolling"]);
    announce(&daemon, &config(Some("stable-26.5.0"), "stable"), None).await;
    assert_eq!(daemon.calls(), [post(&stable()), adopt(&stable())]);
}

#[tokio::test]
async fn records_a_registered_identity_but_removes_nothing_when_the_adopt_fails() {
    let daemon = Daemon::new()
        .with_row("nightly-rolling")
        .failing(adopt(&stable()), Failure::Status(502));
    let record = MemoryAnnouncedRows::with(&[&nightly()]);
    announce(&daemon, &config(Some("stable-26.5.0"), ""), Some(&record)).await;
    assert!(daemon.deletes().is_empty());
    assert_eq!(record.snapshot(), [nightly(), stable()]);
}

#[tokio::test]
async fn removes_nothing_and_adopts_nothing_when_the_register_itself_fails() {
    let daemon = Daemon::new()
        .with_row("nightly-rolling")
        .failing(post(&stable()), Failure::Down);
    let record = MemoryAnnouncedRows::with(&[&nightly()]);
    announce(&daemon, &config(Some("stable-26.5.0"), ""), Some(&record)).await;
    assert_eq!(daemon.calls(), [post(&stable())]);
    assert_eq!(record.snapshot(), [nightly()]);
}

#[tokio::test]
async fn still_registers_and_adopts_when_the_catalog_cannot_be_listed_but_records_and_removes_nothing(
) {
    let daemon = Daemon::new().failing(format!("GET {LIST}"), Failure::Down);
    let record = MemoryAnnouncedRows::with(&[&nightly()]);
    announce(&daemon, &config(Some("stable-26.5.0"), ""), Some(&record)).await;
    // The row may have been the user's already: an unread catalog cannot say.
    assert_eq!(daemon.calls(), [post(&stable()), adopt(&stable())]);
    assert_eq!(record.snapshot(), [nightly()]);
}

#[tokio::test]
async fn records_and_removes_nothing_after_a_refless_register() {
    let daemon = Daemon::new().with_row("nightly-rolling").with_channels(&[]);
    let record = MemoryAnnouncedRows::with(&[&nightly()]);
    announce(&daemon, &config(None, ""), Some(&record)).await;
    assert_eq!(daemon.calls(), [post(NS)]);
    assert_eq!(record.snapshot(), [nightly()]);
}
