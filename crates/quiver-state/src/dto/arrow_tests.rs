use serde_json::{json, Value};

use super::arrow::*;
use super::manifest::*;
use crate::domain::{
    ArrowOrigin, ArrowState, ChannelKind, DependencyType, InferenceConfidence, Outcome,
    SelectorKind, SurfaceMode,
};

fn items(value: Value) -> Vec<ArrowListResponseItemDto> {
    serde_json::from_value(value).unwrap()
}

fn detail_dto(extra: Value) -> ArrowDetailDto {
    let mut base = json!({
        "namespace": "github.com/rabbyte/minecraft@v1.21.4", "name": "Minecraft Server", "description": "A server.",
        "license": "MIT", "state": "ready", "tags": ["game"], "user_installed": true, "selector_kind": "pin",
        "resolved_ref": "v1.21.4", "installed_commit": "abc"
    });
    base.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    serde_json::from_value(base).unwrap()
}

fn manifest(extra: Value) -> ArrowManifestDto {
    let mut base = json!({
        "namespace": "github.com/rabbyte/minecraft", "name": "Minecraft Server", "description": "A server.",
        "tags": null, "variables": [{"name": "PORT", "description": "p", "type": "number", "default": "25565"}],
        "targets": {
            "linux/amd64": {
                "requirements": {"cpu_cores": 2, "memory_gb": 4, "disk_gb": 10},
                "lifecycle": {"install": [{"type": "dependencies", "title": "deps"}], "update": [], "execute": [], "stop": [], "uninstall": []},
                "methods": {"backup": {"name": "backup", "description": "b", "available_in": ["ready"], "steps": []},
                            "rcon": {"name": "rcon", "description": "r", "available_in": ["running"], "steps": []}}
            },
            "darwin/arm64": {"requirements": {"cpu_cores": 1, "memory_gb": 1, "disk_gb": 1},
                "lifecycle": {"install": [], "update": [], "execute": [], "stop": [], "uninstall": []}, "methods": null}
        },
        "manifest": {
            "metadata": {"url": "https://example.com", "maintainers": [{"name": "A", "email": "a@x"}],
                         "credits": [{"name": "C"}], "media": {"icon": "i.png"}},
            "netbridge": [{"name": "game", "protocol": "tcp", "default": 25565, "required": true}],
            "variables": null, "targets": null
        }
    });
    base.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    serde_json::from_value(base).unwrap()
}

fn detail(extra: Value) -> crate::domain::ArrowDetail {
    to_arrow_detail(
        detail_dto(extra),
        manifest(json!({})),
        vec![],
        None,
        vec![],
        vec![],
    )
}

#[test]
fn catalog_records_read_nested_media_and_stamp_the_connection() {
    let records = to_arrow_catalog_records(
        &items(
            json!([{"namespace": "github.com/a/b", "name": "B", "description": "d", "tags": ["t"],
            "media": {"icon": "i.png", "banner": ""}, "versions": [{"ref": "stable", "resolved_ref": "v1", "state": "ready"}]}]),
        ),
        "remote-1",
    );
    assert_eq!(records.len(), 1);
    let r = &records[0];
    assert_eq!(
        (
            r.connection_id.as_str(),
            r.namespace.as_str(),
            r.version.as_str()
        ),
        ("remote-1", "github.com/a/b@stable", "v1")
    );
    assert_eq!((r.icon.as_deref(), r.banner.clone()), (Some("i.png"), None));
}

#[test]
fn catalog_produces_one_record_per_version_and_reads_a_missing_resolved_ref_as_unresolved() {
    let list = items(json!([{"namespace": "n", "name": "N", "tags": null,
        "versions": [{"ref": "stable", "state": "ready"}, {"ref": "v1.*", "resolved_ref": "v1.4.0", "state": "absent"}]}]));
    let records = to_arrow_catalog_records(&list, "local");
    assert_eq!(
        records
            .iter()
            .map(|r| r.namespace.as_str())
            .collect::<Vec<_>>(),
        vec!["n@stable", "n@v1.*"]
    );
    assert_eq!(
        (records[0].version.as_str(), records[1].version.as_str()),
        ("", "v1.4.0")
    );
    assert!(records[0].tags.is_empty(), "null tags become an empty list");
    assert!(records[0].icon.is_none());
}

#[test]
fn catalog_origin_defaults_to_declared_and_drops_unknown_confidence() {
    let list = items(json!([
        {"namespace": "a", "name": "A", "versions": [{"ref": "s", "state": "ready"}], "origin": "inferred", "confidence": "high"},
        {"namespace": "b", "name": "B", "versions": [{"ref": "s", "state": "ready"}]},
        {"namespace": "c", "name": "C", "versions": [{"ref": "s", "state": "ready"}], "origin": "inferred", "confidence": "certain"}
    ]));
    let r = to_arrow_catalog_records(&list, "local");
    assert_eq!(
        (r[0].origin, r[0].confidence),
        (Some(ArrowOrigin::Inferred), Some(InferenceConfidence::High))
    );
    assert_eq!(
        (r[1].origin, r[1].confidence),
        (Some(ArrowOrigin::Declared), None)
    );
    assert_eq!(r[2].confidence, None);
}

