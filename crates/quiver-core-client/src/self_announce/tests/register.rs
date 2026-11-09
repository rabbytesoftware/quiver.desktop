//! Registering and adopting, ported from the first three `describe` blocks.

use super::*;

mod tagged {
    use super::*;

    #[tokio::test]
    async fn registers_the_stable_channel_then_adopts_the_tag_it_was_built_from() {
        let daemon = Daemon::new();
        let outcome = announce(&daemon, &config(Some("stable-26.5.0"), ""), None).await;

        assert_eq!(daemon.calls(), [post(&id("stable")), adopt(&id("stable"))]);
        assert_eq!(
            daemon.body_of("/adopt"),
            Some(serde_json::json!({ "resolved_ref": "stable-26.5.0" }))
        );
        assert!(
            outcome.adopted,
            "the host must be told to re-read the catalog"
        );
    }

    #[tokio::test]
    async fn sends_nothing_but_the_listing_when_the_same_build_announces_again() {
        let daemon = Daemon::new().with_resolved_row("stable", "stable-26.5.0");
        let outcome = announce(&daemon, &config(Some("stable-26.5.0"), ""), None).await;

        assert_eq!(daemon.all_calls(), [format!("GET {LIST}")]);
        assert!(!outcome.adopted);
    }

    #[tokio::test]
    async fn advances_the_same_identity_to_a_newer_build_through_adopt_never_a_new_row() {
        let daemon = Daemon::new().with_resolved_row("stable", "stable-26.5.0");
        announce(&daemon, &config(Some("stable-26.9.0"), ""), None).await;

        assert_eq!(daemon.calls(), [adopt(&id("stable"))]);
        assert_eq!(
            daemon.body_of("/adopt"),
            Some(serde_json::json!({ "resolved_ref": "stable-26.9.0" }))
        );
    }

    #[tokio::test]
    async fn leaves_the_plain_registration_and_forgets_nothing_when_core_refuses_the_tag() {
        for status in [400, 404] {
            let daemon = Daemon::new()
                .with_row("stable-26.4")
                .failing(adopt(&id("stable")), Failure::Status(status));
            let record = MemoryAnnouncedRows::with(&[&id("nightly-rolling")]);

            let outcome =
                announce(&daemon, &config(Some("stable-26.5.0"), ""), Some(&record)).await;

            assert!(!outcome.adopted);
            assert!(daemon.deletes().is_empty(), "status {status}");
        }
    }

    #[tokio::test]
    async fn any_other_adopt_failure_forgets_nothing_and_still_returns() {
        let daemon = Daemon::new()
            .with_row("stable-26.4")
            .failing(adopt(&id("stable")), Failure::Status(502));
        let record = MemoryAnnouncedRows::with(&[&id("nightly-rolling")]);

        announce(&daemon, &config(Some("stable-26.5.0"), ""), Some(&record)).await;

        assert!(daemon.deletes().is_empty());
    }
}

mod untagged {
    use super::*;

    #[tokio::test]
    async fn registers_the_repositorys_default_channel_explicitly_and_adopts_nothing() {
        let daemon = Daemon::new();
        announce(&daemon, &config(None, ""), None).await;
        assert_eq!(daemon.calls(), [post(&id("nightly-rolling"))]);
    }

    #[tokio::test]
    async fn treats_an_empty_tag_as_no_tag() {
        let daemon = Daemon::new();
        announce(&daemon, &config(Some(""), ""), None).await;
        assert_eq!(daemon.calls(), [post(&id("nightly-rolling"))]);
    }

    #[tokio::test]
    async fn does_not_register_again_when_the_default_channel_is_already_in_the_library() {
        let daemon = Daemon::new().with_row("nightly-rolling");
        announce(&daemon, &config(None, ""), None).await;
        assert!(daemon.calls().is_empty());
    }

    #[tokio::test]
    async fn registers_refless_and_forgets_nothing_when_the_channels_cannot_be_read() {
        let daemon = Daemon::new()
            .with_row("stable-26.4")
            .failing(format!("GET /v0/arrow/{}/channels", enc(NS)), Failure::Down);
        announce(&daemon, &config(None, ""), None).await;
        assert_eq!(daemon.calls(), [post(NS)]);
    }

    #[tokio::test]
    async fn registers_refless_when_the_repository_publishes_no_channel() {
        let daemon = Daemon::new().with_row("stable-26.4").with_channels(&[]);
        announce(&daemon, &config(None, ""), None).await;
        assert_eq!(daemon.calls(), [post(NS)]);
    }
}

mod named_channel {
    use super::*;

    #[tokio::test]
    async fn registers_a_nightly_build_under_its_own_channel_not_the_stable_default() {
        let daemon = Daemon::new().with_channels(&["stable", "nightly-rolling"]);
        announce(&daemon, &config(None, "nightly-rolling"), None).await;
        assert_eq!(daemon.calls(), [post(&id("nightly-rolling"))]);
    }

    #[tokio::test]
    async fn registers_a_beta_build_under_beta_and_leaves_the_stable_row_alone() {
        let daemon = Daemon::new()
            .with_resolved_row("stable", "stable-26.5")
            .with_channels(&["stable", "beta"]);
        announce(&daemon, &config(None, " beta "), None).await;
        assert_eq!(daemon.calls(), [post(&id("beta"))]);
    }

    #[tokio::test]
    async fn still_adopts_the_tag_of_a_stable_build_that_names_its_channel() {
        let daemon = Daemon::new().with_channels(&["stable", "nightly-rolling"]);
        announce(&daemon, &config(Some("stable-26.5.0"), "stable"), None).await;
        assert_eq!(daemon.calls(), [post(&id("stable")), adopt(&id("stable"))]);
    }

    #[tokio::test]
    async fn falls_back_to_the_default_channel_while_the_repository_publishes_none_of_that_name() {
        let daemon = Daemon::new();
        announce(&daemon, &config(None, "beta"), None).await;
        assert_eq!(daemon.calls(), [post(&id("nightly-rolling"))]);
    }

    #[tokio::test]
    async fn falls_back_to_stable_for_a_tagged_build_whose_channel_is_not_published_yet() {
        let daemon = Daemon::new().with_channels(&["stable"]);
        announce(&daemon, &config(Some("stable-26.5.0"), "beta"), None).await;
        assert_eq!(daemon.calls(), [post(&id("stable")), adopt(&id("stable"))]);
    }

    #[tokio::test]
    async fn registers_the_named_channel_when_the_channels_cannot_be_read() {
        let daemon =
            Daemon::new().failing(format!("GET /v0/arrow/{}/channels", enc(NS)), Failure::Down);
        announce(&daemon, &config(None, "nightly-rolling"), None).await;
        assert_eq!(daemon.calls(), [post(&id("nightly-rolling"))]);
    }
}
