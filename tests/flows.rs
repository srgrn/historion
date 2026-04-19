use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn record_then_search_json_flow_works() {
    let temp_home = make_temp_dir("record-search-json");
    let project_dir = temp_home.join("project");
    fs::create_dir_all(&project_dir).expect("project dir should exist");

    let record = run_hy(
        &temp_home,
        Some(&project_dir),
        &[
            "record",
            "--cwd",
            project_dir.to_str().expect("project path should be utf8"),
            "--command",
            "cargo test --lib",
            "--history-id",
            "101",
            "--shell",
            "bash",
        ],
    );
    assert!(
        record.status.success(),
        "record failed: {}",
        String::from_utf8_lossy(&record.stderr)
    );

    let search = run_hy(
        &temp_home,
        Some(&project_dir),
        &["cargo", "--folder", ".", "--json"],
    );
    assert!(
        search.status.success(),
        "search failed: {}",
        String::from_utf8_lossy(&search.stderr)
    );

    let stdout = String::from_utf8(search.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("\"command\":\"cargo test --lib\""));
    assert!(stdout.contains(project_dir.to_string_lossy().as_ref()));

    cleanup(&temp_home);
}

#[test]
fn duplicate_history_ids_are_suppressed_end_to_end() {
    let temp_home = make_temp_dir("record-dedupe");
    let project_dir = temp_home.join("project");
    fs::create_dir_all(&project_dir).expect("project dir should exist");

    for _ in 0..2 {
        let output = run_hy(
            &temp_home,
            Some(&project_dir),
            &[
                "record",
                "--cwd",
                project_dir.to_str().expect("project path should be utf8"),
                "--command",
                "cargo test",
                "--history-id",
                "77",
                "--shell",
                "zsh",
            ],
        );
        assert!(
            output.status.success(),
            "record failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let search = run_hy(&temp_home, Some(&project_dir), &["cargo", "--folder", "."]);
    assert!(
        search.status.success(),
        "search failed: {}",
        String::from_utf8_lossy(&search.stderr)
    );

    let stdout = String::from_utf8(search.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.lines().count(), 1);

    cleanup(&temp_home);
}

#[test]
fn folder_only_search_returns_all_commands_in_tree() {
    let temp_home = make_temp_dir("folder-only-search");
    let project_dir = temp_home.join("project");
    let subdir = project_dir.join("src");
    fs::create_dir_all(&subdir).expect("project dir should exist");

    for (history_id, cwd, command) in [
        ("1", &project_dir, "cargo check"),
        ("2", &subdir, "rustc main.rs"),
    ] {
        let output = run_hy(
            &temp_home,
            Some(&project_dir),
            &[
                "record",
                "--cwd",
                cwd.to_str().expect("cwd path should be utf8"),
                "--command",
                command,
                "--history-id",
                history_id,
                "--shell",
                "bash",
            ],
        );
        assert!(
            output.status.success(),
            "record failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let search = run_hy(&temp_home, Some(&project_dir), &["--folder", "."]);
    assert!(
        search.status.success(),
        "search failed: {}",
        String::from_utf8_lossy(&search.stderr)
    );

    let stdout = String::from_utf8(search.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.lines().count(), 2);
    assert!(stdout.contains("cargo check"));
    assert!(stdout.contains("rustc main.rs"));

    cleanup(&temp_home);
}

#[test]
fn ignore_case_env_enables_case_insensitive_search() {
    let temp_home = make_temp_dir("ignore-case-env");
    let project_dir = temp_home.join("project");
    fs::create_dir_all(&project_dir).expect("project dir should exist");

    let record = run_hy(
        &temp_home,
        Some(&project_dir),
        &[
            "record",
            "--cwd",
            project_dir.to_str().expect("project path should be utf8"),
            "--command",
            "Cargo Test",
            "--history-id",
            "201",
            "--shell",
            "bash",
        ],
    );
    assert!(
        record.status.success(),
        "record failed: {}",
        String::from_utf8_lossy(&record.stderr)
    );

    let search = run_hy_with_env(
        &temp_home,
        Some(&project_dir),
        &["cargo"],
        &[("HY_IGNORE_CASE", "1")],
    );
    assert!(
        search.status.success(),
        "search failed: {}",
        String::from_utf8_lossy(&search.stderr)
    );

    let stdout = String::from_utf8(search.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Cargo Test"));

    cleanup(&temp_home);
}

fn run_hy(home_dir: &Path, current_dir: Option<&Path>, args: &[&str]) -> Output {
    run_hy_with_env(home_dir, current_dir, args, &[])
}

fn run_hy_with_env(
    home_dir: &Path,
    current_dir: Option<&Path>,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hy"));
    command.env("HOME", home_dir);

    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }

    for (key, value) in extra_env {
        command.env(key, value);
    }

    command.args(args).output().expect("hy command should run")
}

fn make_temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("hy-tests-{label}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}
