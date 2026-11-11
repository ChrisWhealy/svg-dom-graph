//! How long one test held the browser.

use std::{io::Write, time::Instant};

/// The environment variable that names a file to which each test appends how long it held the browser, as a line of
/// `test name`, a tab, and milliseconds. `.github/scripts/cdp-deal.sh` uses these times to deal the tests out to the CI
/// shards so that each shard takes about as long as the others. Nothing is recorded if it is not set.
const TIMING_FILE_VAR: &str = "CDP_TIMING_FILE";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How long one test held the browser. It starts when the test's turn has begun and the browser is up, so the time it
/// spent waiting for other tests, and the one-off start of Chrome, are not counted. It ends when the test's [`DemoTab`]
/// is dropped, which is when the test ends, whether it passed or failed.
pub(crate) struct Timing {
    started: Instant,
}

impl Timing {
    pub(crate) fn start() -> Self {
        Self { started: Instant::now() }
    }
}

impl Drop for Timing {
    fn drop(&mut self) {
        let Some(path) = std::env::var_os(TIMING_FILE_VAR) else { return };
        // libtest names each test's thread after the test, as `hamming::restart_from_position_0_returns_to_the_start`.
        let thread = std::thread::current();
        // With a single test thread, libtest runs the tests on the main thread, which does not say which test it is.
        let Some(name) = thread.name().filter(|name| *name != "main") else { return };
        let line = format!("{name}\t{}\n", self.started.elapsed().as_millis());
        // One `write` of the whole line, so lines from separate runs appending to one file do not interleave.
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}
