use crate::arrow_details::release_variables::is_self_arrow;
use crate::arrow_details::{compute_actions, ArrowActionKind, ArrowFacts};
use crate::i18n::Key;

use ArrowActionKind as K;

/// Actions the sidebar can run straight from the menu; the rest need the details page.
fn is_direct(kind: ArrowActionKind) -> bool {
    matches!(
        kind,
        K::Execute
            | K::Stop
            | K::Install
            | K::Reinstall
            | K::Update
            | K::Uninstall
            | K::RemoveFromLibrary
    )
}

fn is_release_action(kind: ArrowActionKind) -> bool {
    matches!(kind, K::Install | K::Reinstall | K::Update)
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ArrowMenuItem {
    pub kind: ArrowActionKind,
    pub label: Key,
    /// Needs input or sequencing the details page owns: the menu item navigates there instead of running.
    pub via_details: bool,
}

/// The menu mirrors the details page's own action set (`compute_actions`), so the two can never disagree
/// about what an arrow can do right now. An action that asks for variables, or is Quiver's own
/// self-update, is routed to the details page rather than guessed at.
pub fn arrow_menu_items(facts: &ArrowFacts) -> Vec<ArrowMenuItem> {
    compute_actions(facts)
        .into_iter()
        .filter(|action| !action.force_disabled)
        .map(|action| ArrowMenuItem {
            kind: action.kind,
            label: action.label,
            via_details: !is_direct(action.kind)
                || !action.uses_variables.is_empty()
                || (is_release_action(action.kind) && is_self_arrow(&facts.namespace)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_details::facts::fixtures::installed;
    use crate::arrow_details::RunState;

    #[test]
    fn mirrors_the_details_page_action_set() {
        let items = arrow_menu_items(&installed(RunState::Ready));
        let kinds: Vec<_> = items.iter().map(|i| i.kind).collect();
        assert_eq!(kinds, [K::Execute, K::Uninstall]);
        assert!(items.iter().all(|i| !i.via_details));
    }

    #[test]
    fn routes_an_action_that_needs_variables_through_the_details_page() {
        let mut facts = installed(RunState::Ready);
        facts.variable_names = vec!["server-name".into()];
        let items = arrow_menu_items(&facts);
        let via = |k| items.iter().find(|i| i.kind == k).map(|i| i.via_details);
        assert_eq!(via(K::Execute), Some(true));
        assert_eq!(via(K::Uninstall), Some(false));
    }

    #[test]
    fn routes_quivers_own_update_through_the_details_page() {
        let mut facts = installed(RunState::Ready);
        facts.namespace = "github.com/rabbytesoftware/quiver.desktop@stable".into();
        facts.available_ref = Some("v2".into());
        let items = arrow_menu_items(&facts);
        assert_eq!(
            items
                .iter()
                .find(|i| i.kind == K::Update)
                .map(|i| i.via_details),
            Some(true)
        );
    }
}
