# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project's crate version follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]

# [Released]

## [0.1.0] - 2026-08-25

## Fixed

- Correct README (`4f7c534`)
- Remove `SvgNode` ownership cycle (`af00332`)
- Implement pointer -> user-space coordinate conversion (`b00e9f9`)
- Remove `Scene` ownership cycle (`d5aaf42`)
- Restrict visibility of geometry helpers (`f4cae07`)
- Remove internal functionality from public API (``)

## Changed

- Define authoritative graph model (`95f32aa`)
- Detach demo from `svg-dom-graph` library (`720d4b1`)
- Separate PoC structures from public API (`93e3aac`)
- Depend on published `svg-dom` crate (`c3d4e5b`)
- Switch publish flag (`574838f`)
- Change node labels away from `&'static str` (`15eecf6`)
- Ensure `NodeId` and `EdgeId` are only crate public (`cbcf27e`)

## Added

- Initial commit (`04d5cd7`)
- Add browser-level test layer and CI testing (`9eee6e7`)
- `Scene` must genuinely own or bind to its SVG root (`dd421e8`)
- `NodeId` identifies the graph to which it belongs (`2fe939e`)
- Track the active `pointer_id` (`5733572`)
- Add an MSRV 1.85 job to CI (`811cd64`)
