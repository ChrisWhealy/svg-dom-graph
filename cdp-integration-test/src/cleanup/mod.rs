//! Keeps the Chrome process from which a test run launches from outliving the run, and keeps its profile folder from
//! piling up.
//!
//! The harness holds its browsers in statics, and a static is never dropped when a process exits. So
//! `headless_chrome`'s own drop, which kills Chrome and removes its profile, never ran. Under `cargo nextest` every
//! test is a process of its own, so each one left a Chrome, with its helpers, and a profile folder of its own behind.
//! Hundreds built up in one session.
//!
//! This module closes that in three ways:
//!
//! - Each browser gets a profile folder that this crate names, `svg-dom-graph-cdp-<pid>-<n>` in the temporary
//!   directory, so the owner of a folder can be told from its name.
//! - Every browser that is launched is registered. A hook that runs when the process exits kills it and removes its
//!   folder. A handler for `SIGTERM`, `SIGINT` and `SIGHUP` kills it too, which is how a runner stops a test that has
//!   timed out.
//! - Every launch first sweeps up after runs that did not get that far, because they were killed with `SIGKILL` or
//!   crashed. A folder whose owner process is gone has the Chrome that still uses it killed, and is removed.

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Mutex, Once,
        atomic::{AtomicI32, AtomicU32, Ordering},
    },
};

/// What the name of every profile folder that this crate makes starts with.
pub(crate) const PROFILE_PREFIX: &str = "svg-dom-graph-cdp-";

/// The most browsers that one process registers. A run uses two, one for the fixture and one for the demo.
const SLOTS: usize = 8;

/// The browsers that this process has launched and not yet shut down: the process ID of Chrome and its profile folder.
static LAUNCHED: Mutex<Vec<(u32, PathBuf)>> = Mutex::new(Vec::new());

/// The same process IDs, for the signal handler, which may call nothing that takes a lock. A slot is 0 when it is free.
static CHROME_PIDS: [AtomicI32; SLOTS] = [const { AtomicI32::new(0) }; SLOTS];

/// Counted up for each folder that is named, so that two browsers of one process do not share one.
static NEXT: AtomicU32 = AtomicU32::new(0);

static INSTALLED: Once = Once::new();

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The folder for the next browser of the process `owner`, in `temp`.
pub(crate) fn profile_dir_in(temp: &Path, owner: u32) -> PathBuf {
    temp.join(format!("{PROFILE_PREFIX}{owner}-{}", NEXT.fetch_add(1, Ordering::Relaxed)))
}

/// The process that made the folder called `name`, or `None` if this crate did not make it.
pub(crate) fn owner_of(name: &str) -> Option<u32> {
    name.strip_prefix(PROFILE_PREFIX)?.split('-').next()?.parse().ok()
}

