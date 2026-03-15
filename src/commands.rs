use clap::error::ErrorKind;
use clap::CommandFactory;
use edit::get_editor;
use fancy_regex::{Expr, NeuralMatcherFactory, RegexBuilder};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::iter;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, RwLock};
use walkdir::WalkDir;

use anyhow::{Context, Result};

use crate::config::{ModelConfig, NgrepConfig};
use crate::displayers::{MatchDisplayer, MatchDisplayerOptions};
use crate::neural_matchers::EmbedNeuralMatcherFactory;
use crate::Args;
use embeddings::converters::{self, Formats};
use embeddings::ng;

struct NamedReader {
    pub name: String,
    pub reader: Box<dyn BufRead>,
}

fn readers(path: &str, recursive: bool) -> Result<Box<dyn Iterator<Item = Result<NamedReader>>>> {
    if path == "-" {
        let reader: Box<dyn BufRead> = Box::new(io::stdin().lock());
        let reader = NamedReader {
            name: "(standard input)".to_string(),
            reader: reader,
        };
        return Ok(Box::new(iter::once(Ok(reader))));
    }

    let meta = std::fs::metadata(path).with_context(|| format!("failed to access: {}", path))?;
    if meta.is_dir() && !recursive {
        return Err(anyhow::anyhow!("{}: Is a directory", path));
    }

    let readers = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let path = e.path();
            let file = File::open(path).with_context(|| format!("failed to open: {:?}", path))?;
            let reader = NamedReader {
                name: path.to_string_lossy().into_owned(),
                reader: Box::new(BufReader::new(file)) as Box<dyn BufRead>,
            };

            Ok(reader)
        });

    Ok(Box::new(readers))
}

fn pattern_uses_neural(pattern: &str) -> Result<bool> {
    Ok(Expr::parse_tree(pattern)?.expr.contains_neural())
}

pub fn handle_import<P: AsRef<Path>>(
    config: &mut NgrepConfig,
    path: P,
    name: &str,
    threshold: f64,
    set_default: bool,
) -> Result<()> {
    let model_path = config.home().join(name).with_extension(ng::NG_EXTENSION);
    let model_path_temp = model_path.with_extension(format!("{}~", ng::NG_EXTENSION));

    converters::to_ng(Formats::Text, path.as_ref(), &model_path_temp)
        .context("Error during import of the model")
        .map_err(|e| {
            let _ = std::fs::remove_file(&model_path_temp);
            e
        })?;

    std::fs::rename(&model_path_temp, &model_path)?;

    let model_conf = ModelConfig::new(name.into(), model_path, threshold)?;
    config.add_model(&model_conf, set_default)
}

pub fn handle_config(config: &NgrepConfig) -> Result<()> {
    let editor = get_editor().context("No default $EDITOR found")?;

    let error = Command::new(editor).args([config.path()]).exec();

    eprintln!("Failed to exec: {}", error);
    std::process::exit(1);
}

pub fn handle_match(config: &mut NgrepConfig, args: Args) -> Result<()> {
    let mut cli = Args::command();

    if args.pattern.is_none() {
        cli.error(
            ErrorKind::MissingRequiredArgument,
            "Missing required argument: pattern",
        )
        .exit();
    }

    let pattern = args.pattern.as_ref().unwrap();

    // --- model initialization
    let mut neural_regex = RegexBuilder::new(pattern);
    if pattern_uses_neural(pattern).context("Invalid regex pattern")? {
        let model_config = config
            .model()
            .context("No default model found, run `ngrep import` first")?;
        let neural_matcher =
            EmbedNeuralMatcherFactory::new(&model_config.path, model_config.threshold);
        neural_regex.neural_matcher_factory(
            Arc::new(RwLock::new(neural_matcher)) as Arc<RwLock<dyn NeuralMatcherFactory>>
        );
    }

    let neural_regex = neural_regex.build().context("Invalid regex pattern")?;

    // --- matching loop
    let displayer = MatchDisplayer::new(
        MatchDisplayerOptions::default()
            .with_line_number(args.line_number)
            .with_only_matching(args.only_matching)
            .with_file_name(args.recursive),
    );

    let mut stdout = std::io::stdout().lock();

    for input in readers(&args.file, args.recursive)? {
        let input = input?;

        for (inx, line) in input.reader.lines().enumerate() {
            let line = line?;
            let captures: Vec<(usize, usize)> = neural_regex
                .find_iter(line.as_str())
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_context(|| format!("Regex matching failed on line {}", inx + 1))?
                .into_iter()
                .map(|cap| (cap.start(), cap.end()))
                .collect();

            if !captures.is_empty() {
                let ret = displayer.display_line(&mut stdout, &input.name, inx, &line, &captures);
                if let Err(e) = ret {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        std::process::exit(0);
                    }
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}
