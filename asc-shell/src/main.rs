use std::{io::{stdin, IsTerminal, Read}, process::Command};

const VALID_COMMANDS: [&str; 1] = [
    "asc-server"
];

macro_rules! error {
    ($($t:tt)*) => {{
        eprintln!($($t)*);

        return Ok(());
    }};
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

    let mut child = cmd.spawn()?;

    child.wait()?;

    Ok(())
}
