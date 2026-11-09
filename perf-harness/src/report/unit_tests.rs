use super::*;
use crate::manifest::Manifest;

fn manifest(workload: &str, nodes: u32, edges: u32, iterations: u32, every: Option<u32>, seed: u64) -> Manifest {
    // The schema asks for a checkpoint interval whenever the correctness mode is one of the modes.
    let modes = if every.is_some() {
        r#"["correctness", "performance"]"#
    } else {
        r#"["performance"]"#
    };
    let every = every.map_or(String::new(), |n| format!("checkpoint_every = {n}"));
    let text = format!(
        r#"
        schema_version = 1
        workload = "{workload}"
        seed = {seed}
        prng = "splitmix64"
        [graph]
        nodes = {nodes}
        edges = {edges}
        maximum_degree = 8
        [execution]
        modes = {modes}
        warmup_iterations = 2
        measurement_iterations = {iterations}
        repetitions = 3
        [measurement]
        capture_dom_counts = true
        capture_listener_tally = true
        capture_heap = false
        capture_wasm_capacity = true
        capture_wasm_live_allocation = true
        capture_frame_timing = false
        {every}
        [checkpoint]
        view = "unchanged"
        [limits]
        wall_ms = 1000
        "#
    );
    Manifest::from_toml(&text).expect("the test manifest is valid")
}

