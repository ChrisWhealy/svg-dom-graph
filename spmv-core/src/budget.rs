//! Budgets, checked before any scene is built. The checks, their order and their words are those of contract section 8.

use crate::spec::{Error, Spec};

/// The limits an application sets before it turns a description into scene calls (`docs/security.md`), and the model that turns a
/// size into an estimate of the page's elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub max_nodes: u64,
    pub max_edges: u64,
    pub max_degree: u64,
    pub max_cells_per_node: u64,
    pub max_total_cells: u64,
    pub max_label_len: u64,
    pub max_dom_elements: u64,
    pub dom_base: u64,
    pub dom_per_node: u64,
    pub dom_per_edge: u64,
    pub dom_per_cell: u64,
}

impl Default for Budget {
    /// The budget the gallery panel uses. Every figure has a measured basis, recorded in `spmv/docs/resource-risks.md`: Chrome 157 on
    /// one machine, so it is a budget for that class of device and not a guarantee for another.
    ///
    /// The page model is a least-squares fit of 38 measured pages, rounded up so that it never falls below the page that was
    /// measured: about 4 elements for a node, 1 for an edge and 2 for a data cell.
    fn default() -> Self {
        Self {
            max_nodes: 10_000,
            max_edges: 60_000,
            max_degree: 2_000,
            max_cells_per_node: 1_000,
            max_total_cells: 2_000,
            max_label_len: 256,
            max_dom_elements: 120_000,
            dom_base: 60,
            dom_per_node: 4,
            dom_per_edge: 1,
            dom_per_cell: 2,
        }
    }
}

impl Budget {
    /// A budget that refuses nothing, for the tests that are not about budgets.
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            max_nodes: u64::MAX,
            max_edges: u64::MAX,
            max_degree: u64::MAX,
            max_cells_per_node: u64::MAX,
            max_total_cells: u64::MAX,
            max_label_len: u64::MAX,
            max_dom_elements: u64::MAX,
            ..Self::default()
        }
    }

    /// The budget from JSON with the fields of the struct, each a number. A missing field is the default's.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] if a field is not a non-negative integer.
    pub fn from_json(v: &serde_json::Value) -> Result<Self, Error> {
        let d = Self::default();
        let field = |name: &str, default: u64| match v.get(name) {
            None => Ok(default),
            Some(x) => x
                .as_u64()
                .ok_or_else(|| Error::Invalid(format!("budget {name} is not a non-negative integer"))),
        };
        Ok(Self {
            max_nodes: field("max_nodes", d.max_nodes)?,
            max_edges: field("max_edges", d.max_edges)?,
            max_degree: field("max_degree", d.max_degree)?,
            max_cells_per_node: field("max_cells_per_node", d.max_cells_per_node)?,
            max_total_cells: field("max_total_cells", d.max_total_cells)?,
            max_label_len: field("max_label_len", d.max_label_len)?,
            max_dom_elements: field("max_dom_elements", d.max_dom_elements)?,
            dom_base: field("dom_base", d.dom_base)?,
            dom_per_node: field("dom_per_node", d.dom_per_node)?,
            dom_per_edge: field("dom_per_edge", d.dom_per_edge)?,
            dom_per_cell: field("dom_per_cell", d.dom_per_cell)?,
        })
    }

    /// The budget as JSON, with the fields of the struct.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "max_nodes": self.max_nodes, "max_edges": self.max_edges, "max_degree": self.max_degree,
            "max_cells_per_node": self.max_cells_per_node, "max_total_cells": self.max_total_cells,
            "max_label_len": self.max_label_len, "max_dom_elements": self.max_dom_elements,
            "dom_base": self.dom_base, "dom_per_node": self.dom_per_node, "dom_per_edge": self.dom_per_edge,
            "dom_per_cell": self.dom_per_cell
        })
    }

    /// The estimate of the page's elements for a size. Every sum saturates.
    #[must_use]
    pub fn estimate(&self, nodes: u64, edges: u64, cells: u64) -> u64 {
        self.dom_base
            .saturating_add(self.dom_per_node.saturating_mul(nodes))
            .saturating_add(self.dom_per_edge.saturating_mul(edges))
            .saturating_add(self.dom_per_cell.saturating_mul(cells))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A valid specification that is over a budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// One of `nodes`, `cells`, `total_cells`, `label_len`, `edges`, `degree` and `dom`.
    pub code: &'static str,
    pub message: String,
}

