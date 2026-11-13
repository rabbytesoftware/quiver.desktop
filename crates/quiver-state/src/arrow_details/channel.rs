//! Channel + version picking: which selector a new library entry follows (port of
//! `use-channel-selection.ts`). Picking changes nothing on core: a catalog row's selector never
//! changes, so a pick only becomes the identity of a row that does not exist yet.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChannelKind {
    /// Ranked tags (`members`, highest precedence first, `members[0] == latest`).
    Ordered,
    /// An unversioned branch/ref: `latest` is its only version.
    Pointer,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChannelFacts {
    pub name: String,
    pub kind: ChannelKind,
    pub latest: String,
    pub members: Vec<String>,
}

/// A channel's default version: its own `latest` for a pointer, the first member for an ordered one.
fn default_version(channel: &ChannelFacts) -> String {
    match channel.kind {
        ChannelKind::Ordered => channel.members.first().unwrap_or(&channel.latest).clone(),
        ChannelKind::Pointer => channel.latest.clone(),
    }
}

/// The selector a pick means. Keeping a channel's newest version follows the channel (`crowbar@stable`);
/// anything else pins exactly that ref (`crowbar@v1.2.0`).
pub fn selector_for(
    channel: Option<&ChannelFacts>,
    version: Option<&str>,
) -> Option<String> {
    let channel = channel?;
    match version {
        None => Some(channel.name.clone()),
        Some(v) if v == default_version(channel) => Some(channel.name.clone()),
        Some(v) => Some(v.to_owned()),
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ChannelSelection {
    channels: Vec<ChannelFacts>,
    selected_channel: Option<String>,
    selected_version: Option<String>,
}

impl ChannelSelection {
    /// Seeds from `selector` (the one the page's identity follows). Call again when `channels`
    /// arrives: they come from their own slower query and are empty on the first pass.
    pub fn seed(
        selector: &str,
        channels: Vec<ChannelFacts>,
    ) -> Self {
        let (channel, version) = if let Some(named) = channels.iter().find(|c| c.name == selector) {
            (Some(named.name.clone()), Some(default_version(named)))
        } else if let Some(listing) = channels
            .iter()
            .find(|c| c.members.iter().any(|m| m == selector))
        {
            (Some(listing.name.clone()), Some(selector.to_owned()))
        } else {
            // A constraint or commit no channel lists: nothing is picked until the person chooses.
            (None, None)
        };
        Self {
            channels,
            selected_channel: channel,
            selected_version: version,
        }
    }

    pub fn channels(&self) -> &[ChannelFacts] {
        &self.channels
    }

    pub fn selected_channel(&self) -> Option<&str> {
        self.selected_channel.as_deref()
    }

    pub fn selected_version(&self) -> Option<&str> {
        self.selected_version.as_deref()
    }

    pub fn selected_channel_entry(&self) -> Option<&ChannelFacts> {
        let name = self.selected_channel.as_deref()?;
        self.channels.iter().find(|c| c.name == name)
    }

    /// The selector the current pick registers as; `None` while there is nothing to pick from.
    pub fn selector(&self) -> Option<String> {
        selector_for(
            self.selected_channel_entry(),
            self.selected_version.as_deref(),
        )
    }

    /// Picks a channel, resetting the version to that channel's own newest.
    pub fn select_channel(
        &mut self,
        name: &str,
    ) {
        self.selected_channel = Some(name.to_owned());
        self.selected_version = self
            .channels
            .iter()
            .find(|c| c.name == name)
            .map(default_version);
    }

    pub fn select_version(
        &mut self,
        reference: &str,
    ) {
        self.selected_version = Some(reference.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stable() -> ChannelFacts {
        ChannelFacts {
            name: "stable".into(),
            kind: ChannelKind::Ordered,
            latest: "v2".into(),
            members: vec!["v2".into(), "v1".into()],
        }
    }

    fn nightly() -> ChannelFacts {
        ChannelFacts {
            name: "nightly".into(),
            kind: ChannelKind::Pointer,
            latest: "nightly-latest".into(),
            members: vec![],
        }
    }

    fn beta() -> ChannelFacts {
        ChannelFacts {
            name: "beta".into(),
            kind: ChannelKind::Ordered,
            latest: "v3-beta".into(),
            members: vec!["v3-beta".into()],
        }
    }

    #[test]
    fn the_channels_newest_follows_the_channel_and_older_versions_pin() {
        assert_eq!(
            selector_for(Some(&stable()), Some("v2")).as_deref(),
            Some("stable")
        );
        assert_eq!(
            selector_for(Some(&stable()), Some("v1")).as_deref(),
            Some("v1")
        );
    }

    #[test]
    fn a_pointer_follows_when_its_own_ref_is_kept_and_pins_anything_typed() {
        assert_eq!(
            selector_for(Some(&nightly()), Some("nightly-latest")).as_deref(),
            Some("nightly")
        );
        assert_eq!(
            selector_for(Some(&nightly()), Some("a1b2c3d")).as_deref(),
            Some("a1b2c3d")
        );
    }

    #[test]
    fn no_version_follows_the_channel_and_no_channel_says_nothing() {
        assert_eq!(
            selector_for(Some(&stable()), None).as_deref(),
            Some("stable")
        );
        assert_eq!(selector_for(None, Some("v1")), None);
    }

    #[test]
    fn seeds_from_the_channel_the_identity_follows() {
        let sel = ChannelSelection::seed("beta", vec![stable(), beta()]);
        assert_eq!(sel.selected_channel(), Some("beta"));
        assert_eq!(sel.selected_version(), Some("v3-beta"));
        assert_eq!(sel.selector().as_deref(), Some("beta"));
    }

    #[test]
    fn seeds_a_pinned_identity_onto_the_channel_that_lists_it() {
        let sel = ChannelSelection::seed("v1", vec![stable(), beta()]);
        assert_eq!(sel.selected_channel(), Some("stable"));
        assert_eq!(sel.selected_version(), Some("v1"));
        assert_eq!(sel.selector().as_deref(), Some("v1"));
    }

    #[test]
    fn a_selector_no_channel_lists_picks_nothing_until_channels_arrive() {
        let sel = ChannelSelection::seed("stable", vec![]);
        assert_eq!(sel.selector(), None);
        let sel = ChannelSelection::seed("v9.*", vec![stable()]);
        assert_eq!(sel.selected_channel(), None);
        assert_eq!(sel.selector(), None);
        assert_eq!(
            ChannelSelection::seed("stable", vec![stable()]).selected_channel(),
            Some("stable")
        );
    }

    #[test]
    fn picking_a_channel_resets_to_its_newest_and_a_version_pins() {
        let mut sel = ChannelSelection::seed("v1", vec![stable(), beta()]);
        sel.select_channel("beta");
        assert_eq!(sel.selected_version(), Some("v3-beta"));
        assert_eq!(sel.selector().as_deref(), Some("beta"));
        sel.select_channel("stable");
        sel.select_version("v1");
        assert_eq!(sel.selector().as_deref(), Some("v1"));
    }
}
