//! Configuration loading for Writ projects.
//!
//! Parses `writ.toml` to discover project settings, source directories,
//! and locale configuration.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Top-level Writ project configuration, loaded from `writ.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct WritConfig {
    /// Project metadata.
    pub project: ProjectConfig,
    /// Locale settings (optional).
    pub locale: Option<LocaleConfig>,
    /// Compiler settings.
    #[serde(default)]
    pub compiler: CompilerConfig,
    /// Conditional compilation flags.
    #[serde(default)]
    pub conditions: HashMap<String, bool>,
}

/// Project metadata section.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    /// Project name.
    pub name: String,
    /// Project version.
    pub version: String,
}

/// Locale configuration section.
#[derive(Debug, Clone, Deserialize)]
pub struct LocaleConfig {
    /// Default locale identifier.
    pub default_locale: String,
    /// List of supported locale identifiers.
    pub locales: Vec<String>,
}

/// Compiler configuration section.
#[derive(Debug, Clone, Deserialize)]
pub struct CompilerConfig {
    /// Source directories to scan for `.writ` files.
    #[serde(default = "default_sources")]
    pub sources: Vec<String>,
    /// Output directory for compiled artifacts.
    pub output: Option<String>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            sources: default_sources(),
            output: None,
        }
    }
}

fn default_sources() -> Vec<String> {
    vec!["src/".to_string()]
}

/// Errors that can occur when loading configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// I/O error reading configuration file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Error parsing TOML configuration.
    #[error("configuration parse error: {0}")]
    Parse(#[from] toml::de::Error),
    /// Missing writ.toml file.
    #[error("writ.toml not found at {0}")]
    MissingToml(PathBuf),
}

/// Load the `writ.toml` configuration from the given project root.
///
/// Returns `ConfigError::MissingToml` if the file does not exist.
pub fn load_config(project_root: &Path) -> Result<WritConfig, ConfigError> {
    let toml_path = project_root.join("writ.toml");
    if !toml_path.exists() {
        return Err(ConfigError::MissingToml(toml_path));
    }
    let content = std::fs::read_to_string(&toml_path)?;
    let config: WritConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Discover all `.writ` source files in the configured source directories.
///
/// Recursively walks each source directory listed in `config.compiler.sources`,
/// relative to `project_root`, and returns all files ending in `.writ`.
pub fn discover_source_files(
    project_root: &Path,
    config: &WritConfig,
) -> Result<Vec<PathBuf>, ConfigError> {
    let mut files = Vec::new();
    for src_dir in &config.compiler.sources {
        let dir = project_root.join(src_dir);
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "writ") {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_basic_config() {
        let toml_str = r#"
[project]
name = "test-game"
version = "0.1.0"

[locale]
default_locale = "en"
locales = ["en", "ja"]

[compiler]
sources = ["src/", "scripts/"]
output = "build/"
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.project.name, "test-game");
        assert_eq!(config.project.version, "0.1.0");
        assert_eq!(config.locale.as_ref().unwrap().default_locale, "en");
        assert_eq!(config.compiler.sources, vec!["src/", "scripts/"]);
        assert_eq!(config.compiler.output.as_deref(), Some("build/"));
    }

    #[test]
    fn default_sources_when_omitted() {
        let toml_str = r#"
[project]
name = "test"
version = "0.1.0"
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.compiler.sources, vec!["src/"]);
    }

    #[test]
    fn discover_writ_files() {
        let tmp = std::env::temp_dir().join("writ_test_discover");
        let _ = fs::remove_dir_all(&tmp);
        let src = tmp.join("src").join("combat");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("weapons.writ"), "fn slash() {}").unwrap();
        fs::write(src.join("armor.writ"), "fn defend() {}").unwrap();
        fs::write(src.join("notes.txt"), "not a writ file").unwrap();

        let config: WritConfig = toml::from_str(r#"
[project]
name = "test"
version = "0.1.0"
"#).unwrap();

        let files = discover_source_files(&tmp, &config).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.extension().unwrap() == "writ"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn missing_toml_error() {
        let tmp = std::env::temp_dir().join("writ_test_missing");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let result = load_config(&tmp);
        assert!(matches!(result, Err(ConfigError::MissingToml(_))));

        let _ = fs::remove_dir_all(&tmp);
    }
}