fn over(code: &'static str, what: &str, value: u64, limit: u64) -> Result<(), Error> {
    if value > limit {
        return Err(Error::Refused(Refusal {
            code,
            message: format!("{what} {value} is over the budget of {limit}"),
        }));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a scenario will ask of the page, which a budget is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub nodes: u64,
    pub edges: u64,
    pub cells: u64,
    pub max_degree: u64,
    pub est_dom: u64,
}

/// The first stage: the checks that need only the specification (contract section 8).
///
/// # Errors
///
/// Returns [`Error::Refused`] for the first limit that the specification would break.
pub fn check_spec(spec: &Spec, budget: &Budget) -> Result<(), Error> {
    let nodes = spec.rows + spec.cols;
    let cells = 2 * spec.cells;
    let edges_upper = spec.rows.saturating_mul(spec.per_row);
    over("nodes", "nodes", nodes, budget.max_nodes)?;
    over("cells", "cells in a data node", spec.cells, budget.max_cells_per_node)?;
    over("total_cells", "cells in all data nodes", cells, budget.max_total_cells)?;
    over("label_len", "label length", spec.label_len, budget.max_label_len)?;
    over("edges", "edges", edges_upper, budget.max_edges)?;
    over(
        "dom",
        "estimated page elements",
        budget.estimate(nodes, edges_upper, cells),
        budget.max_dom_elements,
    )
}

/// The second stage: the check that needs the generated matrix, made before any scene call. It is the degree alone, since the
/// stage before has bounded the edges and the estimate (contract decision 5).
///
/// # Errors
///
/// Returns [`Error::Refused`] for the first limit that the matrix breaks.
pub fn check_matrix(spec: &Spec, stats: &crate::matrix::Stats, budget: &Budget) -> Result<Plan, Error> {
    let nodes = spec.rows + spec.cols;
    let cells = 2 * spec.cells;
    let degree = stats.max_row_degree.max(stats.max_col_degree);
    over("degree", "degree of a node", degree, budget.max_degree)?;
    let est_dom = budget.estimate(nodes, stats.nnz, cells);
    Ok(Plan {
        nodes,
        edges: stats.nnz,
        cells,
        max_degree: degree,
        est_dom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> Spec {
        Spec {
            seed: 1,
            rows: 100,
            cols: 100,
            per_row: 4,
            hub_cols: 0,
            hub_percent: 0,
            label_len: 8,
            cells: 10,
            updates: 2,
            steps: 2,
        }
    }

    #[test]
    fn each_limit_refuses_in_its_order_and_in_its_words() {
        let tight = Budget::unlimited();
        let cases: [(Budget, Spec, &str, &str); 6] = [
            (
                Budget { max_nodes: 199, ..tight },
                spec(),
                "nodes",
                "nodes 200 is over the budget of 199",
            ),
            (
                Budget { max_cells_per_node: 9, ..tight },
                spec(),
                "cells",
                "cells in a data node 10 is over the budget of 9",
            ),
            (
                Budget { max_total_cells: 19, ..tight },
                spec(),
                "total_cells",
                "cells in all data nodes 20 is over the budget of 19",
            ),
            (
                Budget { max_label_len: 7, ..tight },
                spec(),
                "label_len",
                "label length 8 is over the budget of 7",
            ),
            (
                Budget { max_edges: 399, ..tight },
                spec(),
                "edges",
                "edges 400 is over the budget of 399",
            ),
            (
                Budget { max_dom_elements: 100, ..tight },
                spec(),
                "dom",
                "estimated page elements 1300 is over the budget of 100",
            ),
        ];
        for (budget, spec, code, message) in cases {
            match check_spec(&spec, &budget) {
                Err(Error::Refused(r)) => assert_eq!((r.code, r.message.as_str()), (code, message)),
                other => panic!("{code}: {other:?}"),
            }
        }
        assert!(check_spec(&spec(), &tight).is_ok());
    }

    #[test]
    fn nodes_are_checked_before_cells_when_both_are_over() {
        let budget = Budget {
            max_nodes: 1,
            max_cells_per_node: 1,
            ..Budget::unlimited()
        };
        match check_spec(&spec(), &budget) {
            Err(Error::Refused(r)) => assert_eq!(r.code, "nodes"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_estimate_saturates_and_never_wraps() {
        let b = Budget {
            dom_per_node: u64::MAX,
            ..Budget::default()
        };
        assert_eq!(b.estimate(u64::MAX, u64::MAX, u64::MAX), u64::MAX);
    }
}
