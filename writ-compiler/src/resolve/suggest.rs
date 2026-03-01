//! Fuzzy name suggestion engine.
//!
//! When a name cannot be resolved, this module produces "did you mean?"
//! suggestions using Jaro-Winkler similarity scoring from strsim.

use crate::resolve::def_map::DefMap;
use crate::resolve::prelude;

/// Similarity threshold for suggestions (Jaro-Winkler score).
const SIMILARITY_THRESHOLD: f64 = 0.8;

/// Maximum number of suggestions to show.
const MAX_SUGGESTIONS: usize = 3;

/// A name suggestion for an unresolved reference.
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// The suggested name.
    pub name: String,
    /// Fully-qualified name if different from simple name.
    pub fqn: Option<String>,
    /// Whether this suggestion requires a `using` import.
    pub needs_import: bool,
    /// Namespace to import if `needs_import` is true.
    pub import_ns: Option<String>,
    /// Similarity score (higher is better).
    pub score: f64,
}

impl Suggestion {
    /// Format the suggestion as a help string.
    pub fn to_help_string(&self) -> String {
        if self.needs_import {
            if let Some(ref fqn) = self.fqn {
                if let Some(ref ns) = self.import_ns {
                    return format!("did you mean `{fqn}`? (add `using {ns};`)");
                }
                return format!("did you mean `{fqn}`?");
            }
        }
        format!("did you mean `{}`?", self.name)
    }
}

/// Produce suggestions for an unresolved name.
///
/// `visible_names` are names already in scope (simple names).
/// `def_map` provides all names for cross-namespace search.
pub fn suggest_similar(
    unresolved: &str,
    visible_names: &[String],
    def_map: &DefMap,
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    // Phase A: Check visible names (already in scope)
    for name in visible_names {
        let score = strsim::jaro_winkler(unresolved, name);
        if score >= SIMILARITY_THRESHOLD && name != unresolved {
            suggestions.push(Suggestion {
                name: name.clone(),
                fqn: None,
                needs_import: false,
                import_ns: None,
                score,
            });
        }
    }

    // Also check primitives and prelude names
    for &prim in prelude::PRELUDE_PRIMITIVE_NAMES {
        let score = strsim::jaro_winkler(unresolved, prim);
        if score >= SIMILARITY_THRESHOLD && prim != unresolved {
            suggestions.push(Suggestion {
                name: prim.to_string(),
                fqn: None,
                needs_import: false,
                import_ns: None,
                score,
            });
        }
    }

    for &ty in prelude::PRELUDE_TYPE_NAMES {
        let score = strsim::jaro_winkler(unresolved, ty);
        if score >= SIMILARITY_THRESHOLD && ty != unresolved {
            suggestions.push(Suggestion {
                name: ty.to_string(),
                fqn: None,
                needs_import: false,
                import_ns: None,
                score,
            });
        }
    }

    for &contract in prelude::PRELUDE_CONTRACT_NAMES {
        let score = strsim::jaro_winkler(unresolved, contract);
        if score >= SIMILARITY_THRESHOLD && contract != unresolved {
            suggestions.push(Suggestion {
                name: contract.to_string(),
                fqn: None,
                needs_import: false,
                import_ns: None,
                score,
            });
        }
    }

    // Phase B: Cross-namespace search (only if we have room for more suggestions)
    if suggestions.len() < MAX_SUGGESTIONS {
        for (fqn, _) in &def_map.by_fqn {
            // Extract simple name from FQN
            let simple_name = fqn.rsplit("::").next().unwrap_or(fqn);
            let score = strsim::jaro_winkler(unresolved, simple_name);
            if score >= SIMILARITY_THRESHOLD && simple_name != unresolved {
                // Check if this is already in visible_names
                if !visible_names.contains(&simple_name.to_string()) {
                    let ns = if let Some(pos) = fqn.rfind("::") {
                        Some(fqn[..pos].to_string())
                    } else {
                        None
                    };
                    suggestions.push(Suggestion {
                        name: simple_name.to_string(),
                        fqn: Some(fqn.clone()),
                        needs_import: true,
                        import_ns: ns,
                        score,
                    });
                }
            }
        }
    }

    // Sort by score descending, take top MAX_SUGGESTIONS
    suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    suggestions.truncate(MAX_SUGGESTIONS);

    suggestions
}

/// Format a list of suggestions into a single help string.
pub fn format_suggestions(suggestions: &[Suggestion]) -> Option<String> {
    if suggestions.is_empty() {
        return None;
    }

    if suggestions.len() == 1 {
        return Some(suggestions[0].to_help_string());
    }

    let parts: Vec<String> = suggestions.iter().map(|s| s.to_help_string()).collect();
    Some(parts.join("\n  or: "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_close_match() {
        let visible = vec!["HealthPotion".to_string(), "ManaPotion".to_string()];
        let def_map = DefMap::new();
        let suggestions = suggest_similar("HelthPotion", &visible, &def_map);
        assert!(!suggestions.is_empty(), "should suggest HealthPotion");
        assert_eq!(suggestions[0].name, "HealthPotion");
    }

    #[test]
    fn no_suggest_for_unrelated() {
        let visible = vec!["HealthPotion".to_string()];
        let def_map = DefMap::new();
        let suggestions = suggest_similar("XyzAbc123", &visible, &def_map);
        assert!(suggestions.is_empty(), "should not suggest for unrelated name");
    }

    #[test]
    fn suggest_primitive() {
        let visible = vec![];
        let def_map = DefMap::new();
        let suggestions = suggest_similar("strng", &visible, &def_map);
        assert!(!suggestions.is_empty(), "should suggest 'string' for 'strng'");
        assert_eq!(suggestions[0].name, "string");
    }

    #[test]
    fn suggest_prelude_type() {
        let visible = vec![];
        let def_map = DefMap::new();
        let suggestions = suggest_similar("Optiom", &visible, &def_map);
        assert!(!suggestions.is_empty(), "should suggest 'Option' for 'Optiom'");
        assert_eq!(suggestions[0].name, "Option");
    }

    #[test]
    fn max_three_suggestions() {
        let visible = vec![
            "Apple".to_string(),
            "Applet".to_string(),
            "Applied".to_string(),
            "Appliance".to_string(),
        ];
        let def_map = DefMap::new();
        let suggestions = suggest_similar("Appl", &visible, &def_map);
        assert!(suggestions.len() <= 3, "should show at most 3 suggestions");
    }
}
