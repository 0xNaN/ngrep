use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs::File, io::Write};

use anyhow::{bail, Context, Ok, Result};
use shellexpand::tilde;
use toml_edit::{DocumentMut, Table, Value};

use crate::Args;

const NGREP_HOME: &str = "~/.ngrep";
const NGREP_TOML_CONFIG: &str = "config.toml";
const NGREP_TOML_INIT: &str = r#"# ngrep configuration
#
# [ngrep]
# model = <name>             # set a default model
#
# [models.<name>]
# path = <ng-path>           # path to the .ng model
# threshold = <number>       # default threshold for this model

[ngrep]
"#;

#[derive(Debug, Clone)]
struct TomlConfig {
    doc: DocumentMut,
    path: PathBuf,
}

impl TomlConfig {
    pub fn load_or_init<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::maybe_init(&path)?;

        Ok(TomlConfig {
            path: path.as_ref().to_path_buf(),
            doc: Self::load_toml(path)?,
        })
    }

    pub fn path(self: &Self) -> PathBuf {
        self.path.clone()
    }

    pub fn home(self: &Self) -> PathBuf {
        self.path
            .parent()
            .expect("Error getting parent")
            .to_path_buf()
    }

    fn maybe_init<P: AsRef<Path>>(path: P) -> Result<()> {
        if let Some(parent_dir) = path.as_ref().to_path_buf().parent() {
            fs::create_dir_all(parent_dir)?;
        }

        File::create_new(&path)
            .and_then(|mut handle| handle.write_all(NGREP_TOML_INIT.as_bytes()))
            .ok();

        Ok(())
    }

    fn load_toml<P: AsRef<Path>>(path: P) -> Result<DocumentMut> {
        let path_str = path.as_ref().to_string_lossy();
        let mut file = File::open(&path).context(format!("Error opening '{}'", path_str))?;
        let mut toml: String = String::new();

        file.read_to_string(&mut toml)?;

        toml.parse::<DocumentMut>()
            .context("Invalid TOML configuration")
    }

    pub fn get_table(self: &Self, key: &str) -> Result<&Table> {
        let keys: Vec<&str> = key.split(".").collect();

        let mut table = self.doc.as_table();
        for key in keys {
            table = table
                .get(key)
                .context(format!("Specified key '{}' not found", key))?
                .as_table()
                .context(format!("Specified key '{}' is not a table", key))?;
        }

        Ok(table)
    }

    pub fn get_value(self: &Self, key: &str) -> Result<&Value> {
        let mut keys: Vec<&str> = key.split(".").collect();
        let value = keys.pop().unwrap();

        self.get_table(keys.join(".").as_str())?
            .get(value)
            .context(format!("key '{}' not found in configuration", key))?
            .as_value()
            .context(format!("Can't read '{}' value", key))
    }

    pub fn add_value<V: Into<toml_edit::Value>>(
        self: &mut Self,
        table_key: &str,
        key: &str,
        value: V,
    ) -> Result<()> {
        let keys: Vec<&str> = table_key.split('.').collect();
        let last = keys.len().saturating_sub(1);

        let mut table: &mut toml_edit::Table = self.doc.as_table_mut();
        for (idx, key) in keys.iter().enumerate() {
            if table.contains_key(key) {
                table = table[*key].as_table_mut().unwrap();
            } else {
                table[*key] = toml_edit::table();
                let next = table[*key].as_table_mut().unwrap();
                if idx != last {
                    next.set_implicit(true);
                }
                table = next;
            }
        }

        table[key] = toml_edit::value(value);

        let mut file = File::create(&self.path)?;
        file.write_all(self.doc.to_string().as_bytes())
            .context("Error updating configuration")?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ModelConfig {
    pub name: String,
    pub path: PathBuf,
    pub threshold: f64,
}

impl ModelConfig {
    pub fn new(name: String, path: PathBuf, threshold: f64) -> Result<ModelConfig> {
        if name.contains('.') {
            bail!("Model name can't contains '.'")
        }

        Ok(ModelConfig {
            name,
            path,
            threshold,
        })
    }
}

#[derive(Debug)]
pub struct NgrepConfig {
    toml: TomlConfig,
    args: Args,
    model: Option<ModelConfig>,
}

impl NgrepConfig {
    pub fn load_or_init(args: &Args) -> Result<NgrepConfig> {
        let toml_path =
            PathBuf::from_iter([tilde(NGREP_HOME).to_string().as_str(), NGREP_TOML_CONFIG]);
        Self::load(args.clone(), TomlConfig::load_or_init(&toml_path)?)
    }

    fn load(args: Args, toml_config: TomlConfig) -> Result<NgrepConfig> {
        Ok(NgrepConfig {
            toml: toml_config,
            args: args,
            model: None,
        })
    }

    pub fn home(&self) -> PathBuf {
        self.toml.home()
    }

    pub fn path(&self) -> PathBuf {
        self.toml.path()
    }

    pub fn model(&mut self) -> Result<&ModelConfig> {
        match self.model {
            Some(ref model) => Ok(model),
            None => {
                self.model = Some(self.resolve_model()?);
                Ok(self.model.as_ref().unwrap())
            }
        }
    }

    pub fn add_model(&mut self, model: &ModelConfig, default: bool) -> Result<()> {
        let model_path = model.path.to_str().context("Invalid path provided")?;

        let table = format!("models.{}", model.name);
        self.toml.add_value(&table, "path", model_path)?;
        self.toml.add_value(&table, "threshold", model.threshold)?;

        if default {
            self.toml.add_value("ngrep", "model", &model.name)?
        }

        Ok(())
    }

    fn resolve_model(self: &Self) -> Result<ModelConfig> {
        let model_name: String = match self.args.model {
            Some(ref name) => name.into(),
            None => self
                .toml
                .get_value("ngrep.model")
                .and_then(|v| v.as_str().context("`model` expected to be a string"))?
                .into(),
        };

        let model_toml = self.toml.get_table(&format!("models.{}", model_name))?;

        let model_path = PathBuf::from(
            &model_toml
                .get("path")
                .context(format!("`.path` not found for model '{}'", model_name))?
                .as_str()
                .context("`path` expected to be a string")?,
        );
        let model_th: f64 = match self.args.threshold {
            Some(th) => th,
            None => model_toml
                .get("threshold")
                .context(format!("`.threshold` not found for model '{}'", model_name))?
                .as_float()
                .context("`.threshold` expected to be a float")?,
        };

        ModelConfig::new(model_name, model_path, model_th)
    }
}
