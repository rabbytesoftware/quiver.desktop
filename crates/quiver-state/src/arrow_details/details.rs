//! The Details column (right of the README): requirements, network, maintainers, credits, links.
//! Every section is optional: omitted entirely (header included) rather than shown empty.

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct RequirementFacts {
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub disk_gb: f64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PortFacts {
    pub name: String,
    pub protocol: String,
    pub default: u16,
    pub required: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CreditFacts {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct DetailsFacts {
    pub requirement: Option<RequirementFacts>,
    pub ports: Vec<PortFacts>,
    pub maintainers: Vec<CreditFacts>,
    pub credits: Vec<CreditFacts>,
    pub url: String,
    pub depends_on: usize,
    pub required_by: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DetailsSection {
    Requirements,
    Dependencies,
    Network,
    Maintainers,
    Credits,
    Links,
}

/// The sections to draw, in order. Requirements need at least one non-zero figure.
pub fn visible_sections(d: &DetailsFacts) -> Vec<DetailsSection> {
    let mut out = Vec::new();
    if d.requirement
        .is_some_and(|r| r.cpu_cores > 0.0 || r.memory_gb > 0.0 || r.disk_gb > 0.0)
    {
        out.push(DetailsSection::Requirements);
    }
    if d.depends_on > 0 {
        out.push(DetailsSection::Dependencies);
    }
    if !d.ports.is_empty() {
        out.push(DetailsSection::Network);
    }
    if !d.maintainers.is_empty() {
        out.push(DetailsSection::Maintainers);
    }
    if !d.credits.is_empty() {
        out.push(DetailsSection::Credits);
    }
    if !d.url.is_empty() {
        out.push(DetailsSection::Links);
    }
    out
}

/// The avatar letter of a credit row.
pub fn credit_initial(credit: &CreditFacts) -> String {
    credit
        .name
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default()
}

/// A stable identity for a credit row that does not depend on its position.
pub fn credit_key(credit: &CreditFacts) -> String {
    format!(
        "{}|{}|{}",
        credit.name,
        credit.email.as_deref().unwrap_or(""),
        credit.url.as_deref().unwrap_or("")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_details_show_no_section_at_all() {
        assert!(visible_sections(&DetailsFacts::default()).is_empty());
    }

    #[test]
    fn zeroed_requirements_are_not_a_section() {
        let d = DetailsFacts {
            requirement: Some(RequirementFacts::default()),
            ..DetailsFacts::default()
        };
        assert!(visible_sections(&d).is_empty());
        let d = DetailsFacts {
            requirement: Some(RequirementFacts {
                disk_gb: 10.0,
                ..Default::default()
            }),
            ..d
        };
        assert_eq!(visible_sections(&d), [DetailsSection::Requirements]);
    }

    #[test]
    fn sections_keep_their_order() {
        let credit = CreditFacts {
            name: "Rabbyte".into(),
            ..CreditFacts::default()
        };
        let d = DetailsFacts {
            requirement: Some(RequirementFacts {
                cpu_cores: 2.0,
                ..Default::default()
            }),
            ports: vec![PortFacts {
                name: "game".into(),
                protocol: "tcp".into(),
                default: 7777,
                required: true,
            }],
            maintainers: vec![credit.clone()],
            credits: vec![credit],
            url: "github.com/x".into(),
            depends_on: 1,
            required_by: 0,
        };
        use DetailsSection::*;
        assert_eq!(
            visible_sections(&d),
            [
                Requirements,
                Dependencies,
                Network,
                Maintainers,
                Credits,
                Links
            ]
        );
    }

    #[test]
    fn credits_have_an_initial_and_a_position_independent_key() {
        let c = CreditFacts {
            name: "rabbyte".into(),
            email: None,
            url: Some("u".into()),
        };
        assert_eq!(credit_initial(&c), "R");
        assert_eq!(credit_key(&c), "rabbyte||u");
        assert_eq!(credit_initial(&CreditFacts::default()), "");
    }
}
