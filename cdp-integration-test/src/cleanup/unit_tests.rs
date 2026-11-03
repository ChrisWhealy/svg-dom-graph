//! That a folder says who made it, that the sweep removes only what a dead process left, and that shutting down kills a
//! real process and removes its folder.

use super::*;

/// A directory of its own under the system's temporary one, empty, for a test to put folders in.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cdp-cleanup-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("could not create the scratch directory");
    dir
}

#[test]
fn a_folder_is_named_for_the_process_that_made_it_and_says_so() {
    let temp = Path::new("/tmp");
    let first = profile_dir_in(temp, 4321);
    let second = profile_dir_in(temp, 4321);
    assert_ne!(first, second, "two browsers of one process do not share a folder");
    for dir in [&first, &second] {
        let name = dir.file_name().and_then(|n| n.to_str()).expect("a name");
        assert!(name.starts_with(PROFILE_PREFIX), "{name}");
        assert_eq!(owner_of(name), Some(4321));
    }
}

#[test]
fn only_a_folder_that_this_crate_made_has_an_owner() {
    assert_eq!(owner_of("svg-dom-graph-cdp-77-0"), Some(77));
    assert_eq!(owner_of("svg-dom-graph-cdp-77"), Some(77));
    assert_eq!(owner_of("svg-dom-graph-cdp-"), None);
    assert_eq!(owner_of("svg-dom-graph-cdp-x-1"), None);
    assert_eq!(owner_of("rust-headless-chrome-profileAbC"), None);
    assert_eq!(owner_of("my-folder"), None);
}

#[cfg(unix)]
#[test]
fn this_process_is_alive_and_a_process_that_cannot_exist_is_not() {
    assert!(is_alive(std::process::id()));
    assert!(!is_alive(u32::MAX));
    assert!(!is_alive(4_194_303 + 1_000_000));
}

#[test]
fn the_sweep_removes_what_a_dead_process_left_and_nothing_else() {
    let temp = scratch("sweep");
    let me = std::process::id();
    let dead = temp.join(format!("{PROFILE_PREFIX}4000000-0"));
    let alive = temp.join(format!("{PROFILE_PREFIX}{me}-0"));
    let other = temp.join("somebody-elses-folder");
    let old_style = temp.join("rust-headless-chrome-profileXyZ");
    for dir in [&dead, &alive, &other, &old_style] {
        std::fs::create_dir_all(dir.join("Default")).expect("a folder");
        std::fs::write(dir.join("Default").join("file"), "x").expect("a file");
    }
    let removed = sweep_stale(&temp, |pid| pid == me);
    assert_eq!(removed, vec![dead.clone()]);
    assert!(!dead.exists(), "the folder of a dead process is removed with what is in it");
    assert!(
        alive.exists() && other.exists() && old_style.exists(),
        "everything else is left"
    );
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn a_sweep_of_a_directory_that_is_not_there_does_nothing() {
    assert!(sweep_stale(Path::new("/this/does/not/exist"), |_| false).is_empty());
}

#[cfg(unix)]
#[test]
fn shutting_down_kills_the_process_that_was_registered_and_removes_its_folder() {
    let temp = scratch("shutdown");
    let profile = temp.join(format!("{PROFILE_PREFIX}{}-9", std::process::id()));
    std::fs::create_dir_all(&profile).expect("a folder");
    // A process that would run for a minute, and that has the folder's name on its command line, as Chrome does.
    let mut child = Command::new("sleep").arg("60").spawn().expect("sleep starts");
    register(child.id(), profile.clone());
    assert!(is_alive(child.id()));
    shut_down();
    let status = child.wait().expect("the child is waited for");
    assert!(!status.success(), "it was stopped by a signal, and did not finish");
    assert!(!profile.exists(), "its profile folder is removed");
    let _ = std::fs::remove_dir_all(&temp);
}

#[cfg(unix)]
#[test]
fn shutting_down_twice_is_harmless() {
    shut_down();
    shut_down();
}

/// One line of `ps -axo pid=,ppid=,command=`.
fn line(pid: u32, ppid: u32, command: &str) -> String {
    format!("{pid:>6} {ppid:>6} {command}\n")
}

const CHROME: &str = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const REPORTER: &str =
    "/Applications/Google Chrome.app/Contents/Frameworks/x/Helpers/chrome_crashpad_handler --monitor-self";

#[test]
fn a_crash_reporter_is_a_leftover_when_no_chrome_of_somebody_elses_is_running() {
    let ps = [
        line(1, 0, "/sbin/launchd"),
        line(900, 1, REPORTER),
        line(901, 1, REPORTER),
        line(
            902,
            1,
            "/Applications/Brave Browser.app/Contents/Frameworks/x/Helpers/chrome_crashpad_handler --monitor-self",
        ),
        line(903, 500, "/usr/bin/some-other-program"),
    ]
    .concat();
    assert_eq!(orphaned_crash_reporters(&ps), vec![900, 901], "only Chrome's, and not Brave's");
}

#[test]
fn a_crash_reporter_is_left_alone_when_somebody_is_running_their_own_chrome() {
    let ps = [
        line(900, 1, REPORTER),
        line(1200, 1, &format!("{CHROME} --restore-last-session")),
    ]
    .concat();
    assert!(orphaned_crash_reporters(&ps).is_empty());
}

#[test]
fn a_chrome_that_this_crate_launched_does_not_count_as_somebodys_own() {
    let ps = [
        line(900, 1, REPORTER),
        line(
            1200,
            1,
            &format!("{CHROME} --remote-debugging-port=1 --user-data-dir=/tmp/{PROFILE_PREFIX}7-0"),
        ),
        line(
            1201,
            1200,
            "/Applications/Google Chrome.app/x/Google Chrome Helper (Renderer).app/y --type=renderer",
        ),
    ]
    .concat();
    assert_eq!(orphaned_crash_reporters(&ps), vec![900]);
}

#[test]
fn a_crash_reporter_with_a_live_parent_is_not_a_leftover() {
    let ps = [line(900, 4321, REPORTER)].concat();
    assert!(orphaned_crash_reporters(&ps).is_empty());
}

#[test]
fn an_empty_list_of_processes_has_no_leftovers() {
    assert!(orphaned_crash_reporters("").is_empty());
    assert!(orphaned_crash_reporters("garbage\n\n   \n").is_empty());
}
