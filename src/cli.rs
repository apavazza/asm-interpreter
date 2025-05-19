use clap::Parser;

#[derive(Parser)]
#[command(name = crate::APP_NAME)]
#[command(version = crate::APP_VERSION)]
#[command(about = crate::APP_DESCRIPTION, long_about = None)]
pub struct Cli {
    /// Optional input file to execute.
    /// If not provided, the interpreter runs in interactive mode.
    pub input_file: Option<String>,
}