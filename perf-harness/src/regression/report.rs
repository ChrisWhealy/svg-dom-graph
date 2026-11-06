//! Reads the results of two collections of runs and writes the comparison report that `comparison.schema.json` describes.
//!
//! A collection is the runs of one layer and one mode of one manifest. Before any number is compared the two collections are
//! checked to have been measured alike: the same workload (the manifest and the operation stream), and the same environment key
//! of operating system, processor, runner class, browser version and measurement protocol. If they differ, every metric is
//! reported as not comparable and no figure is compared, since a difference between two machines is not a difference between
//! two versions of the code.
//!
//! A run that is not valid is not a slow run. A collection that holds one is refused, so a timing is never compared when the
//! workload behind it did not do what it was meant to.

use super::{Comparison, Settings, Statistic, compare, not_comparable, to_json};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What has to be the same for two runs to be compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub os: String,
    pub cpu: String,
    pub runner_class: String,
    pub browser_version: String,
    pub protocol_version: String,
}

impl Key {
    /// A name for the key that is safe in a file name.
    pub fn slug(&self) -> String {
        let raw = format!(
            "{}-{}-{}-{}-p{}",
            self.runner_class, self.os, self.cpu, self.browser_version, self.protocol_version
        );
        let mut slug = String::new();
        for c in raw.chars() {
            let c = if c.is_ascii_alphanumeric() || c == '.' { c.to_ascii_lowercase() } else { '-' };
            if c != '-' || !slug.ends_with('-') {
                slug.push(c);
            }
        }
        slug.trim_matches('-').to_string()
    }

    fn differences(&self, other: &Self) -> Vec<String> {
        [
            ("operating system", &self.os, &other.os),
            ("processor", &self.cpu, &other.cpu),
            ("runner class", &self.runner_class, &other.runner_class),
            ("browser version", &self.browser_version, &other.browser_version),
            ("measurement protocol", &self.protocol_version, &other.protocol_version),
        ]
        .into_iter()
        .filter(|(_, a, b)| a != b)
        .map(|(what, a, b)| format!("{what} ({a} against {b})"))
        .collect()
    }

    fn to_json(&self) -> Value {
        json!({"os": self.os, "cpu": self.cpu, "runner_class": self.runner_class,
               "browser_version": self.browser_version, "protocol_version": self.protocol_version})
    }
}

/// One run: its identity, and the observations of every metric that kept them.
#[derive(Debug, Clone)]
pub struct Run {
    pub id: String,
    pub status: String,
    pub git_sha: String,
    pub key: Key,
    pub manifest_sha256: String,
    pub stream_sha256: String,
    pub layers: BTreeMap<String, String>,
    pub observations: BTreeMap<String, Vec<f64>>,
}

