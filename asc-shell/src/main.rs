use std::{fs, io::{IsTerminal, Read, stdin}, process::Command};

use chrono::Utc;
use directories::BaseDirs;

const VALID_COMMANDS: [&str; 1] = [
    "asc-server"
];

macro_rules! error {
    ($($t:tt)*) => {{
        eprintln!($($t)*);

        return Ok(());
    }};
}

fn save_error(error: &eyre::Report) {
    let mut now = Utc::now().to_string();

    if let Some(i) = now.find('.') {
        let _ = now.split_off(i);
    }

    let name = format!("asc-server-{}", now);

    let Some(dirs) = BaseDirs::new() else {
        eprintln!("Failed to identify user directories through `directories` crate.");

        return;
    };

    let log_path = dirs.cache_dir().join(name);

    let _ = fs::write(
        log_path,
        format!("{error:?}")
    );
}

fn main() -> eyre::Result<()> {
    stable_eyre::install()?;

    let mut stdin = stdin().lock();

    if stdin.is_terminal() {
        error!("Interactive sessions are not allowed.");
    }

    let mut input = String::new();

    stdin.read_to_string(&mut input)?;

    let Some(split) = shlex::split(&input) else {
        error!("Failed to separate input into arguments.");
    };

    let Some(target) = split.first() else {
        error!("Nothing to run.");
    };

    if !VALID_COMMANDS.contains(&target.as_str()) {
        error!("Unknown command: {target:?}");
    }

    let mut cmd = Command::new(target);

    cmd.args(&split[1..]);

    let mut result = move || -> eyre::Result<()> {
        let mut child = cmd.spawn()?;

        child.wait()?;

        Ok(())
    };

    if let Err(e) = result() {
        save_error(&e);

        error!("{e:?}");
    }

    Ok(())
}
