//! A whole scenario as one JSON value, which the oracle's value is compared with.

use crate::budget::{Budget, Plan, check_matrix, check_spec};
use crate::hash::sha256_hex;
use crate::matrix::{Matrix, label};
use crate::run::Run;
use crate::spec::{Error, Spec};
use serde_json::{Value, json};

/// What a scenario was and did: the plan, the statistics, and the hashes of the matrix and of the vectors after every
/// step.
///
/// # Errors
///
/// Returns [`Error::Invalid`] for a malformed specification, [`Error::Refused`] for one over the budget, or
/// [`Error::Internal`] if the product disagrees with itself.
pub fn trace(spec: &Spec, budget: &Budget) -> Result<Value, Error> {
    let Prepared { matrix, plan, .. } = Prepared::new(spec, budget)?;
    let stats = matrix.stats();
    let matrix_sha256 = matrix.hash();
    let labels_sha256 = labels_hash(spec);
    let mut run = Run::new(spec, matrix);
    let (x0, y0) = (run.x_hash(), run.y_hash());
    let mut steps = Vec::new();
    for _ in 0..spec.steps {
        run.advance(true)?;
        steps.push(json!({ "step": run.step, "x_sha256": run.x_hash(), "y_sha256": run.y_hash() }));
    }
    Ok(json!({
        "outcome": "ok",
        "plan": plan_json(&plan),
        "stats": {
            "nnz": stats.nnz, "max_row_degree": stats.max_row_degree, "max_col_degree": stats.max_col_degree,
            "empty_rows": stats.empty_rows, "empty_cols": stats.empty_cols
        },
        "matrix_sha256": matrix_sha256,
        "labels_sha256": labels_sha256,
        "x0_sha256": x0,
        "y0_sha256": y0,
        "steps": steps,
    }))
}

/// The hash of every node label (contract section 6): the columns' and then the rows', each followed by a line feed.
fn labels_hash(spec: &Spec) -> String {
    let len = spec.label_len as usize;
    let mut text = String::new();
    for j in 0..spec.cols as usize {
        text.push_str(&label('x', j, len));
        text.push('\n');
    }
    for i in 0..spec.rows as usize {
        text.push_str(&label('y', i, len));
        text.push('\n');
    }
    sha256_hex(text.as_bytes())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scenario that has passed its checks: a valid specification, within both stages of a budget, and the matrix it
/// generated.
///
/// This is the only way to a matrix that a scene is built from. A scene builder that takes a `Prepared` cannot be given
/// a scenario that was not checked, so a budget is enforced before any scene call by the types and not by the caller's
/// care.
#[derive(Debug, Clone)]
pub struct Prepared {
    spec: Spec,
    matrix: Matrix,
    plan: Plan,
}

impl Prepared {
    /// Validates the specification, checks it against the budget, generates the matrix and checks that against the
    /// budget.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] for a malformed specification and [`Error::Refused`] for one over the budget. No
    /// matrix is generated for a specification that the first stage refuses.
    pub fn new(spec: &Spec, budget: &Budget) -> Result<Self, Error> {
        spec.validate()?;
        check_spec(spec, budget)?;
        let matrix = Matrix::generate(spec);
        let plan = check_matrix(spec, &matrix.stats(), budget)?;
        Ok(Self { spec: *spec, matrix, plan })
    }

    #[must_use]
    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    #[must_use]
    pub fn matrix(&self) -> &Matrix {
        &self.matrix
    }

    #[must_use]
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// The initial state of the scenario's run.
    #[must_use]
    pub fn run(&self) -> Run {
        Run::new(&self.spec, self.matrix.clone())
    }
}

fn plan_json(p: &Plan) -> Value {
    json!({ "nodes": p.nodes, "edges": p.edges, "cells": p.cells, "max_degree": p.max_degree, "est_dom": p.est_dom })
}

/// The outcome of a scenario that did not run, in the form the oracle reports it.
#[must_use]
pub fn failure_json(e: &Error) -> Value {
    match e {
        Error::Invalid(m) => json!({ "outcome": "invalid", "code": "invalid", "message": m }),
        Error::Refused(r) => json!({ "outcome": "refused", "code": r.code, "message": r.message }),
        Error::Internal(m) => json!({ "outcome": "internal", "code": "internal", "message": m }),
    }
}

/// [`trace`], with a failure turned into a value instead of an error.
#[must_use]
pub fn outcome(spec: &Spec, budget: &Budget) -> Value {
    trace(spec, budget).unwrap_or_else(|e| failure_json(&e))
}
