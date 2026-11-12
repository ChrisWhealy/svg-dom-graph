# Versioning and compatibility

This page says what a release promises, so that an application can decide how to depend on the crate.
Where it describes what the project already does, that is said.
Where it is a rule that the project has not yet written down, it is marked as proposed, and the owner decides it.

## What is the public API

Everything that is `pub` and reachable from the crate root, and what it is documented to do, is the API.
The root and `svg_dom_graph::scene` are the usual entry points.
Anything in a private module is not, whatever it is called, and a doc comment that says an item is internal means that.
Behaviour that a doc comment promises, such as the outcome of a failed call (`nothing committed` or `partly committed`), is part of the API as much as a signature is.
The contracts in `perf/fault-contracts.json` are checked against the library at every DOM call, so a promise there is held by a test.

## Version numbers

The crate follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html), as the changelog says.
While the major version is 0, rely on the rule that Cargo applies.
A change of the second number may break, and a change of the third may not.

| Change                                                                          | Number          | Example                                                        |
| ------------------------------------------------------------------------------- | --------------- | -------------------------------------------------------------- |
| A removed or renamed public item, a changed signature, a promise that is weaker | Second, while 0 | Removing a method                                              |
| A new public item, a promise that is stronger                                   | Third           | A new `Scene` method, a failure that now leaves nothing behind |
| A fix that makes the code do what its documentation says                        | Third           | `set_edge_label` no longer leaving a stray box                 |
| A new `#[non_exhaustive]` variant                                               | Third           | A new `Error` variant                                          |

Enums that may grow are `#[non_exhaustive]`, and so are the option and value types that may gain a field.
An application that matches on one needs a wildcard arm, and the compiler says so.

## The minimum Rust version

The minimum is the `rust-version` in `Cargo.toml`, which is 1.85, and CI builds the crate on it.
A change of it is a change of the second number while the major version is 0, and is in the changelog.
It does not move for a feature that is only convenient.
It is not moved past a version that is a year old without a reason that is written down (proposed).

## Dependencies

`svg-dom` is a public dependency, since its `SvgRoot` and geometry types appear in the signatures.
It is required as `^0.2.17`, so a patch release of it does not need a release of this crate.
A release of `svg-dom` that changes a type used in the API needs a release of this crate that says so.
The dependency list is checked for advisories, licences and sources by `cargo deny`, on every release check and in CI.

## How a release is checked

`scripts/release-check.sh` runs what has to hold of the package that would be published.
See [release qualification](release-qualification.md).
`cargo semver-checks` compares the public API with the last published release and says whether the number that was chosen is enough.
It is run before a release, and its result is read by a person, since it knows signatures and not promises.

## What an application can rely on

- A call that returns an error has changed what its documentation says it changes, and nothing else.
- The accessibility attributes that a node carries are written by the library, and an application does not write them.
- A scene that is dropped releases its listeners and its frame request, and removing its host `<svg>` leaves the page as it was.
- The library applies no limit of its own to what it is given.
  See [security](security.md) for the budgets that an application sets, and for a tested example of enforcing them.