/// Whether the process `pid` is running.
#[cfg(unix)]
pub(crate) fn is_alive(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    // SAFETY: signal 0 sends nothing. It only asks whether the process exists, and whether this one may signal it.
    unsafe { libc::kill(pid, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
}

/// Whether the process `pid` is running, which cannot be told here, so it is taken to be.
#[cfg(not(unix))]
pub(crate) fn is_alive(_pid: u32) -> bool {
    true
}

/// Kills every process that was started with `dir` as its profile: Chrome, and the helpers that it started.
fn kill_users_of(dir: &Path) {
    let _ = Command::new("pkill")
        .args(["-KILL", "-f", "--"])
        .arg(dir.to_string_lossy().as_ref())
        .status();
}

/// Removes the folders in `temp` that this crate made for a process that is gone, after killing the Chrome that still
/// uses them. It gives the folders that it removed. A folder of a process that is running, and anything else, is left
/// alone.
pub(crate) fn sweep_stale(temp: &Path, alive: impl Fn(u32) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(temp) else {
        return Vec::new();
    };
    let mut removed = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(owner) = name.to_str().and_then(owner_of) else {
            continue;
        };
        if alive(owner) {
            continue;
        }
        let path = entry.path();
        kill_users_of(&path);
        if std::fs::remove_dir_all(&path).is_ok() {
            removed.push(path);
        }
    }
    kill_orphaned_crash_reporters();
    removed
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Remembers a browser that has been launched, so that it is shut down when the process exits or is stopped.
pub(crate) fn register(chrome: u32, profile: PathBuf) {
    install();
    if let Some(slot) = CHROME_PIDS.iter().find(|slot| slot.load(Ordering::SeqCst) == 0) {
        slot.store(i32::try_from(chrome).unwrap_or(0), Ordering::SeqCst);
    }
    LAUNCHED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push((chrome, profile));
}

/// Kills every browser that is registered, waiting a moment for each to go, and removes its profile folder.
pub(crate) fn shut_down() {
    let launched = std::mem::take(&mut *LAUNCHED.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
    for (chrome, profile) in launched {
        stop(chrome);
        kill_users_of(&profile);
        let _ = std::fs::remove_dir_all(&profile);
    }
    kill_orphaned_crash_reporters();
    for slot in &CHROME_PIDS {
        slot.store(0, Ordering::SeqCst);
    }
}

/// The crash reporters of Chrome that are left with nothing to report on, from the output of `ps -axo
/// pid=,ppid=,command=`.
///
/// Chrome starts its crash reporter detached, as a child of process 1, and it keeps Chrome's own shared crash database
/// and not the profile folder. So nothing finds it by the folder, and it stays for half a minute after Chrome has gone.
/// One is taken to be a leftover only when no Chrome is running that this crate did not launch, since the crash
/// reporter of somebody's own Chrome must not be touched.
pub(crate) fn orphaned_crash_reporters(ps: &str) -> Vec<u32> {
    let mut reporters = Vec::new();
    for line in ps.lines() {
        let mut parts = line.split_whitespace();
        let (Some(pid), Some(ppid)) = (parts.next(), parts.next()) else {
            continue;
        };
        let command = line
            .trim_start()
            .splitn(3, char::is_whitespace)
            .nth(2)
            .unwrap_or("")
            .trim_start();
        let is_chrome = command.contains("Google Chrome.app")
            || command.contains("/google/chrome/")
            || command.contains("chromium");
        if !is_chrome {
            continue;
        }
        if command.contains("chrome_crashpad_handler") {
            if ppid == "1" {
                reporters.extend(pid.parse::<u32>().ok());
            }
        } else if !command.contains("Helper") && !command.contains(PROFILE_PREFIX) && !command.contains("--type=") {
            // A Chrome that is not one of this crate's, and not one of its helpers.
            return Vec::new();
        }
    }
    reporters
}

/// Kills the crash reporters that [`orphaned_crash_reporters`] finds.
fn kill_orphaned_crash_reporters() {
    let Ok(out) = Command::new("ps").args(["-axo", "pid=,ppid=,command="]).output() else {
        return;
    };
    for pid in orphaned_crash_reporters(&String::from_utf8_lossy(&out.stdout)) {
        let _ = Command::new("kill").args(["-KILL", &pid.to_string()]).status();
    }
}

/// Asks Chrome to stop, and kills it if it has not in a second.
#[cfg(unix)]
fn stop(chrome: u32) {
    let Ok(pid) = i32::try_from(chrome) else {
        return;
    };
    // SAFETY: sends a signal to a process that this crate launched, and that is waited for below.
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    for _ in 0..20 {
        if !is_alive(chrome) || is_zombie(chrome) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // SAFETY: as above.
    unsafe {
        libc::kill(pid, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn stop(_chrome: u32) {}

/// Whether the process has stopped and is only waiting for its parent to collect it, which `kill` still counts as
/// running.
#[cfg(unix)]
fn is_zombie(pid: u32) -> bool {
    Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).trim_start().starts_with('Z'))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(unix)]
extern "C" fn at_exit() {
    shut_down();
}

/// Kills the browsers, which is all that a signal handler may safely do, and then lets the signal do what it would
/// have. The profile folders are left, and the next launch sweeps them.
#[cfg(unix)]
extern "C" fn on_signal(signal: libc::c_int) {
    for slot in &CHROME_PIDS {
        let pid = slot.load(Ordering::SeqCst);
        if pid > 0 {
            // SAFETY: `kill` is async-signal-safe, and the process is one that this crate launched.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
    // SAFETY: restores the default action for this signal and raises it again, both async-signal-safe.
    unsafe {
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}

/// Installs the exit hook and the signal handlers, once.
fn install() {
    INSTALLED.call_once(|| {
        #[cfg(unix)]
        // SAFETY: registers a function that takes no arguments, and handlers that only call async-signal-safe
        // functions.
        unsafe {
            libc::atexit(at_exit);
            for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
                libc::signal(signal, on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t);
            }
        }
    });
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
