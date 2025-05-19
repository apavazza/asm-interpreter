use std::env;
use clap::Parser;
use ctrlc;
use std::fs::File;
use std::io::BufReader;

mod interpreter;
mod cli;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> std::io::Result<()> {
    ctrlc::set_handler(|| {
        println!("\nCtrl-C pressed. Exiting...");
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");
    
    let cli = cli::Cli::parse();

    if let Some(input_file) = cli.input_file {
        let file = File::open(&input_file)?;
        let reader = BufReader::new(file);
        interpreter::run_with_reader(reader, false);
    } else {
        println!("Welcome to the Assembly Interpreter.");
        interpreter::interactive();
    }

    Ok(())
}
