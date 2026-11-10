use super::*;

const EXAMPLE: &str = include_str!("../../../perf/schemas/manifest.example.toml");

fn example() -> Result<Manifest, String> {
    Manifest::from_toml(EXAMPLE).map_err(|e| e.to_string())
}

/// The example with the value of one key replaced, so a case changes only the thing it is about. The first line that
/// starts with the key is the one changed, whatever its spacing, and a key the example does not have is an error.
fn with(key: &str, value: &str) -> Result<String, String> {
    let mut changed = false;
    let lines: Vec<String> = EXAMPLE
        .lines()
        .map(|line| {
            let starts = line
                .trim_start()
                .strip_prefix(key)
                .is_some_and(|rest| rest.trim_start().starts_with('='));
            if starts && !changed {
                changed = true;
                format!("{key} = {value}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if changed {
        Ok(lines.join("\n"))
    } else {
        Err(format!("the example has no key {key:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_annotated_example_is_a_valid_manifest() -> Result<(), String> {
    let manifest = example()?;
    if manifest.workload != Workload::ConnectorRerouting || manifest.seed != 12345 || manifest.graph.nodes != 1000 {
        return Err(format!("the example was read as {manifest:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The hash is of the typed fields, so a different layout of the same manifest gives the same hash.
#[test]
fn the_canonical_hash_ignores_comments_layout_and_key_order() -> Result<(), String> {
    let reordered = r#"
        seed = 12345
        prng = "splitmix64"
        workload = "connector_rerouting"
        schema_version = 1
        [limits]
        wall_ms = 120000
        dom_nodes = 50000
        [checkpoint]
        view = "unchanged"
        [measurement]
        checkpoint_every = 100
        capture_frame_timing = true
        capture_wasm_live_allocation = true
        capture_wasm_capacity = true
        capture_heap = false
        capture_listener_tally = true
        capture_dom_counts = true
        [execution]
        order = "interleaved"
        repetitions = 5
        measurement_iterations = 1000
        warmup_iterations = 100
        modes = ["correctness", "performance"]
        [graph]
        maximum_degree = 16
        edges = 3000
        nodes = 1000
    "#;
    let other = Manifest::from_toml(reordered).map_err(|e| e.to_string())?;
    if other.canonical_sha256() != example()?.canonical_sha256() {
        return Err("the same manifest in another layout hashed differently".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn changing_a_value_changes_the_hash() -> Result<(), String> {
    let base = example()?.canonical_sha256();
    for (key, value) in [("seed", "12346"), ("nodes", "1001")] {
        let changed = Manifest::from_toml(&with(key, value)?).map_err(|e| e.to_string())?;
        if changed.canonical_sha256() == base {
            return Err(format!("{key} = {value} left the hash as it was"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_manifest_that_breaks_the_rules_is_refused() -> Result<(), String> {
    let cases = [
        ("an unknown key", format!("typo = 1\n{EXAMPLE}")),
        ("an unknown workload", with("workload", "\"nothing\"")?),
        ("zero repetitions", with("repetitions", "0")?),
        ("a negative seed", with("seed", "-1")?),
        ("no nodes", with("nodes", "0")?),
        ("no measured iterations", with("measurement_iterations", "0")?),
        ("a wall limit of zero", with("wall_ms", "0")?),
        ("a checkpoint interval of zero", with("checkpoint_every", "0")?),
        ("no modes", with("modes", "[]")?),
        ("a repeated mode", with("modes", "[\"performance\", \"performance\"]")?),
        ("a wrong schema version", with("schema_version", "2")?),
        ("garbage", "this is not toml [".to_string()),
    ];
    for (label, text) in cases {
        if Manifest::from_toml(&text).is_ok() {
            return Err(format!("{label} was accepted"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn more_edges_than_the_nodes_can_hold_are_refused() -> Result<(), String> {
    let too_many_pairs = {
        let ten_nodes = with("nodes", "10")?;
        ten_nodes
            .lines()
            .map(|l| if l.trim_start().starts_with("edges") { "edges = 91" } else { l })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let over_the_degree = with("edges", "9000")?;
    for (label, text) in [
        ("more edges than ordered pairs", too_many_pairs),
        ("a degree total that does not fit", over_the_degree),
    ] {
        match Manifest::from_toml(&text) {
            Err(ManifestError::Semantic(_)) => {},
            other => return Err(format!("{label}: expected a semantic error, got {other:?}")),
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(feature = "schema")]
#[test]
fn a_schema_violation_names_where_it_is() -> Result<(), String> {
    let text = with("repetitions", "0")?;
    match Manifest::from_toml(&text) {
        Err(ManifestError::Schema(violations)) if violations.iter().any(|v| v.contains("repetitions")) => Ok(()),
        other => Err(format!("expected a schema error that names repetitions, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The schema is not there in a browser build, so the rules have to hold without it.
#[test]
fn the_conditions_of_the_schema_are_also_checked_without_it() -> Result<(), String> {
    let manifest = example()?;
    let mut no_policy = manifest.clone();
    no_policy.checkpoint = None;
    let mut no_interval = manifest.clone();
    no_interval.measurement.checkpoint_every = None;
    let mut no_edges = manifest.clone();
    no_edges.execution.modes = vec![Mode::Performance];
    no_edges.graph.edges = None;
    for (label, broken) in [("no policy", no_policy), ("no interval", no_interval), ("no edges", no_edges)] {
        let value = serde_json::to_value(&broken).map_err(|e| e.to_string())?;
        match broken.check_semantics() {
            Err(ManifestError::Semantic(_)) => {},
            other => return Err(format!("{label}: expected a semantic refusal, got {other:?} for {value}")),
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cases that the JavaScript schema check also reads, from `perf/schemas/examples/rejected-manifests.json`.
///
/// Each edits the example manifest and has to be refused. This holds with the schema feature on, and with it off, since
/// the typed fields and the semantic checks refuse every one of them too.
#[test]
fn every_shared_rejected_manifest_is_refused() -> Result<(), String> {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../perf/schemas/examples/rejected-manifests.json"))
            .map_err(|e| e.to_string())?;
    let cases = file["cases"].as_array().ok_or("the file has no cases")?;
    if cases.len() < 15 {
        return Err(format!("only {} shared cases were found", cases.len()));
    }
    let valid: serde_json::Value = toml::from_str(EXAMPLE).map_err(|e| e.to_string())?;

    for case in cases {
        let label = case["label"].as_str().ok_or("a case has no label")?;
        let mut document = valid.clone();
        for op in case["ops"].as_array().ok_or("a case has no operations")? {
            let (path, value) = match (op.get("set"), op.get("delete")) {
                (Some(path), _) => (path, Some(op["value"].clone())),
                (None, Some(path)) => (path, None),
                _ => return Err(format!("{label}: an operation is neither set nor delete")),
            };
            let keys: Vec<&str> = path
                .as_array()
                .ok_or("a path is not a list")?
                .iter()
                .filter_map(|k| k.as_str())
                .collect();
            let (last, parents) = keys.split_last().ok_or("an empty path")?;
            let mut target = &mut document;
            for key in parents {
                target = target.get_mut(*key).ok_or_else(|| format!("{label}: no key {key}"))?;
            }
            let object = target
                .as_object_mut()
                .ok_or_else(|| format!("{label}: the parent is not an object"))?;
            match value {
                Some(value) => object.insert((*last).to_string(), value),
                None => object.remove(*last),
            };
        }
        if Manifest::from_json(document).is_ok() {
            return Err(format!("{label} was accepted"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const RECOVERY: &str = include_str!("../../../perf/manifests/w5-recovery-small.toml");
const CONTRACTS: &str = include_str!("../../../perf/fault-contracts.json");
const SERVICE: &str = include_str!("../../../perf/manifests/w5-service-recovery-small.toml");
const SIMULATION: &str = include_str!("../../../perf/manifests/w6-service-simulation-smoke.toml");
const SIMULATION_100K: &str = include_str!("../../../perf/manifests/w6-service-simulation-100k.toml");
const SIMULATION_1K: &str = include_str!("../../../perf/manifests/w6-service-simulation-1k.toml");

/// The expectations in a recovery manifest are taken from the library's documented behaviour, which
/// `fault-contracts.json` writes down. So they must agree with it, for every fault point the manifest names.
#[test]
fn the_recovery_manifest_expects_what_the_contracts_say() -> Result<(), String> {
    let manifest = Manifest::from_toml(RECOVERY).map_err(|e| e.to_string())?;
    let recovery = manifest.recovery.ok_or("no recovery table")?;
    let contracts: serde_json::Value = serde_json::from_str(CONTRACTS).map_err(|e| e.to_string())?;
    // The contracts of the graph target, which is the one that a manifest names when it names no other. Those of the
    // service target have a manifest of their own, below.
    let points: Vec<serde_json::Value> = contracts["points"]
        .as_array()
        .ok_or("no points")?
        .iter()
        .filter(|point| point.get("target").is_none_or(|t| t == "graph"))
        .cloned()
        .collect();
    for entry in &recovery.points {
        let id = serde_json::to_value(entry.point).map_err(|e| e.to_string())?;
        let contract = points
            .iter()
            .find(|point| point["id"] == id)
            .ok_or_else(|| format!("{id} is not in the contracts"))?;
        let expected = serde_json::to_value(entry.expect).map_err(|e| e.to_string())?;
        if contract["contract"] != expected {
            return Err(format!(
                "{id}: the manifest expects {expected}, the contracts say {}",
                contract["contract"]
            ));
        }
    }
    if recovery.points.len() != points.len() {
        return Err(format!(
            "{} fault points in the manifest, {} in the contracts",
            recovery.points.len(),
            points.len()
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_bad_recovery_table_is_refused() -> Result<(), String> {
    let refused = |text: String, why: &str| match Manifest::from_toml(&text) {
        Ok(_) => Err(format!("{why} was accepted")),
        Err(_) => Ok(()),
    };
    refused(RECOVERY.replace("points = [", "points = [] #"), "an empty list of fault points")?;
    refused(
        RECOVERY.replace(
            "{ point = \"dom_transform\", expect = \"nothing_committed\" },",
            "{ point = \"node_construction\", expect = \"nothing_committed\" },",
        ),
        "a repeated fault point",
    )?;
    refused(
        RECOVERY.replace("edges = 18", "edges = 0"),
        "a scene with no connector to break",
    )?;
    refused(RECOVERY.replace("node_construction", "node_building"), "an unknown fault point")?;
    refused(RECOVERY.replace("nothing_committed", "all_committed"), "an unknown expectation")?;
    refused(
        RECOVERY.replace("workload = \"recovery\"", "workload = \"scene_lifecycle\""),
        "a recovery table on another workload",
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The recovery workload aimed at the simulator has no graph, and its expectations are the contracts of the service
/// target.
#[test]
fn the_service_recovery_manifest_is_read_and_expects_what_the_service_contracts_say() -> Result<(), String> {
    let manifest = Manifest::from_toml(SERVICE).map_err(|e| e.to_string())?;
    if !manifest.is_service_recovery() {
        return Err("the manifest is not the service target".to_owned());
    }
    let recovery = manifest.recovery.ok_or("no recovery table")?;
    if recovery.scenario.as_deref() != Some("E0") {
        return Err("the scenario was not read".to_owned());
    }
    let contracts: serde_json::Value = serde_json::from_str(CONTRACTS).map_err(|e| e.to_string())?;
    let service: Vec<&serde_json::Value> = contracts["points"]
        .as_array()
        .ok_or("no points")?
        .iter()
        .filter(|point| point["target"] == "service_simulation")
        .collect();
    if service.len() != recovery.points.len() {
        return Err(format!(
            "{} service points in the manifest, {} in the contracts",
            recovery.points.len(),
            service.len()
        ));
    }
    for entry in &recovery.points {
        let id = serde_json::to_value(entry.point).map_err(|e| e.to_string())?;
        let contract = service
            .iter()
            .find(|p| p["id"] == id)
            .ok_or_else(|| format!("{id} has no service contract"))?;
        let expected = serde_json::to_value(entry.expect).map_err(|e| e.to_string())?;
        if contract["contract"] != expected {
            return Err(format!(
                "{id}: the manifest expects {expected}, the contracts say {}",
                contract["contract"]
            ));
        }
    }
    Ok(())
}

/// A manifest of the service target is written back as it was read: it names its target, and it has no graph to write.
#[test]
fn a_service_manifest_round_trips_and_a_graph_manifest_does_not_gain_a_target() -> Result<(), String> {
    let service = Manifest::from_toml(SERVICE).map_err(|e| e.to_string())?;
    let json = String::from_utf8(service.canonical_json()).map_err(|e| e.to_string())?;
    if json.contains("\"graph\"") || !json.contains("\"target\":\"service_simulation\"") {
        return Err(format!("the canonical form of a service manifest is wrong: {json}"));
    }
    let again =
        Manifest::from_json(serde_json::from_str(&json).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    if again != service {
        return Err("a service manifest changed when it was written and read".to_owned());
    }
    let graph = Manifest::from_toml(RECOVERY).map_err(|e| e.to_string())?;
    let graph_json = String::from_utf8(graph.canonical_json()).map_err(|e| e.to_string())?;
    if graph_json.contains("\"target\"") || graph_json.contains("\"scenario\"") {
        return Err(
            "a graph manifest gained a target or a scenario in its canonical form, so every digest of one would change"
                .to_owned(),
        );
    }
    Ok(())
}

#[test]
fn a_bad_service_manifest_is_refused() -> Result<(), String> {
    let refused = |text: String, why: &str| match Manifest::from_toml(&text) {
        Ok(_) => Err(format!("{why} was accepted")),
        Err(_) => Ok(()),
    };
    refused(SERVICE.replace("service_simulation", "service_simulator"), "a mistyped target")?;
    refused(
        SERVICE.replace("scenario = \"E0\"\n", ""),
        "the service target with no scenario",
    )?;
    refused(SERVICE.replace("scenario = \"E0\"", "scenario = \"\""), "an empty scenario")?;
    refused(
        SERVICE.replace("service_overlay_write", "node_construction"),
        "a graph point on the service target",
    )?;
    refused(
        RECOVERY.replace("[recovery]", "[recovery]\nscenario = \"E0\""),
        "a scenario on the graph target",
    )?;
    refused(
        RECOVERY.replace("node_construction", "service_overlay_write"),
        "a service point on the graph target",
    )?;
    refused(
        SERVICE.replace("[execution]", "[graph]\nnodes = 12\nedges = 18\n\n[execution]"),
        "a graph with the service target",
    )?;
    refused(
        SERVICE.replace("[recovery]", "[recovery]\ntarjet = \"graph\""),
        "a mistyped key",
    )?;
    // A graph manifest still needs its graph.
    refused(
        RECOVERY.replace("[graph]\nnodes = 12\nedges = 18\nmaximum_degree = 6\n", ""),
        "the graph target with no graph",
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The simulator workload (W6) has no graph, no checkpoint policy and a frozen generator, and its canonical form says
/// so.
#[test]
fn the_simulator_workload_is_read_and_written_back_as_it_was() -> Result<(), String> {
    let manifest = Manifest::from_toml(SIMULATION).map_err(|e| e.to_string())?;
    if manifest.workload != Workload::ServiceSimulation || !manifest.has_no_graph() {
        return Err("the manifest is not the simulator workload, or it has a graph".to_owned());
    }
    let simulation = manifest.simulation.clone().ok_or("no [simulation] table")?;
    if (simulation.servers, simulation.queue_capacity, simulation.batch_slices) != (3, 12, 4) {
        return Err("the simulation table was not read".to_owned());
    }
    let json = String::from_utf8(manifest.canonical_json()).map_err(|e| e.to_string())?;
    if json.contains("\"graph\"") || json.contains("\"checkpoint\"") {
        return Err(format!(
            "a simulator manifest was written with a graph or a checkpoint policy: {json}"
        ));
    }
    let again =
        Manifest::from_json(serde_json::from_str(&json).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    if again != manifest {
        return Err("a simulator manifest changed when it was written and read".to_owned());
    }
    // A graph workload is not touched: it still has a graph and writes no simulation table.
    let graph = Manifest::from_toml(RECOVERY).map_err(|e| e.to_string())?;
    let graph_json = String::from_utf8(graph.canonical_json()).map_err(|e| e.to_string())?;
    if graph_json.contains("\"simulation\"") {
        return Err("a graph manifest gained a simulation table, so every digest of one would change".to_owned());
    }
    Ok(())
}

#[test]
fn a_bad_simulator_manifest_is_refused() -> Result<(), String> {
    let refused = |text: String, why: &str| match Manifest::from_toml(&text) {
        Ok(_) => Err(format!("{why} was accepted")),
        Err(_) => Ok(()),
    };
    refused(
        SIMULATION.replace("mixed_outage", "mixed_outrage"),
        "a scenario with no frozen revision",
    )?;
    refused(
        SIMULATION.replace("servers = 3", "servers = 4"),
        "a parameter of another revision",
    )?;
    refused(
        SIMULATION.replace("queue_capacity = 12", "queue_capacity = 13"),
        "another queue capacity",
    )?;
    refused(
        SIMULATION.replace("batch_slices = 4", "batch_slices = 3"),
        "a batch of three slices",
    )?;
    refused(SIMULATION.replace("min_pops = 10000", "min_pops = 0"), "no pop minimum")?;
    refused(
        SIMULATION.replace("measurement_iterations = 60", "measurement_iterations = 61"),
        "a measured window that would be wrapped",
    )?;
    refused(
        SIMULATION.replace("warmup_iterations = 10", "warmup_iterations = 11"),
        "a warm-up window that would be wrapped",
    )?;
    refused(
        SIMULATION.replace("\"endurance\"]", "\"endurance\", \"recovery\"]"),
        "the recovery mode, which is W5's",
    )?;
    refused(
        SIMULATION.replace("[simulation]", "[graph]\nnodes = 4\n\n[simulation]"),
        "a graph",
    )?;
    refused(
        SIMULATION.replace("[execution]", "[checkpoint]\nview = \"unchanged\"\n\n[execution]"),
        "a checkpoint policy",
    )?;
    refused(
        SIMULATION.replace("batch_slices = 4", "batch_slices = 4\nspeed = 1"),
        "a mistyped key",
    )?;
    refused(
        SIMULATION.replace("service_simulation", "scene_construction"),
        "a simulation table on a graph workload",
    )?;
    // The simulator workload needs its table.
    let without: String = SIMULATION
        .lines()
        .filter(|line| {
            ![
                "scenario",
                "servers",
                "queue_capacity",
                "max_attempts",
                "retry_deferral_cap",
                "min_pops",
                "batch_slices",
                "[simulation]",
            ]
            .iter()
            .any(|key| line.starts_with(key))
        })
        .collect::<Vec<_>>()
        .join("\n");
    refused(without, "the simulator workload with no table")?;
    Ok(())
}

/// A scaled run of the frozen revision says how many arrivals it has, and a manifest that does not say writes nothing
/// about it, so every digest of a manifest from before the key is what it was.
#[test]
fn scaled_simulator_manifests_say_their_arrivals_and_the_smoke_one_says_none() -> Result<(), String> {
    let smoke = Manifest::from_toml(SIMULATION).map_err(|e| e.to_string())?;
    let smoke_json = String::from_utf8(smoke.canonical_json()).map_err(|e| e.to_string())?;
    if smoke_json.contains("arrivals") {
        return Err(format!("the smoke manifest was written with an arrivals key: {smoke_json}"));
    }
    for (text, arrivals) in [(SIMULATION_100K, 36_000), (SIMULATION_1K, 360)] {
        let manifest = Manifest::from_toml(text).map_err(|e| e.to_string())?;
        if manifest.simulation.as_ref().and_then(|s| s.arrivals) != Some(arrivals) {
            return Err(format!("the arrivals {arrivals} were not read"));
        }
        let json = String::from_utf8(manifest.canonical_json()).map_err(|e| e.to_string())?;
        let again =
            Manifest::from_json(serde_json::from_str(&json).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        if again != manifest {
            return Err("a scaled manifest changed when it was written and read".to_owned());
        }
    }
    let refused = |text: String, why: &str| match Manifest::from_toml(&text) {
        Ok(_) => Err(format!("{why} was accepted")),
        Err(_) => Ok(()),
    };
    refused(SIMULATION_100K.replace("arrivals = 36000", "arrivals = 10"), "too few arrivals")?;
    refused(
        SIMULATION_100K.replace("arrivals = 36000", "arrivals = 2000000"),
        "too many arrivals",
    )?;
    Ok(())
}
