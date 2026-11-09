use super::*;

/// The result schema, with its reference to the manifest schema satisfied, as a validator that every builder is held
/// to.
#[cfg(feature = "schema")]
fn validate(metric: &Value) -> Result<(), String> {
    let load = |text: &str| serde_json::from_str::<Value>(text).map_err(|e| e.to_string());
    let mut schema = load(include_str!("../../../perf/schemas/result.schema.json"))?;
    let metric_schema = schema["properties"]["metrics"]["items"].clone();
    // Only the metric part of the result schema is checked here, so the rest of that schema is dropped.
    let mut standalone = metric_schema;
    standalone["$schema"] = json!("https://json-schema.org/draft/2020-12/schema");
    schema = standalone;
    let validator = jsonschema::validator_for(&schema).map_err(|e| e.to_string())?;
    let violations: Vec<String> = validator
        .iter_errors(metric)
        .map(|e| format!("{e} at {}", e.instance_path()))
        .collect();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!("{metric}: {}", violations.join("; ")))
    }
}

#[cfg(not(feature = "schema"))]
fn validate(_: &Value) -> Result<(), String> {
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_kind_of_metric_satisfies_the_result_schema() -> Result<(), String> {
    let few: Vec<f64> = (1..=40).map(f64::from).collect();
    let many: Vec<f64> = (1..=1200).map(f64::from).collect();
    for metric in [
        timing("mutation.move_node", Layer::B, "us", &few, None, "performance.now"),
        timing("mutation.move_node", Layer::B, "us", &many, Some(8), "performance.now"),
        timing(
            "rust.connector_endpoints",
            Layer::A,
            "us",
            &[0.2],
            Some(4000),
            "std::time::Instant",
        ),
        count("dom.elements", Layer::B, "count", 7000.0, "dom"),
        count("wasm.live_bytes", Layer::B, "bytes", 48128.0, "counting allocator"),
        capacity(
            "wasm.linear_memory_high_water",
            Layer::B,
            16_777_216.0,
            "WebAssembly.Memory.buffer.byteLength",
        ),
        unavailable("presentation.move_node", Layer::C, "no frame could be attributed"),
        measurement_error("heap", Layer::C, "the collection failed"),
        timing("nothing", Layer::B, "us", &[], None, "performance.now"),
    ] {
        validate(&metric)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_timing_is_a_latency_alone_and_a_batch_throughput_when_batched() -> Result<(), String> {
    let samples = [1.0, 2.0, 3.0];
    let alone = timing("m", Layer::B, "us", &samples, Some(1), "t");
    let batched = timing("m", Layer::B, "us", &samples, Some(10), "t");
    if alone["kind"] != "latency" || alone.get("batch_size").is_some() {
        return Err(format!("{alone}"));
    }
    if batched["kind"] != "batch_throughput" || batched["batch_size"] != 10 {
        return Err(format!("{batched}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn percentiles_appear_only_with_enough_observations() -> Result<(), String> {
    let few = timing("m", Layer::B, "us", &(1..=199).map(f64::from).collect::<Vec<_>>(), None, "t");
    let many = timing("m", Layer::B, "us", &(1..=1000).map(f64::from).collect::<Vec<_>>(), None, "t");
    if few["summary"].get("p95").is_some()
        || many["summary"].get("p95").is_none()
        || many["summary"].get("p99").is_none()
    {
        return Err(format!("{} {}", few["summary"], many["summary"]));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn no_observations_is_unavailable_and_never_a_made_up_figure() -> Result<(), String> {
    let metric = timing("m", Layer::B, "us", &[], None, "t");
    if metric["status"] != "unavailable" || metric.get("summary").is_some() || metric.get("reason").is_none() {
        return Err(format!("{metric}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_timing_under_fifty_timer_steps_is_flagged_and_a_longer_one_is_not() -> Result<(), String> {
    // A 5 us timer needs 250 us. 20 us is under it, and 400 us is over.
    let mut short = timing("m", Layer::B, "us", &[20.0, 25.0, 15.0], None, "t");
    let mut long = timing("m", Layer::B, "us", &[400.0, 410.0, 390.0], None, "t");
    let mut batched = timing("m", Layer::B, "us", &[20.0, 25.0, 15.0], Some(30), "t");
    for metric in [&mut short, &mut long, &mut batched] {
        mark_resolution_limited(metric, 5.0);
    }
    if short["resolution_limited"] != true
        || long.get("resolution_limited").is_some()
        || batched.get("resolution_limited").is_some()
    {
        return Err(format!("{short} {long} {batched}"));
    }
    validate(&short)?;
    // With no timer reading there is nothing to compare with, so nothing is flagged.
    let mut unknown = timing("m", Layer::B, "us", &[20.0], None, "t");
    mark_resolution_limited(&mut unknown, 0.0);
    if unknown.get("resolution_limited").is_some() {
        Err("a metric was flagged against a zero resolution".to_string())
    } else {
        Ok(())
    }
}
