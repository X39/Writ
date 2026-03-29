//! Configuration loading for Writ projects.
//!
//! Parses `writ.toml` to discover project settings, source directories,
//! and locale configuration.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A library dependency entry in `writ.toml`.
///
/// Supports both the short path string form:
/// ```toml
/// writ-std = "path/to/writ-std.writc"
/// ```
/// and the detailed table form:
/// ```toml
/// [dependencies.writ-std]
/// path = "path/to/writ-std.writc"
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DependencyConfig {
    /// Short-form: `name = "path/to/file.writc"`
    Path(String),
    /// Long-form: `[dependencies.name]\npath = "path/to/file.writc"`
    Detailed { path: String },
}

impl DependencyConfig {
    /// Return the file path for this dependency.
    pub fn path(&self) -> &str {
        match self {
            DependencyConfig::Path(p) => p,
            DependencyConfig::Detailed { path } => path,
        }
    }
}

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
    /// Build profile settings (debug and release).
    #[serde(default)]
    pub profile: ProfilesConfig,
    /// External library dependencies (`.writc` files).
    #[serde(default)]
    pub dependencies: HashMap<String, DependencyConfig>,
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
    /// Default locale identifier (TOML key: `default`).
    #[serde(rename = "default")]
    pub default_locale: String,
    /// Supported locale identifiers (TOML key: `supported`).
    #[serde(rename = "supported")]
    #[serde(default)]
    pub locales: Vec<String>,
}

/// Build profile configuration (debug vs release settings).
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    /// Whether to emit DebugLocal entries in the compiled module.
    #[serde(default = "default_debug_info")]
    pub debug_info: bool,
}

fn default_debug_info() -> bool {
    true
}

/// Profiles configuration section, containing debug and release profiles.
#[derive(Debug, Clone, Deserialize)]
pub struct ProfilesConfig {
    /// Debug profile settings.
    #[serde(default = "default_debug_profile")]
    pub debug: ProfileConfig,
    /// Release profile settings.
    #[serde(default = "default_release_profile")]
    pub release: ProfileConfig,
}

fn default_debug_profile() -> ProfileConfig {
    ProfileConfig { debug_info: true }
}

fn default_release_profile() -> ProfileConfig {
    ProfileConfig { debug_info: false }
}

impl Default for ProfilesConfig {
    fn default() -> Self {
        Self {
            debug: default_debug_profile(),
            release: default_release_profile(),
        }
    }
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
default = "en"
supported = ["en", "ja"]

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
    fn locale_without_supported() {
        let toml_str = r#"
[project]
name = "test-game"
version = "0.1.0"

[locale]
default = "en"
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.locale.as_ref().unwrap().default_locale, "en");
        assert!(config.locale.as_ref().unwrap().locales.is_empty());
    }

    #[test]
    fn scaffold_toml_round_trips() {
        // Mirrors the scaffold output from `writ new my-project` after the sources fix
        let toml_str = r#"
[project]
name = "my-project"
version = "0.1.0"

[locale]
default = "en"

[compiler]
sources = ["sources/"]
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.project.name, "my-project");
        assert_eq!(config.locale.as_ref().unwrap().default_locale, "en");
        assert!(config.locale.as_ref().unwrap().locales.is_empty());
        assert_eq!(config.compiler.sources, vec!["sources/"]);
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
    fn profile_defaults_when_omitted() {
        // No [profile] section — defaults apply
        let toml_str = r#"
[project]
name = "test"
version = "0.1.0"
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert!(config.profile.debug.debug_info, "debug profile should default to debug_info=true");
        assert!(!config.profile.release.debug_info, "release profile should default to debug_info=false");
    }

    #[test]
    fn profile_explicit_override() {
        // Both [profile.debug] and [profile.release] sections explicitly set
        let toml_str = r#"
[project]
name = "test"
version = "0.1.0"

[profile.debug]
debug_info = false

[profile.release]
debug_info = true
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.profile.debug.debug_info, "debug profile should be overridden to debug_info=false");
        assert!(config.profile.release.debug_info, "release profile should be overridden to debug_info=true");
    }

    #[test]
    fn profile_partial_override() {
        // Only [profile.release] is set; debug profile should use its default
        let toml_str = r#"
[project]
name = "test"
version = "0.1.0"

[profile.release]
debug_info = true
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert!(config.profile.debug.debug_info, "debug profile should keep its default of debug_info=true");
        assert!(config.profile.release.debug_info, "release profile should have the explicit override debug_info=true");
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

    #[test]
    fn parse_dependencies_config() {
        let toml_str = r#"
[project]
name = "test"
version = "0.1.0"

[dependencies]
writ-std = "path/to/writ-std.writc"
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.dependencies.len(), 1);
        assert_eq!(
            config.dependencies["writ-std"].path(),
            "path/to/writ-std.writc"
        );
    }

    #[test]
    fn parse_detailed_dependency_config() {
        let toml_str = r#"
[project]
name = "test"
version = "0.1.0"

[dependencies.writ-std]
path = "libs/writ-std.writc"
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.dependencies.len(), 1);
        assert_eq!(config.dependencies["writ-std"].path(), "libs/writ-std.writc");
    }

    #[test]
    fn dependencies_default_empty() {
        let toml_str = r#"
[project]
name = "test"
version = "0.1.0"
"#;
        let config: WritConfig = toml::from_str(toml_str).unwrap();
        assert!(config.dependencies.is_empty());
    }
}
