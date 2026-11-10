//! Maps the merged [`ArrowDetail`] (the six reads of one arrow) onto the small fact structs the
//! decisions in this module take. The view overlays the live runtime state on the detail first, then
//! calls these; nothing here reads a store.

use super::details::{CreditFacts, DetailsFacts, PortFacts, RequirementFacts};
use super::facts::{
    ActiveRunFacts, ArrowFacts, LifecycleFacts, MethodFacts, RunStepFacts, StepSpec, StepStatus,
};
use super::status::{LastRunFacts, RunState};
use crate::domain::StepStatus as WireStepStatus;
use crate::domain::{
    is_inferred, is_platform_supported, target_for_platform, ActiveRun, ArrowCredit, ArrowDetail,
    ArrowStepDefinition, Outcome, Overridable, SignalKind,
};

/// The value that applies on `platform`: the plain scalar, or the platform's entry, or `default`.
fn resolve<'a, T>(value: &'a Overridable<T>, platform: &str) -> Option<&'a T> {
    match value {
        Overridable::Plain(v) => Some(v),
        Overridable::PerPlatform(map) => map.get(platform).or_else(|| map.get("default")),
    }
}

fn text(value: &Overridable<String>, platform: &str) -> Option<String> {
    resolve(value, platform).filter(|v| !v.is_empty()).cloned()
}

fn signal_name(kind: SignalKind) -> &'static str {
    match kind {
        SignalKind::Graceful => "graceful",
        SignalKind::Kill => "kill",
        SignalKind::Interrupt => "interrupt",
    }
}

/// (kind, title, command, manifest fields) of one step before its YAML is rebuilt.
type StepParts<'a> = (
    &'a str,
    &'a str,
    Option<String>,
    Vec<(&'a str, Option<String>)>,
);

/// One declared step, with its manifest YAML rebuilt for the step inspector.
pub fn step_spec(step: &ArrowStepDefinition, platform: &str) -> StepSpec {
    use ArrowStepDefinition as S;
    let (kind, title, command, fields): StepParts<'_> = match step {
        S::Run {
            base,
            command,
            timeout,
            ..
        } => {
            let command = text(command, platform);
            (
                "run",
                &base.title,
                command.clone(),
                vec![("command", command), ("timeout", text(timeout, platform))],
            )
        }
        S::Fetch {
            base,
            url,
            to,
            checksum,
            ..
        } => (
            "fetch",
            &base.title,
            None,
            vec![
                ("url", text(url, platform)),
                ("to", text(to, platform)),
                ("checksum", text(checksum, platform)),
            ],
        ),
        S::Signal { base, signal, .. } => (
            "signal",
            &base.title,
            None,
            vec![(
                "signal",
                resolve(signal, platform).map(|s| signal_name(*s).to_owned()),
            )],
        ),
        S::Dependencies { base } => ("dependencies", &base.title, None, vec![]),
        S::Extract { base } => ("extract", &base.title, None, vec![]),
        S::Portable { base } => ("portable", &base.title, None, vec![]),
        S::Expose { base } => ("expose", &base.title, None, vec![]),
        S::Unexpose { base } => ("unexpose", &base.title, None, vec![]),
    };
    let mut code = format!("{kind}:\n  title: {title}");
    for (name, value) in fields {
        if let Some(value) = value {
            code.push_str(&format!("\n  {name}: {value}"));
        }
    }
    StepSpec {
        title: title.to_owned(),
        step_type: kind.to_owned(),
        command,
        code,
    }
}

fn steps(list: &[ArrowStepDefinition], platform: &str) -> Vec<StepSpec> {
    list.iter().map(|s| step_spec(s, platform)).collect()
}

fn run_status(status: WireStepStatus) -> StepStatus {
    match status {
        WireStepStatus::Pending => StepStatus::Pending,
        WireStepStatus::Running => StepStatus::Running,
        WireStepStatus::Completed => StepStatus::Completed,
        WireStepStatus::Failed => StepStatus::Failed,
    }
}

