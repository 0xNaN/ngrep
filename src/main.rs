mod commands;
mod config;
mod displayers;
mod neural_matchers;

use std::path::PathBuf;

use anyhow::{Error, Result};
use clap::{Parser, Subcommand};
use commands::{handle_config, handle_import, handle_match};
use config::NgrepConfig;

// --- Command Line Arguments
#[derive(Parser, Clone, Debug)]
#[command(name = "ngrep")]
#[command(version = "0.1.0")]
#[command(version, about, about = "A neural grep")]
pub struct Args {
    #[command(subcommand, name = "command")]
    command: Option<Commands>,

    /// Model name
    #[arg(short, long)]
    model: Option<String>,

    /// Similarity threshold
    #[arg(short, long)]
    threshold: Option<f64>,

    /// The search pattern
    #[arg(name = "pattern")]
    pattern: Option<String>,

    /// File to search in or '-' for stdin.
    #[arg(name = "file", default_value = "-")]
    file: String,

    /// Prefix each line with its line number, starting at line 1.
    #[arg(long, short = 'n', default_value = "false")]
    line_number: bool,

    /// Prints only the matching part of the line
    #[arg(long, short = 'o', default_value = "false")]
    only_matching: bool,

    /// Recursively search subdirectories listed
    #[arg(long, short = 'r', short_alias = 'R', default_value = "false")]
    recursive: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Import a supported model
    Import {
        /// Path to a model
        #[arg(name = "path")]
        path: PathBuf,

        /// Name of the model
        #[arg(name = "name")]
        name: String,

        /// Default threshold for the model
        #[arg(short, long, default_value = "0.5")]
        threshold: f64,

        /// Do not set as default
        #[arg(short, long, default_value = "false")]
        no_default: bool,
    },
    /// Edit ~/.ngrep/config.toml
    Config,
}

fn main() -> Result<(), Error> {
    let args: Args = Args::parse();
    let mut config = NgrepConfig::load_or_init(&args)?;

    if let Some(command) = args.command {
        return match command {
            Commands::Import {
                path,
                name,
                threshold,
                no_default,
            } => handle_import(&mut config, path, &name, threshold, !no_default),
            Commands::Config {} => handle_config(&config),
        };
    }

    handle_match(&mut config, args)
}
