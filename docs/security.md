# Security

## Table of Contents

- [Summary](#summary)
- [What Is Not a Problem](#what-is-not-a-problem)
- [Resource Limits for an Attacker-Controlled Graph Description](#resource-limits-for-an-attacker-controlled-graph-description)
  - [Graph and Data Size](#graph-and-data-size)
  - [Node Degree](#node-degree)
  - [String Lengths](#string-lengths)
  - [Suggested Budgets](#suggested-budgets)
- [Underlying `svg-dom` Trust Boundaries](#underlying-svg-dom-trust-boundaries)

## Summary

`svg-dom-graph` builds on the functionality found in `svg-dom` and introduces no script-injection or XSS risk of its own.

Its graph-specific concerns are those related to the availability, consumption and exhaustion of browser resources.
An application that turns untrusted input into `Scene` calls can be made to consume excessive browser CPU and memory.

These are not memory-safety issues, and trusted graphs of ordinary size are not affected.
This document offer guidance related to application hardening, not a vulnerability.

## What Is Not a Problem

Node names and labels are safe to take from untrusted sources as far as injection is concerned.
A label such as `<script>alert("You've been hacked!")</script>` is passed to `svg-dom`'s text setters, which use `textContent`, so the content is never `eval`ed; it is simply rendered as plain text.

`svg-dom-graph` does not expose caller-supplied attribute names, URLs, CSS, or paint strings.
Every attribute name and style it writes is fixed in the crate.

The toolbar, panning, and wheel zoom add no caller-supplied input.
The zoom scale is limited to 0.25 to 4.0, and panning only changes a single `transform` attribute, so neither changes the size of the DOM.
While a toolbar is shown, a Ctrl or Cmd plus wheel event over the scene is cancelled so the browser does not zoom the whole page.

Node coordinates and dimensions must be finite, dimensions must be positive, and elbow corner radii must be finite and non-negative.
Non-finite geometry is therefore rejected up front and needs no separate precaution.

## Resource Limits for an Attacker-Controlled Graph Description

The crate deliberately imposes no application-level limits.
An application that builds a `Scene` from externally supplied data must impose its own budget constraints.

### Graph and Data Size

A data node's grid can hold an arbitrarily large number of cells, and each multi-value cell creates DOM elements.
A crafted description can therefore produce a very large DOM tree, with a correspondingly high CPU and memory cost.

The same applies to the total number of nodes and edges in the scene.

### Node Degree

A node's degree matters separately from the total edge count, for two reasons.

- **Accessibility metadata growth.**

   Every edge appends a relationship clause to both endpoint nodes.

   `Scene::append_relationship` then rewrites the whole accumulated string into both `aria-label` and `<title>`.
   A node with thousands of edges therefore ends up with a huge description, and each further edge writes a longer string than the last.
   Long node names will only make this worse.

- **Per-frame drag cost.**

   The high resolution pointer movement events created at the hardware level are coalesced to at most one `move_node` event per animation frame.

   ⚠️ Caveat ⚠️

   `move_node` must still redraw every edge incident to the moving node.
   The coalescing process limits how often a new frame is rendered, not how much work must be performed to render that frame.

### String Lengths

Plain-node labels are unrestricted strings.
Each one becomes DOM text, is retained as the node's `ref_name`, and is measured by the browser with `getBBox()`.
The same names are then copied into the accessibility relationship strings described above.

Limit string lengths as well as node counts.

### Suggested Budgets

An application accepting untrusted graph descriptions should consider imposing the following budget constraints:

- total nodes and edges
- the maximum degree of any individual node
- total values or cells across all data nodes
- the maximum number of cells in a single data node
- node-name and label lengths
- the number of draggable, interactively updated nodes
- optionally, an overall rendered-DOM-element budget

Reject or truncate a description that exceeds a budget limit before any `Scene` call is made.

## Underlying `svg-dom` Trust Boundaries

`svg-dom` documents its own trust boundaries, including caller-supplied URLs, CSS, paint strings, arbitrary attributes, and its resource-limit guidance.
Those are upstream rules, not vulnerabilities introduced by this crate.

See `svg-dom`'s [security](https://github.com/ChrisWhealy/svg-dom/blob/main/docs/security.md) documentation for details.

`svg-dom-graph` does not expose any of those inputs itself.
An application that also uses the same `SvgRoot` through `svg-dom` directly remains subject to them.
