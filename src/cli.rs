use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = crate::APP_NAME)]
#[command(version = crate::APP_VERSION)]
#[command(about = crate::APP_DESCRIPTION, long_about = None)]
pub struct Cli {
    /// The command to run
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run in interactive mode
    #[command(visible_aliases = &["r"])]
    Run {},
    /// Run from a file
    #[command(visible_aliases = &["e"])]
    Execute {
        /// Input file to execute
        input_file: String,
    },
}