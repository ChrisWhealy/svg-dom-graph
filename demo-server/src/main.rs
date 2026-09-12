//! Static file server for the `svg-dom-graph` demo.
//!
//! Run from the project root with:
//! ```sh
//! cargo demo
//! ```
//!
//! The following steps are performed:
//!
//! 1. Stages `index.html` into the target directory (see [`build`])
//! 1. Rebuilds the `svg-dom-graph-demo` crate's wasm package using `wasm-pack build demo-app --target web`
//! 1. Serves the result at <http://127.0.0.1:8000/>
//!
//! The staged `index.html` and the wasm `pkg/` are kept under `target/demo-stage/` rather than written into the
//! source tree (as the old `./demo` shell script did, alongside a `python3 -m http.server` to serve it) or served
//! straight out of the project root. Concretely:
//!
//! ```text
//! target/demo-stage/
//! ├── index.html   (copied from the project root's own index.html)
//! └── pkg/         (wasm-pack's output)
//! ```
//!
//! Serving from outside the source tree means:
//!
//! * The checkout stays untouched by `cargo demo` (nothing to accidentally commit and `git status` stays clean)
//! * A failed or interrupted run will not leave a partial artefact sitting in a directory monitored by source control
//! * The whole staged output is removed by an ordinary `cargo clean`, the same as any other build output
//!
//! The port number can be overridden using the `PORT` environment variable, e.g. `PORT=9000 cargo demo`.
//!
//! [`build::prepare_stage`] also reruns before every request the server handles, not just once at startup. So
//! editing `index.html` is visible on the next browser refresh alone. Nothing restages the wasm package per
//! request: `wasm-pack` is too slow for that, and editing Rust source needs a restart regardless, for the wasm
//! rebuild to even happen.
//!
//! The build pipeline itself — resolving staging paths through to a wasm package ready to serve — lives in
//! [`build`], as a `Result`-returning [`build::build_demo`] rather than something that reports errors and exits on
//! its own. That keeps every "how do we stage the demo" decision testable and reusable independently of Actix, and
//! leaves `main` as the one place that decides what a build failure means for the process.
//!
//! Run with `--prepare-only` (`cargo run -p demo-server -- --prepare-only`) to run [`build::prepare_stage`] alone
//! — stage `index.html` — and exit, without rebuilding the wasm package or starting the server. This is what lets
//! CI exercise the staging copy without paying for a full wasm build every run.
//!
//! Run with `--build-only` (`cargo run -p demo-server -- --build-only`) to run the full [`build::build_demo`]
//! pipeline — stage `index.html` and rebuild the wasm package — and exit, without starting the server. CI's `wasm`
//! job uses this instead of invoking `wasm-pack build demo-app ...` directly, so it exercises the exact command
//! `build::build_wasm` actually constructs (working directory, argument order, the absolute `--out-dir` computed
//! from `StagePaths`) rather than a hand-written approximation of it that could quietly drift out of step with
//! what `cargo demo` really runs.
mod build;

use actix_files::Files;
use actix_web::{App, HttpServer, middleware::Logger};
use build::StagePaths;
use std::{env::VarError, path::PathBuf, process};

const DEFAULT_PORT: u16 = 8000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // The demo-server crate lives one level below the project root.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| std::io::Error::other("demo-server must live inside the project"))?
        .to_path_buf();

    // Respects a `CARGO_TARGET_DIR` override the same way `cargo build` itself would, rather than assuming the
    // target directory always sits directly under the project root.
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let stage = StagePaths::new(&target_dir);

    // Unlike CARGO_TARGET_DIR above (where any absence just means "use the default layout"), a *present but
    // unparseable* PORT is a configuration mistake worth reporting rather than silently falling back to
    // DEFAULT_PORT — `PORT=abc cargo demo` should fail loudly, not quietly start on 8000.
    let port: u16 = match std::env::var("PORT") {
        Ok(value) => value
            .parse()
            .map_err(|_| std::io::Error::other(format!("invalid PORT value: {value:?}")))?,
        Err(VarError::NotPresent) => DEFAULT_PORT,
        Err(err) => return Err(std::io::Error::other(err)),
    };

    if std::env::args().any(|arg| arg == "--prepare-only") {
        if let Err(err) = build::prepare_stage(&root, &stage) {
            eprintln!("aborting: {err}");
            process::exit(1);
        }
        println!("demo staged successfully at {}", stage.stage_dir.display());
        return Ok(());
    }

    if std::env::args().any(|arg| arg == "--build-only") {
        if let Err(err) = build::build_demo(&root, &stage) {
            eprintln!("aborting: {err}");
            process::exit(1);
        }
        println!("demo built successfully at {}", stage.stage_dir.display());
        return Ok(());
    }

    // Every build phase — staging index.html, rebuilding the wasm package — runs here, in order, before the server
    // ever starts; a failure at either phase is fatal, so `main` reports it and exits rather than starting Actix in
    // front of an incomplete or stale demo.
    if let Err(err) = build::build_demo(&root, &stage) {
        eprintln!("aborting: {err}");
        process::exit(1);
    }

    let addr = ("127.0.0.1", port);
    let stage_dir = stage.stage_dir.clone();

    println!("\n  svg-dom-graph demo running on http://127.0.0.1:{port}/\n");

    HttpServer::new(move || {
        let root = root.clone();
        let stage = stage.clone();

        App::new()
            .wrap(Logger::default())
            // Re-stages index.html before every request reaches Files below, so an edit to it is visible on the
            // very next browser refresh. build_demo's wasm rebuild is deliberately not repeated here: wasm-pack is
            // far too slow to run per request and editing Rust source already requires restarting cargo demo
            // regardless.
            //
            // A refresh failure (e.g. index.html was left mid-edit) is only logged, not fatal: the previously
            // staged file is left in place and keeps being served, the same file-not-found-yet tolerance an
            // editor's own autosave already needs. This is a real guarantee, not just a likely outcome —
            // prepare_stage's own doc comment explains why it writes through a temporary file and renames it into
            // place, rather than copying straight onto the live index.html.
            .wrap_fn(move |req, srv| {
                if let Err(err) = build::prepare_stage(&root, &stage) {
                    eprintln!("warning: could not refresh the staged demo ({err})");
                }
                actix_web::dev::Service::call(srv, req)
            })
            .service(Files::new("/", stage_dir.clone()).index_file("index.html"))
    })
    .bind(addr)?
    .run()
    .await
}
