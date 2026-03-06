//! `writ new` subcommand — create a new Writ project.

pub fn cmd_new(name: String) -> Result<(), String> {
    // Validate project name (alphanumeric, hyphens, underscores)
    if name.is_empty() {
        return Err("project name cannot be empty".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "invalid project name '{}'. use alphanumeric characters, hyphens, or underscores",
            name
        ));
    }

    // Create project directory
    std::fs::create_dir(&name)
        .map_err(|e| format!("failed to create directory '{}': {}", name, e))?;

    let project_dir = std::path::Path::new(&name);

    // Create subdirectories
    let dirs = ["sources", "bin/configuration"];
    for dir in &dirs {
        std::fs::create_dir_all(project_dir.join(dir))
            .map_err(|e| format!("failed to create directory '{}': {}", dir, e))?;
    }

    // Create writ.toml
    let toml_content = format!(
        r#"# Writ Project Configuration
# For full documentation, see: https://writ-lang.dev/spec

# ============================================================================
# [project] - Required. Project metadata
# ============================================================================
[project]
# Project name (used in compiled module metadata)
name = "{}"

# Semantic version following semver (https://semver.org/)
version = "0.1.0"

# Optional: Project authors
# authors = ["Your Name"]


# ============================================================================
# [locale] - Required. Localization configuration
# ============================================================================
[locale]
# Default locale for inline dialogue text in .writ source files
# Follows BCP 47 language tags (en, de, fr, ja, ko, zh, pt-BR, en-GB, etc.)
default = "en"

# Optional: All locales your project targets for the 'writ loc export' tool
# If omitted, only the default locale is assumed
# supported = ["en", "de", "fr"]


# ============================================================================
# [compiler] - Optional. Build settings
# ============================================================================
[compiler]
# Source directories (relative to writ.toml). If omitted, defaults to ["src/"]
sources = ["sources/"]

# Output directory for compiled .writc artifacts (relative to writ.toml)
# output = "build/"


# ============================================================================
# [locale.export] - Optional. Localization export configuration
# ============================================================================
# [locale.export]
# Output directory for localization CSV files (relative to writ.toml)
# output = "locale/"


# ============================================================================
# [libraries.<name>] - Optional. External library mappings
# ============================================================================
# Maps logical library names to architecture-specific binary names.
# Used by [Import("name")] attributes in your code.
#
# Resolution precedence (highest to lowest):
#   1. Architecture-specific override in [Import] attribute itself
#   2. Architecture-specific override in writ.toml [libraries.<name>]
#   3. 'default' key in writ.toml [libraries.<name>]
#   4. The logical name from [Import], as-is
#
# Example:
# [libraries.physics]
# default = "libphysics"
# x64 = "physics64"
# arm64 = "physics_arm"
#
# [libraries.audio]
# default = "fmod"


# ============================================================================
# [conditions] - Optional. Conditional compilation flags
# ============================================================================
# Named conditions for #[Conditional("name")] attributes in code.
# Can be overridden via CLI: writ compile --condition debug=false
#
# CLI flags take precedence over writ.toml values.
# Undefined conditions default to false.
#
# Example:
# [conditions]
# debug = true
# playstation = false
# xbox = false
# editor = true
"#,
        name
    );

    std::fs::write(project_dir.join("writ.toml"), toml_content)
        .map_err(|e| format!("failed to write writ.toml: {}", e))?;

    // Create .gitignore
    let gitignore_content = r#"# Build artifacts
/bin/configuration/*.writc
*.writc
*.writil

# Generated files
/build/
/dist/

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db
"#;

    std::fs::write(project_dir.join(".gitignore"), gitignore_content)
        .map_err(|e| format!("failed to write .gitignore: {}", e))?;

    // Create a skeleton main.writ file
    let writ_content = r#"// Entry point for your Writ project

pub fn main() {
    // TODO: Add your code here
}
"#;

    std::fs::write(project_dir.join("sources/main.writ"), writ_content)
        .map_err(|e| format!("failed to write sources/main.writ: {}", e))?;

    eprintln!(
        "Created Writ project '{}':\n  {}/\n  ├─ writ.toml\n  ├─ .gitignore\n  ├─ sources/\n  │  └─ main.writ\n  └─ bin/\n     └─ configuration/",
        name, name
    );
    eprintln!("\nNext steps:");
    eprintln!("  1. cd {}", name);
    eprintln!("  2. Edit sources/main.writ with your code");
    eprintln!("  3. Run 'writ build' to compile");

    Ok(())
}
