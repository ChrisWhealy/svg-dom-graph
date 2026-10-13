//! Shared setup helpers for `cdp-integration-test`'s Chrome-DevTools-Protocol integration tests.
//!
//! The tests themselves live in one Cargo integration-test binary, `tests/cdp/`, with one scenario per sibling module.
//! See
//! [`tests/cdp/main.rs`](https://github.com/ChrisWhealy/svg-dom-graph/blob/main/cdp-integration-test/tests/cdp/main.rs)'s
//! own module doc comment for the scenario list. It also explains why this crate lives in its own on-demand workspace
//! member. This mirrors `svg-dom`'s own `cdp-integration-test` crate, down to this file's structure.
//!
//! The functions below build the `cdp-test-fixture` wasm package, serve it, and launch Chrome. `tests/cdp/common.rs`
//! calls them exactly once per test run, via a lazily-initialised `OnceLock`, and hands every scenario module its own
//! [`Tab`](headless_chrome::Tab) from the one shared [`Browser`] instance.

use fd_lock::RwLock;
use headless_chrome::{Browser, LaunchOptions, browser::default_executable};
use std::{fs::OpenOptions, path::PathBuf, process::Command, thread};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path to the sibling `cdp-test-fixture` wasm crate, relative to this crate's own manifest directory.
pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cdp-integration-test must live inside the svg-dom-graph workspace")
        .join("cdp-test-fixture")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Rebuilds the `cdp-test-fixture` wasm package so `serve`'s output is current.
///
/// `tests/cdp/common.rs` calls this once per test run, from behind its own `OnceLock`, so within this crate's own `cdp`
/// binary only one call is ever in flight. The exclusive cross-process file lock taken below still guards against a
/// second, independent `cargo test`/`cargo nextest run` invocation racing this one. Two `wasm-pack build` invocations
/// running in the same `dir` at once would corrupt each other's intermediate states.
pub fn build_fixture(dir: &PathBuf) {
    let lock_path = dir.join(".build_fixture.lock");
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .expect("could not open build_fixture lock file");
    let mut lock = RwLock::new(lock_file);
    let _guard = lock.write().expect("could not acquire build_fixture lock");

    let status = Command::new("wasm-pack")
        .current_dir(dir)
        .args(["build", "--target", "web"])
        .status()
        .expect("could not run wasm-pack — is it installed and on PATH?");
    assert!(status.success(), "wasm-pack build failed for cdp-test-fixture");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo gallery with `demo-server --build-only` and returns the directory it staged, ready to serve.
///
/// The staged directory holds `index.html`, `style.css`, the demo's window pages and the wasm `pkg/`. The same
/// cross-process file lock `build_fixture` takes guards against a second `cargo test` racing this build.
pub fn build_demo() -> PathBuf {
    let root = fixture_dir()
        .parent()
        .expect("the fixture lives inside the workspace")
        .to_path_buf();
    // The same directory `demo-server` stages into, which `CARGO_TARGET_DIR` can move. It may not exist yet, and the
    // lock has to be taken before the build that would create it. A relative path is relative to where cargo runs.
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), |dir| root.join(dir));
    std::fs::create_dir_all(&target).expect("could not create the target directory");
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(target.join(".build_demo.lock"))
        .expect("could not open build_demo lock file");
    let mut lock = RwLock::new(lock_file);
    let _guard = lock.write().expect("could not acquire build_demo lock");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .current_dir(&root)
        .args(["run", "-p", "demo-server", "--", "--build-only"])
        .status()
        .expect("could not run cargo to build the demo");
    assert!(status.success(), "demo-server --build-only failed");

    target.join("demo-stage")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Serves `dir` on an OS-assigned local port and returns that port. The server runs for the lifetime of the test
/// process on a background thread; there is no shutdown hook, but the process exits when the test does.
pub fn serve(dir: PathBuf) -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind ephemeral port");
    let port = listener.local_addr().expect("no local addr").port();
    let server = tiny_http::Server::from_listener(listener, None).expect("failed to start static file server");

    thread::spawn(move || {
        for request in server.incoming_requests() {
            let mut path = request.url().trim_start_matches('/').to_owned();
            if path.is_empty() {
                path = "index.html".to_owned();
            }
            let file_path = dir.join(&path);
            let response_result = match std::fs::read(&file_path) {
                Ok(bytes) => {
                    let content_type = if path.ends_with(".wasm") {
                        "application/wasm"
                    } else if path.ends_with(".js") {
                        "text/javascript"
                    } else if path.ends_with(".css") {
                        "text/css"
                    } else {
                        "text/html"
                    };
                    let header = tiny_http::Header::from_bytes(b"Content-Type".as_slice(), content_type.as_bytes())
                        .expect("valid header");
                    request.respond(tiny_http::Response::from_data(bytes).with_header(header))
                },
                Err(_) => request.respond(tiny_http::Response::from_string("not found").with_status_code(404)),
            };
            let _ = response_result;
        }
    });

    port
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How long the browser may go without opening, closing or changing a tab before `headless_chrome` drops its connection.
///
/// The library's own default is 30 seconds, and it counts only browser-level events, not the calls a test makes inside
/// a tab. A test that opens one tab and then works in it for longer, as a walk through all 961 positions of the ChaCha20
/// demo does, would leave the browser idle by that count. Every test sharing the browser after it would then fail to
/// open its own tab, with "underlying connection is closed". Ten minutes is far longer than any one test.
const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Launches Chrome with its sandbox disabled and a generously large window.
///
/// `ubuntu-latest` CI runners restrict unprivileged user namespaces via `AppArmor`, which breaks Chrome's own sandbox
/// initialisation unless `--no-sandbox` is passed. Every scenario here only ever loads a local fixture page built by
/// this crate, so there is no untrusted content to be sandboxed. This is disabled unconditionally, not just in CI, so
/// local and CI runs stay on the same code path.
///
/// `window_size` is set explicitly, rather than left at Chrome's own headless default. That default is small enough
/// that a deliberately large drag delta could land short of the actual window edge, or at an unrelated point past it.
/// `bounds.rs` drags a node hundreds of pixels past the fixture's own view box. Every other scenario here only ever
/// uses deltas relative to an element's own midpoint, so a larger window changes nothing for them.
pub fn launch_browser() -> Result<Browser, Box<dyn std::error::Error>> {
    let path = default_executable().map_err(|e| format!("could not locate a Chrome/Chromium binary: {e}"))?;
    let launch_options = LaunchOptions::default_builder()
        .path(Some(path))
        .sandbox(false)
        .window_size(Some((1200, 900)))
        .idle_browser_timeout(IDLE_TIMEOUT)
        .build()?;
    Ok(Browser::new(launch_options)?)
}
