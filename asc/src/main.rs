mod commands;
use commands::run;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    
    run()
}
