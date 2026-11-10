//! The saved choices that views outside Settings also depend on: whether the build indicator is shown
//! and whether Quiver's own components are listed.

use quiver_state::console::build_info::{
    describe_desktop, BuildChannel, BuildIndicatorSetting, DesktopStamp,
};
use quiver_state::persistence::KeyValueStore;
use quiver_state::settings::preferences::{load_build_indicator, load_show_self_components};

use super::AppModel;
use crate::services::AppServices;

/// The channel this desktop build was published on.
pub fn desktop_channel(services: &AppServices) -> BuildChannel {
    let info = services.release.build_info();
    describe_desktop(&DesktopStamp {
        channel: info.channel.clone(),
        label: info.stamp.label.clone(),
        commit: info.stamp.commit.clone(),
        built_at: info.stamp.built_at.and_then(|s| i64::try_from(s).ok()),
    })
    .channel
}

pub fn build_indicator(model: &AppModel) -> BuildIndicatorSetting {
    load_build_indicator(
        model.services.key_value.as_ref(),
        desktop_channel(&model.services),
    )
}

/// Quiver's own components show in lists only when the person asked for them.
pub fn initial_show_self_components(store: &dyn KeyValueStore) -> bool {
    load_show_self_components(store).unwrap_or(false)
}