#[test]
fn initial_runtime_updates_follow_the_catalog_scheme_and_null_the_runs() {
    let list = items(json!([{"namespace": "n", "name": "N", "versions": [
        {"ref": "stable", "state": "running"}, {"ref": "v1", "state": "absent"}]}]));
    let updates = to_initial_runtime_updates(&list);
    assert_eq!(updates.len(), 2);
    assert_eq!(
        (updates[0].namespace.as_str(), updates[0].state),
        ("n@stable", ArrowState::Running)
    );
    assert!(updates
        .iter()
        .all(|u| u.active_run.is_none() && u.last_return.is_none()));
}

#[test]
fn dependencies_cast_the_type_and_handle_empty() {
    let deps = to_arrow_dependencies(
        serde_json::from_value(
            json!([{"namespace": "a@1", "type": "tool"}, {"namespace": "b@1", "type": "service"}]),
        )
        .unwrap(),
    );
    assert_eq!(
        (deps[0].dep_type, deps[1].dep_type),
        (DependencyType::Tool, DependencyType::Service)
    );
    assert!(to_arrow_dependencies(vec![]).is_empty());
}

#[test]
fn channels_map_both_kinds_leaving_pointer_members_undefined() {
    let list: ChannelListDto = serde_json::from_value(json!({"channels": [
        {"name": "stable", "kind": "ordered", "latest": "v2", "count": 2, "members": ["v2", "v1"]},
        {"name": "main", "kind": "pointer", "latest": "main"}]}))
    .unwrap();
    let c = to_arrow_channels(list);
    assert_eq!(
        (c[0].kind, c[0].count, c[0].members.clone()),
        (
            ChannelKind::Ordered,
            Some(2),
            Some(vec!["v2".into(), "v1".into()])
        )
    );
    assert_eq!(
        (c[1].kind, c[1].count, c[1].members.clone()),
        (ChannelKind::Pointer, None, None)
    );
}

#[test]
fn detail_reads_the_row_state_and_what_is_ahead() {
    let d = detail(
        json!({"namespace": "x/y@stable", "selector_kind": "channel", "available": {"ref": "v1.22.0", "commit": "def"}, "outdated": true}),
    );
    assert_eq!(
        (
            d.selector.as_str(),
            d.selector_kind,
            d.resolved_ref.as_str(),
            d.installed_commit.as_str()
        ),
        ("stable", SelectorKind::Channel, "v1.21.4", "abc")
    );
    assert_eq!(
        d.available
            .as_ref()
            .map(|a| (a.reference.as_str(), a.commit.as_str())),
        Some(("v1.22.0", "def"))
    );
    assert!(d.outdated);
}

#[test]
fn detail_derives_outdated_from_available_when_the_payload_omits_it() {
    assert!(detail(json!({"available": {"ref": "v2", "commit": "c"}})).outdated);
    assert!(!detail(json!({})).outdated);
}

#[test]
fn detail_ignores_an_available_with_no_ref_and_reads_a_missing_commit_as_empty() {
    assert!(detail(json!({"available": {"ref": "", "commit": ""}}))
        .available
        .is_none());
    let d = detail(json!({"available": {"ref": "v2"}}));
    assert_eq!(d.available.unwrap().commit, "");
}

#[test]
fn detail_from_an_older_core_reads_as_a_pin_with_nothing_resolved() {
    let legacy: ArrowDetailDto = serde_json::from_value(json!({
        "namespace": "github.com/rabbyte/minecraft@v1.21.4", "name": "M", "description": "d",
        "state": "ready", "tags": null, "user_installed": true}))
    .unwrap();
    let mut m = manifest(json!({}));
    m.manifest = None;
    let d = to_arrow_detail(legacy, m, vec![], None, vec![], vec![]);
    assert_eq!(d.selector_kind, SelectorKind::Pin);
    assert_eq!(
        (
            d.resolved_ref.as_str(),
            d.installed_commit.as_str(),
            d.license.as_str(),
            d.url.as_str()
        ),
        ("", "", "", "")
    );
    assert!(d.available.is_none() && !d.outdated);
    assert_eq!((d.media.icon, d.media.banner), (None, None));
}

#[test]
fn detail_reads_an_unknown_selector_kind_as_a_pin() {
    assert_eq!(
        detail(json!({"selector_kind": "wildcard"})).selector_kind,
        SelectorKind::Pin
    );
}

#[test]
fn detail_sources_author_metadata_from_the_nested_manifest() {
    let d = detail(json!({"license": null}));
    assert_eq!(d.url, "https://example.com");
    assert_eq!(d.maintainers[0].email.as_deref(), Some("a@x"));
    assert_eq!(d.credits[0].name, "C");
    assert_eq!(
        (d.media.icon.as_deref(), d.media.banner),
        (Some("i.png"), None)
    );
    assert_eq!(d.netbridge[0].default, 25565);
    assert_eq!(d.license, "", "no license anywhere reads as empty");
    let with_meta_license = to_arrow_detail(
        detail_dto(json!({"license": null})),
        manifest(json!({"manifest": {"metadata": {"license": "GPL-3.0"}}})),
        vec![],
        None,
        vec![],
        vec![],
    );
    assert_eq!(with_meta_license.license, "GPL-3.0");
}

