//! Everything the page draws, decided once per frame from the merged detail.

use quiver_state::arrow_details::details::DetailsFacts;
use quiver_state::arrow_details::glance::{body_plan, BodyPlan};
use quiver_state::arrow_details::{
    build_action_row, build_header, details_facts, facts_from_detail, last_run_facts, ActionRow,
    ArrowFacts, HeaderContext, HeaderModel,
};
use quiver_state::domain::ArrowDetail;

use super::ArrowDetailsView;

pub(super) struct Page {
    pub detail: ArrowDetail,
    pub facts: ArrowFacts,
    pub header: HeaderModel,
    pub row: ActionRow,
    pub details: DetailsFacts,
    pub plan: BodyPlan,
}

impl Page {
    pub fn build(detail: &ArrowDetail, view: &ArrowDetailsView) -> Self {
        let platform = quiver_platform::platform_key();
        let facts = facts_from_detail(detail, &platform);
        let header = build_header(
            &facts,
            &HeaderContext {
                platform: &platform,
                // The native side knows its own platform: no user-agent guess to wait out.
                platform_resolved: true,
                has_channels: false,
                channels_loading: false,
                last_run: last_run_facts(detail),
                switching: false,
                selector: &detail.selector,
            },
        );
        let row = build_action_row(&facts, view.invoke.pending, false);
        let plan = body_plan(&facts, detail.readme.as_deref(), &detail.url);
        Self {
            details: details_facts(detail, &platform),
            detail: detail.clone(),
            facts,
            header,
            row,
            plan,
        }
    }
}

impl Page {
    /// The sentence behind the header's problem chip.
    pub fn problem_text(&self, tr: quiver_state::i18n::Translator) -> Option<String> {
        self.header
            .problem
            .as_ref()
            .map(|p| quiver_state::arrow_details::header::problem_text(p, &tr))
    }
}
