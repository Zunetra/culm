// === Interpolation

use std::collections::BTreeMap;

/// The values a template may substitute.
/// Deliberately a closed set rather than arbitrary user data, so a template cannot come to depend on something the wizard does not ask.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Variables {
    values: BTreeMap<String, String>,
}

impl Variables {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets one variable, replacing any previous value.
    pub fn with(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(name.into(), value.into());
        self
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    /// Replaces every `{{name}}` occurrence with its value.
    /// An unknown placeholder is left exactly as written rather than blanked, because a silently empty string in a generated config is far harder to notice than a literal `{{typo}}`.
    pub fn render(&self, source: &str) -> String {
        let mut out = String::with_capacity(source.len());
        let mut rest = source;

        while let Some(start) = rest.find("{{") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            match after.find("}}") {
                Some(end) => {
                    let name = after[..end].trim();
                    match self.get(name) {
                        Some(value) => out.push_str(value),
                        None => {
                            out.push_str("{{");
                            out.push_str(&after[..end]);
                            out.push_str("}}");
                        }
                    }
                    rest = &after[end + 2..];
                }
                /* An unterminated `{{` is content, not a placeholder. */
                None => {
                    out.push_str("{{");
                    rest = after;
                }
            }
        }

        out.push_str(rest);
        out
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Variables {
        Variables::new().with("project_name", "klassivox")
    }

    #[test]
    fn substitutes_a_known_placeholder() {
        assert_eq!(
            vars().render("name = \"{{project_name}}\""),
            "name = \"klassivox\""
        );
    }

    #[test]
    fn substitutes_every_occurrence() {
        assert_eq!(
            vars().render("{{project_name}}/{{project_name}}"),
            "klassivox/klassivox"
        );
    }

    #[test]
    fn tolerates_surrounding_whitespace() {
        assert_eq!(vars().render("{{ project_name }}"), "klassivox");
    }

    /* A blanked value in a generated config is far harder to spot than a
     * literal placeholder, so an unknown name is left alone on purpose. */
    #[test]
    fn leaves_an_unknown_placeholder_visible() {
        assert_eq!(vars().render("{{typo}}"), "{{typo}}");
    }

    #[test]
    fn leaves_an_unterminated_brace_alone() {
        assert_eq!(
            vars().render("fn main() {{ let x = 1;"),
            "fn main() {{ let x = 1;"
        );
    }

    #[test]
    fn passes_through_a_file_with_no_placeholders() {
        let source = "fn main() {\n    println!(\"hello\");\n}\n";
        assert_eq!(vars().render(source), source);
    }
}