#[test]
fn detail_maps_each_target_in_wire_order_with_its_methods() {
    let d = detail(json!({}));
    assert_eq!(
        d.targets
            .iter()
            .map(|t| t.platform.as_str())
            .collect::<Vec<_>>(),
        vec!["linux/amd64", "darwin/arm64"]
    );
    assert_eq!(d.targets[0].requirement.cpu_cores, 2.0);
    assert_eq!(
        d.targets[0]
            .methods
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        vec!["backup", "rcon"]
    );
    assert!(d.targets[1].methods.is_empty(), "null methods read as none");
    assert_eq!(d.variables[0].name, "PORT");
}

#[test]
fn detail_reads_null_targets_and_null_lists_as_empty() {
    let d = to_arrow_detail(
        detail_dto(json!({"tags": null})),
        manifest(json!({"targets": null, "variables": null, "tags": null})),
        vec![],
        None,
        vec![],
        vec![],
    );
    assert!(d.targets.is_empty() && d.variables.is_empty() && d.tags.is_empty());
}

#[test]
fn detail_takes_channels_readme_dependencies_from_the_extra_arguments() {
    let chan: ChannelListDto = serde_json::from_value(
        json!({"channels": [{"name": "stable", "kind": "pointer", "latest": "x"}]}),
    )
    .unwrap();
    let d = to_arrow_detail(
        detail_dto(json!({})),
        manifest(json!({})),
        to_arrow_channels(chan),
        Some("# Hi".into()),
        serde_json::from_value(json!([{"namespace": "dep@1", "type": "service"}])).unwrap(),
        vec!["other@1".into()],
    );
    assert_eq!(d.channels.len(), 1);
    assert_eq!(d.readme.as_deref(), Some("# Hi"));
    assert_eq!(d.dependencies[0].dep_type, DependencyType::Service);
    assert_eq!(d.dependents, vec!["other@1"]);
    assert_eq!(detail(json!({})).readme, None);
}

#[test]
fn detail_reads_origin_confidence_and_warnings_from_the_inference_block() {
    let d = detail(
        json!({"origin": "inferred", "inference": {"generator": "fletcher/1", "confidence": "low", "warnings": ["assumed_arch"]}}),
    );
    assert_eq!(
        (d.origin, d.confidence),
        (Some(ArrowOrigin::Inferred), Some(InferenceConfidence::Low))
    );
    assert_eq!(d.warnings, vec!["assumed_arch"]);
    let missing = detail(json!({"origin": "inferred"}));
    assert_eq!(
        (missing.origin, missing.confidence),
        (Some(ArrowOrigin::Inferred), None)
    );
    let declared = detail(json!({}));
    assert_eq!(declared.origin, Some(ArrowOrigin::Declared));
    assert!(declared.warnings.is_empty());
}

#[test]
fn detail_preserves_active_run_surface_and_last_return_detail() {
    let d = detail(json!({
        "active_run": {"method": "execute", "variables": {}, "steps": [], "surface": {"mode": "static", "path": "/p", "ready": false}},
        "last_return": {"method": "install", "outcome": "failed", "variables": {"K": "v"},
                        "steps": [{"index": 0, "title": "t", "status": "failed", "type": "run", "error": "boom"}]}
    }));
    assert_eq!(
        d.active_run.unwrap().surface.unwrap().mode,
        SurfaceMode::Static
    );
    let lr = d.last_return.unwrap();
    assert_eq!(
        (lr.outcome, lr.steps[0].error.as_deref()),
        (Outcome::Failed, Some("boom"))
    );
    let none = detail(json!({"active_run": null, "last_return": null}));
    assert!(none.active_run.is_none() && none.last_return.is_none());
}

// Real quiver.core payloads (captured from a live daemon): see src/lib/core-store/dtos/v0/__fixtures__/live-core.
#[test]
fn maps_a_real_adopted_older_build_with_a_newer_release_available() {
    let dto: ArrowDetailDto = serde_json::from_value(json!({
        "namespace": "localhost/tester/desk@stable", "name": "e2e.rolling-tool", "license": "", "state": "outdated",
        "tags": null, "user_installed": true, "selector_kind": "channel", "resolved_ref": "stable-26.5.0",
        "installed_commit": "caa0b0f9", "available": {"ref": "stable-26.6.0", "commit": "9be6b3cb"}, "outdated": true,
        "description": "Rolling-tag fixture"
    })).unwrap();
    let d = to_arrow_detail(dto, manifest(json!({})), vec![], None, vec![], vec![]);
    assert_eq!(
        (
            d.selector_kind,
            d.resolved_ref.as_str(),
            d.state,
            d.outdated
        ),
        (
            SelectorKind::Channel,
            "stable-26.5.0",
            ArrowState::Outdated,
            true
        )
    );
    assert_eq!(d.available.unwrap().reference, "stable-26.6.0");
}