fn report(workload: &str, every: Option<u32>, seed: u64) -> Result<Report, String> {
    build_report(&manifest(workload, 12, 20, 6, every, seed), "test").map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn building_a_report_twice_gives_the_same_bytes() -> Result<(), String> {
    for workload in ["scene_construction", "connector_rerouting", "scene_lifecycle"] {
        let first = serde_json::to_string(&report(workload, Some(2), 11)?).map_err(|e| e.to_string())?;
        let second = serde_json::to_string(&report(workload, Some(2), 11)?).map_err(|e| e.to_string())?;
        if first != second {
            return Err(format!("{workload} reported differently the second time"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A performance run skips checkpoints, so its proof of having run the intended workload is the mutations digest, and
/// that must not depend on where the checkpoints are.
#[test]
fn the_mutations_digest_ignores_checkpoint_placement_and_the_full_digest_does_not() -> Result<(), String> {
    for workload in ["scene_construction", "connector_rerouting", "scene_lifecycle"] {
        let often = report(workload, Some(1), 5)?;
        let rarely = report(workload, Some(4), 5)?;
        let never = report(workload, None, 5)?;
        if often.stream.mutations_sha256 != rarely.stream.mutations_sha256
            || rarely.stream.mutations_sha256 != never.stream.mutations_sha256
        {
            return Err(format!("{workload}: checkpoint placement changed the mutations digest"));
        }
        if often.stream.sha256 == rarely.stream.sha256 || rarely.stream.sha256 == never.stream.sha256 {
            return Err(format!("{workload}: checkpoint placement did not change the full digest"));
        }
        if often.stream.mutations != rarely.stream.mutations {
            return Err(format!("{workload}: the mutation count depended on checkpoints"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn settings_that_do_not_change_the_stream_do_not_change_its_digests() -> Result<(), String> {
    let base = manifest("connector_rerouting", 12, 20, 6, Some(2), 5);
    let mut other = base.clone();
    other.execution.repetitions = 9;
    other.limits.wall_ms = 5;
    other.measurement.capture_heap = true;
    let (a, b) = (
        build_report(&base, "x").map_err(|e| e.to_string())?,
        build_report(&other, "x").map_err(|e| e.to_string())?,
    );
    if a.stream.sha256 != b.stream.sha256 || a.stream.mutations_sha256 != b.stream.mutations_sha256 {
        return Err("repetitions, limits or capture options changed the stream".to_string());
    }
    if a.manifest.sha256 == b.manifest.sha256 {
        return Err("those settings are part of the manifest, so its hash had to change".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_seed_and_the_graph_size_change_the_stream() -> Result<(), String> {
    let base = report("connector_rerouting", Some(2), 5)?.stream.mutations_sha256;
    let other_seed = report("connector_rerouting", Some(2), 6)?.stream.mutations_sha256;
    let bigger =
        build_report(&manifest("connector_rerouting", 13, 20, 6, Some(2), 5), "x").map_err(|e| e.to_string())?;
    if base == other_seed || base == bigger.stream.mutations_sha256 {
        return Err("a different seed or size gave the same mutations".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_checkpoints_are_numbered_in_order_and_point_at_checkpoint_operations() -> Result<(), String> {
    let r = report("connector_rerouting", Some(3), 5)?;
    for (position, checkpoint) in r.checkpoints.iter().enumerate() {
        if checkpoint.index != position as u64 {
            return Err(format!("checkpoint {position} is numbered {}", checkpoint.index));
        }
        if position > 0 && checkpoint.op_index <= r.checkpoints[position - 1].op_index {
            return Err("checkpoints are not in stream order".to_string());
        }
    }
    let last = r.checkpoints.last().ok_or("no checkpoints")?;
    if last.op_index != r.stream.operations - 1 {
        return Err("the last operation is not the last checkpoint".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_report_is_honest_about_what_the_model_does_not_predict() -> Result<(), String> {
    let r = report("scene_lifecycle", Some(2), 5)?;
    if !r.sections.not_modelled.contains(&"accessibility")
        || !r.sections.modelled.contains(&"geometry")
        || !r.sections.provisional.is_empty()
    {
        return Err(format!("{:?}", r.sections));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The fixed digests of a small manifest of each workload. They are written down so that a change to the generator, the
/// encoding, the model or a dependency shows at once, and so that a second machine can check that it agrees.
#[test]
fn the_digests_of_the_reference_workloads_are_fixed() -> Result<(), String> {
    let mut mismatches = Vec::new();
    for (file, operations, sha256, mutations_sha256, first_model, last_model) in GOLDEN {
        let manifest = Manifest::from_toml(file.text).map_err(|e| format!("{}: {e}", file.name))?;
        let r = build_report(&manifest, file.name).map_err(|e| e.to_string())?;
        let model =
            |checkpoint: Option<&CheckpointDigest>| checkpoint.map_or("", |c| c.model_sha256.as_str()).to_string();
        let got = (
            r.stream.operations,
            r.stream.sha256.clone(),
            r.stream.mutations_sha256.clone(),
            model(r.checkpoints.first()),
            model(r.checkpoints.last()),
        );
        let want = (
            *operations,
            sha256.to_string(),
            mutations_sha256.to_string(),
            first_model.to_string(),
            last_model.to_string(),
        );
        if got != want {
            mismatches.push(format!(
                "(\"{}\", {}, \"{}\", \"{}\", \"{}\", \"{}\"),",
                file.name, got.0, got.1, got.2, got.3, got.4
            ));
        }
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!("update GOLDEN if the change is intended:\n{}", mismatches.join("\n")))
    }
}

/// A manifest that lives in `perf/manifests/`, so that the digests are fixed for the file that a runner will be given.
struct ManifestFile {
    name: &'static str,
    text: &'static str,
}

const W1: ManifestFile = ManifestFile {
    name: "w1-scene-construction-small",
    text: include_str!("../../../perf/manifests/w1-scene-construction-small.toml"),
};
const W2: ManifestFile = ManifestFile {
    name: "w2-connector-rerouting-small",
    text: include_str!("../../../perf/manifests/w2-connector-rerouting-small.toml"),
};
const W3: ManifestFile = ManifestFile {
    name: "w3-scene-lifecycle-small",
    text: include_str!("../../../perf/manifests/w3-scene-lifecycle-small.toml"),
};

/// Manifest file, operations, full digest, mutations digest, and the digests of the first and of the last checkpoint,
/// for the small reference manifests: 12 nodes, 20 edges, 2 + 6 iterations, a checkpoint every 3 and the seed 2026. For
/// W1 and W3 the last checkpoint is the empty scene after the final removal, so the first, which follows the first
/// build, is what pins their model.
const GOLDEN: &[(&ManifestFile, u64, &str, &str, &str, &str)] = &[
    (
        &W1,
        286,
        "0db9d34a16b9bcc40520dc979f28b39e699199573d5f5b7679b2f32a9bf06cd7",
        "bf9d701c5cf78fe93186f08d357402a6b542e0c00489fc608f6e62816182114d",
        "0be9fb64ed2646758a02f7d4e64ba2e86554680c263488a60ea05ec280bb75d6",
        "82c804f325c0d737be08826b65193fe5c0ba9d77364c9472dbdd932f494b70d0",
    ),
    (
        &W2,
        47,
        "c0dedede4e64046f45e6480222439e76a786b9f13b208cb633e9043e91e40e3f",
        "61bc807a211f785252f9ed000c65684f351a1de3fa354fb7bbbee6ccf96af0e4",
        "0be9fb64ed2646758a02f7d4e64ba2e86554680c263488a60ea05ec280bb75d6",
        "86eabe866b7e4231af2dc3a6b3a82b8fca5360b9be7d90e15554f77d405f3cf4",
    ),
    (
        &W3,
        286,
        "c88f7e3dfbefb2bac81d92ca52dfe048e83188f30d79ea3226d9e702691d02b8",
        "fb473ca60eb10c5a1f6dfe49414786c3dbf2b2fc504a8708b8fc4447718aeca0",
        "2f304ac49b31928827916b2dc2ba2f05f7ad33885f6ac311b3bada8a297549b9",
        "82c804f325c0d737be08826b65193fe5c0ba9d77364c9472dbdd932f494b70d0",
    ),
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The report that this crate writes has to be a document that its own schema accepts, since runners read it.
///
/// The schema refers to the manifest schema by an identifier, so the test puts the manifest schema in where that
/// reference is. That keeps the check to the one validator without a registry of schemas.
#[cfg(feature = "schema")]
#[test]
fn a_report_satisfies_the_report_schema() -> Result<(), String> {
    let load = |text: &str| serde_json::from_str::<serde_json::Value>(text).map_err(|e| e.to_string());
    let mut schema = load(include_str!("../../../perf/schemas/workload-report.schema.json"))?;
    let mut manifest_schema = load(include_str!("../../../perf/schemas/manifest.schema.json"))?;
    if let Some(object) = manifest_schema.as_object_mut() {
        object.remove("$id");
        object.remove("$schema");
    }
    schema["properties"]["manifest"]["properties"]["canonical"] = manifest_schema;
    let validator = jsonschema::validator_for(&schema).map_err(|e| e.to_string())?;

    for workload in ["scene_construction", "connector_rerouting", "scene_lifecycle"] {
        let document = serde_json::to_value(report(workload, Some(3), 2026)?).map_err(|e| e.to_string())?;
        let violations: Vec<String> = validator
            .iter_errors(&document)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .collect();
        if !violations.is_empty() {
            return Err(format!("{workload}: {}", violations.join("; ")));
        }
    }
    Ok(())
}
