// === Categories and cardinality

use std::fmt;

use serde::Deserialize;

/// The role a layer plays, which is what the user actually chooses between.
/// Language is implied by the choice rather than asked for separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// Applies to every workspace whatever else is selected.
    /// Carries the ignores that belong to no framework, so a selection is never left without the OS and editor noise covered.
    Base,
    Shape,
    Frontend,
    Backend,
    /// The data layer, which is a separate choice from the HTTP framework.
    /// Nest with Prisma is an ordinary pairing, so they cannot share a category and its cardinality.
    Data,
    Toolchain,
    Quality,
    Runner,
    Infra,
    Editor,
}

/// Every category, in declaration order.
pub const ALL_CATEGORIES: [Category; 10] = [
    Category::Base,
    Category::Shape,
    Category::Frontend,
    Category::Backend,
    Category::Data,
    Category::Toolchain,
    Category::Quality,
    Category::Runner,
    Category::Infra,
    Category::Editor,
];

/// How many layers of one category a selection may contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cardinality {
    /// The category must be empty, and any layer in it is an error.
    None,
    ExactlyOne,
    OneOrMore,
    ZeroOrOne,
    ZeroOrMore,
}

/// Whether the workspace holds one app, many, or none at all.
/// Cardinality depends on it, because "which framework does this app use" and "how many apps are there" are different questions that a flat per-category rule cannot tell apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    SingleApp,
    Monorepo,
    /// No app scaffolding, for dropping ipile onto a repository that already exists.
    /// Named for what it gives you rather than for what it withholds, and it avoids reading as `Option::None` at every match site.
    ToolingOnly,
}

impl Shape {
    pub(super) fn from_id(id: &str) -> Option<Self> {
        match id {
            "single-app" => Some(Self::SingleApp),
            "monorepo" => Some(Self::Monorepo),
            "tooling-only" => Some(Self::ToolingOnly),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SingleApp => "single-app",
            Self::Monorepo => "monorepo",
            Self::ToolingOnly => "tooling-only",
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Category {
    /// The cardinality rule for this category, given the workspace shape.
    /// Declared here rather than as pairwise `conflicts` entries, so changing a category is one edit instead of one per member.
    pub fn cardinality(self, shape: Shape) -> Cardinality {
        match self {
            /* Always present, whatever the shape. Without it a Rust
             * tooling-only selection would ship a .gitignore holding nothing
             * but /target, and the OS and editor noise every repository
             * accumulates would be nobody's job. */
            Self::Base => Cardinality::ExactlyOne,
            Self::Shape => Cardinality::ExactlyOne,
            Self::Runner => Cardinality::ExactlyOne,
            /* One app runs on one framework, so a single-app workspace takes at most one of each.
             * A monorepo is many apps, and holding a Nuxt site
             * beside a React admin panel, or a Rust service beside a Node BFF, is
             * the reason someone reaches for a monorepo at all.
             * Forbidding that
             * would forbid the case ipile exists to serve. */
            Self::Frontend | Self::Backend | Self::Data => match shape {
                Shape::SingleApp => Cardinality::ZeroOrOne,
                Shape::Monorepo => Cardinality::ZeroOrMore,
                /* A framework layer scaffolds an app, and an app implies a
                 * shape. Asking for tooling only and then naming a framework
                 * is a contradiction, so it is caught here rather than
                 * half-scaffolded. */
                Shape::ToolingOnly => Cardinality::None,
            },
            /* Normally the toolchain is implied by the framework choice and
             * never asked. With no framework there is nothing to imply it, so
             * it becomes the explicit answer to "tooling for which ecosystem". */
            Self::Toolchain => match shape {
                Shape::ToolingOnly => Cardinality::OneOrMore,
                _ => Cardinality::ZeroOrMore,
            },
            Self::Quality => Cardinality::ZeroOrMore,
            Self::Infra => Cardinality::ZeroOrMore,
            Self::Editor => Cardinality::ZeroOrMore,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Shape => "shape",
            Self::Frontend => "frontend",
            Self::Backend => "backend",
            Self::Data => "data",
            Self::Toolchain => "toolchain",
            Self::Quality => "quality",
            Self::Runner => "runner",
            Self::Infra => "infra",
            Self::Editor => "editor",
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_app_takes_one_framework_of_each_kind() {
        assert_eq!(
            Category::Frontend.cardinality(Shape::SingleApp),
            Cardinality::ZeroOrOne
        );
        assert_eq!(
            Category::Backend.cardinality(Shape::SingleApp),
            Cardinality::ZeroOrOne
        );
    }

    #[test]
    fn a_monorepo_holds_many_apps() {
        assert_eq!(
            Category::Frontend.cardinality(Shape::Monorepo),
            Cardinality::ZeroOrMore
        );
        assert_eq!(
            Category::Backend.cardinality(Shape::Monorepo),
            Cardinality::ZeroOrMore
        );
    }

    #[test]
    fn a_runner_is_required_whatever_the_shape() {
        assert_eq!(
            Category::Runner.cardinality(Shape::Monorepo),
            Cardinality::ExactlyOne
        );
        assert_eq!(
            Category::Runner.cardinality(Shape::SingleApp),
            Cardinality::ExactlyOne
        );
    }
}