fn active_run(run: &ActiveRun) -> ActiveRunFacts {
    ActiveRunFacts {
        method: run.method.clone(),
        steps: run
            .steps
            .iter()
            .map(|s| RunStepFacts {
                title: s.title.clone(),
                step_type: s.step_type.clone(),
                status: run_status(s.status),
                error: s.error.clone(),
                note: s.note.clone(),
            })
            .collect(),
    }
}

/// Everything the header and action row need, for the machine at `platform` (`os/arch`).
pub fn facts_from_detail(detail: &ArrowDetail, platform: &str) -> ArrowFacts {
    let target = target_for_platform(&detail.targets, platform);
    ArrowFacts {
        namespace: detail.namespace.clone(),
        name: detail.name.clone(),
        description: detail.description.clone(),
        tags: detail.tags.clone(),
        icon: detail.media.icon.clone(),
        banner: detail.media.banner.clone(),
        license: detail.license.clone(),
        resolved_ref: detail.resolved_ref.clone(),
        state: RunState::from(detail.state),
        user_installed: detail.user_installed,
        available_ref: detail
            .available
            .as_ref()
            .map(|a| a.reference.clone())
            .filter(|r| !r.is_empty()),
        variable_names: detail.variables.iter().map(|v| v.name.clone()).collect(),
        lifecycle: target.map(|t| LifecycleFacts {
            install: steps(&t.lifecycle.install, platform),
            update: steps(&t.lifecycle.update, platform),
            execute: steps(&t.lifecycle.execute, platform),
            stop: steps(&t.lifecycle.stop, platform),
            uninstall: steps(&t.lifecycle.uninstall, platform),
        }),
        methods: target
            .map(|t| {
                t.methods
                    .iter()
                    .map(|m| MethodFacts {
                        name: m.name.clone(),
                        description: m.description.clone(),
                        steps: steps(&m.steps, platform),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        platform_supported: is_platform_supported(&detail.targets, platform),
        platforms: detail.targets.iter().map(|t| t.platform.clone()).collect(),
        active_run: detail.active_run.as_ref().map(active_run),
        inferred: is_inferred(detail.origin),
    }
}

/// The outcome of the last run, as the header's problem chip reads it.
pub fn last_run_facts(detail: &ArrowDetail) -> Option<LastRunFacts> {
    let last = detail.last_return.as_ref()?;
    Some(LastRunFacts {
        failed: last.outcome == Outcome::Failed,
        failed_step_error: last
            .steps
            .iter()
            .find(|s| s.status == WireStepStatus::Failed)
            .and_then(|s| s.error.clone()),
    })
}

fn credit(c: &ArrowCredit) -> CreditFacts {
    CreditFacts {
        name: c.name.clone(),
        email: c.email.clone(),
        url: c.url.clone(),
    }
}

/// The Details column's facts: requirements of the machine's target, ports, people, links.
pub fn details_facts(detail: &ArrowDetail, platform: &str) -> DetailsFacts {
    DetailsFacts {
        requirement: target_for_platform(&detail.targets, platform).map(|t| RequirementFacts {
            cpu_cores: t.requirement.cpu_cores,
            memory_gb: t.requirement.memory_gb,
            disk_gb: t.requirement.disk_gb,
        }),
        ports: detail
            .netbridge
            .iter()
            .map(|p| PortFacts {
                name: p.name.clone(),
                protocol: match p.protocol {
                    crate::domain::PortProtocol::Tcp => "tcp".into(),
                    crate::domain::PortProtocol::Udp => "udp".into(),
                },
                default: u16::try_from(p.default).unwrap_or(u16::MAX),
                required: p.required,
            })
            .collect(),
        maintainers: detail.maintainers.iter().map(credit).collect(),
        credits: detail.credits.iter().map(credit).collect(),
        url: detail.url.clone(),
        depends_on: detail.dependencies.len(),
        required_by: detail.dependents.len(),
    }
}

#[cfg(test)]
#[path = "from_detail_tests.rs"]
mod tests;