fn text(value: &Value, path: &[&str]) -> Result<String, String> {
    let mut at = value;
    for step in path {
        at = at.get(step).ok_or_else(|| format!("no {}", path.join(".")))?;
    }
    at.as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{} is not text", path.join(".")))
}

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads one result file and the samples file beside it.
pub fn load_run(path: &Path) -> Result<Run, String> {
    let doc = read_json(path)?;
    let env = doc.get("environment").ok_or("no environment")?;
    let native = text(env, &["kind"])? == "native";
    let key = Key {
        os: text(env, &["os"])?,
        cpu: text(env, &["cpu"])?,
        runner_class: text(env, &["runner_class"])?,
        browser_version: if native { "native".to_string() } else { text(env, &["browser_version"])? },
        protocol_version: text(env, &["protocol_version"])?,
    };
    let layers = doc["metrics"]
        .as_array()
        .map(|metrics| {
            metrics
                .iter()
                .filter_map(|m| Some((m["name"].as_str()?.to_string(), m["layer"].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let mut observations = BTreeMap::new();
    if let Some(raw) = doc.get("raw_samples").and_then(Value::as_str) {
        // A baseline keeps its samples beside its results, under the file's own name, so look there first.
        let name = Path::new(raw).file_name().ok_or("raw_samples has no file name")?;
        let beside = path.parent().map_or_else(|| PathBuf::from(name), |dir| dir.join(name));
        let file = if beside.exists() { beside } else { PathBuf::from(raw) };
        let body = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            let entry: Value = serde_json::from_str(line).map_err(|e| format!("{}: {e}", file.display()))?;
            let values = entry["values"]
                .as_array()
                .ok_or("a samples line has no values")?
                .iter()
                .filter_map(Value::as_f64)
                .collect();
            observations.insert(
                entry["metric"].as_str().ok_or("a samples line has no metric")?.to_string(),
                values,
            );
        }
    }
    Ok(Run {
        id: text(&doc, &["run_id"])?,
        status: text(&doc, &["status"])?,
        git_sha: text(env, &["git_sha"])?,
        key,
        manifest_sha256: text(&doc, &["manifest", "sha256"])?,
        stream_sha256: text(&doc, &["manifest", "operation_stream_sha256"])?,
        layers,
        observations,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The runs of `layer` and `mode` in every directory given, in the order of their names.
///
/// # Errors
///
/// Returns a message if a file cannot be read, or if a directory holds no such run.
pub fn load_collection(dirs: &[PathBuf], layer: &str, mode: &str) -> Result<Vec<Run>, String> {
    let mut runs = Vec::new();
    for dir in dirs {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|p| {
                p.extension().is_some_and(|x| x == "json")
                    && p.file_name().is_some_and(|n| {
                        let n = n.to_string_lossy();
                        !n.ends_with(".cases.json")
                            && n != "promotion.json"
                            && !n.starts_with("workload-report")
                            && !n.starts_with("allocator-verification")
                    })
            })
            .collect();
        files.sort();
        for file in files {
            let doc = read_json(&file)?;
            if doc["mode"] != mode || doc.get("run_id").is_none() {
                continue;
            }
            let in_layer = doc["metrics"]
                .as_array()
                .is_some_and(|ms| ms.iter().any(|m| m["layer"] == layer));
            // A run with no metric at all still belongs to its layer by the letter in its name.
            let by_name = doc["run_id"].as_str().is_some_and(|id| id.contains(&format!("-{layer}-")));
            if in_layer || by_name {
                runs.push(load_run(&file)?);
            }
        }
    }
    if runs.is_empty() {
        return Err(format!("no {mode} run of layer {layer} in {dirs:?}"));
    }
    Ok(runs)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn describe_side(runs: &[Run], promotion: Option<&Value>) -> Value {
    let mut side = json!({
        "git_sha": runs[0].git_sha,
        "manifest_sha256": runs[0].manifest_sha256,
        "operation_stream_sha256": runs[0].stream_sha256,
        "run_ids": runs.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
    });
    if let Some(promotion) = promotion {
        side["promotion"] = promotion.clone();
    }
    side
}

/// Why the two collections cannot be compared, or `None` if they can.
fn incompatibility(baseline: &[Run], candidate: &[Run]) -> Option<String> {
    for (name, runs) in [("baseline", baseline), ("candidate", candidate)] {
        if let Some(bad) = runs.iter().find(|r| r.status != "ok") {
            return Some(format!(
                "the {name} run {} is {}, and a timing from a run that did not do its work is not compared",
                bad.id, bad.status
            ));
        }
        if runs.iter().any(|r| r.key != runs[0].key) {
            return Some(format!("the {name} runs were not all measured in one environment"));
        }
        if runs
            .iter()
            .any(|r| r.manifest_sha256 != runs[0].manifest_sha256 || r.stream_sha256 != runs[0].stream_sha256)
        {
            return Some(format!("the {name} runs did not all run one workload"));
        }
    }
    let differences = baseline[0].key.differences(&candidate[0].key);
    if !differences.is_empty() {
        return Some(format!("measured in different environments: {}", differences.join(", ")));
    }
    if baseline[0].manifest_sha256 != candidate[0].manifest_sha256 {
        return Some("the manifests differ, so the workloads are not the same".to_string());
    }
    if baseline[0].stream_sha256 != candidate[0].stream_sha256 {
        return Some("the operation streams differ, so the workloads are not the same".to_string());
    }
    None
}

/// The observations of `metric` in each run of a collection that has any.
fn repetitions_of(runs: &[Run], metric: &str) -> Vec<Vec<f64>> {
    runs.iter().filter_map(|r| r.observations.get(metric).cloned()).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the report was asked to say about when and how many baseline runs stand behind it.
pub struct Context<'a> {
    pub generated_at: &'a str,
    /// The baseline's promotion record, as `perf/baselines/.../promotion.json` holds it.
    pub promotion: &'a Value,
    /// How many baseline runs a comparison needs before its verdicts may block a change.
    pub runs_to_block: usize,
    pub layer: &'a str,
}

/// Compares two collections and builds the report.
///
/// # Errors
///
/// Returns a message if either collection is empty.
pub fn build_report(
    baseline: &[Run],
    candidate: &[Run],
    settings: &Settings,
    context: &Context,
) -> Result<(Value, Vec<Comparison>), String> {
    if baseline.is_empty() || candidate.is_empty() {
        return Err("a collection with no runs cannot be compared".to_string());
    }
    let refusal = incompatibility(baseline, candidate);
    let metrics: BTreeSet<&String> = baseline.iter().chain(candidate).flat_map(|r| r.observations.keys()).collect();

    let mut comparisons = Vec::new();
    for metric in metrics {
        let layer = candidate
            .iter()
            .chain(baseline)
            .find_map(|r| r.layers.get(metric.as_str()))
            .map_or(context.layer, String::as_str);
        let (base, cand) = (repetitions_of(baseline, metric), repetitions_of(candidate, metric));
        for statistic in Statistic::ALL {
            let enough =
                |side: &[Vec<f64>]| !side.is_empty() && side.iter().all(|r| r.len() >= statistic.min_samples());
            // A percentile with too few observations behind it is not shown, except for the median, which says why it is not.
            if statistic != Statistic::P50 && !(enough(&base) && enough(&cand)) {
                continue;
            }
            comparisons.push(match &refusal {
                Some(reason) => not_comparable(metric, layer, statistic, reason),
                None => compare(metric, layer, statistic, &base, &cand, settings),
            });
        }
    }

    let gating = if baseline.len() >= context.runs_to_block {
        "blocking"
    } else {
        "reporting_only"
    };
    let report = json!({
        "schema_version": 1,
        "generated_at": context.generated_at,
        "baseline": describe_side(baseline, Some(context.promotion)),
        "candidate": describe_side(candidate, None),
        "compatibility": {
            "comparable": refusal.is_none(),
            "key": candidate[0].key.to_json(),
        },
        "method": {
            "statistic": "ratio of repetition medians",
            "bootstrap": "hierarchical: repetitions, then observations within each",
            "bootstrap_resamples": settings.resamples,
            "bootstrap_seed": settings.seed,
            "ratio_tolerance": settings.tolerance,
            "confidence": settings.confidence,
            "minimum_repetitions_for_verdict": settings.min_repetitions,
            "protocol_version": candidate[0].key.protocol_version,
        },
        "gating": {
            "statistical_gates": gating,
            "baseline_runs": baseline.len(),
            "baseline_runs_to_block": context.runs_to_block,
        },
        "comparisons": comparisons.iter().map(|c| to_json(c, settings.confidence)).collect::<Vec<_>>(),
    });
    let mut report = report;
    if let Some(reason) = refusal {
        report["compatibility"]["reason"] = json!(reason);
    }
    Ok((report, comparisons))
}
