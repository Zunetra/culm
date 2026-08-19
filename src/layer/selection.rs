// === Questions asked of a selection

/* Validation and resolution both need to know the shape, how many layers a
 * category holds, and which ones they are. Kept here rather than in either,
 * because the two would otherwise answer the same question in two places and
 * drift apart the first time one of them is corrected. */

use super::category::{Category, Shape};
use super::error::LayerError;
use super::manifest::Manifest;

/// The workspace shape a selection declares.
/// Errors rather than defaulting, because filling an empty shape slot is a policy decision and belongs to the resolver alone.
pub fn shape_of<'a, I>(manifests: I) -> Result<Shape, LayerError>
where
    I: IntoIterator<Item = &'a Manifest>,
{
    let names = names_in(manifests, Category::Shape);
    if names.len() > 1 {
        return Err(LayerError::CategoryOverfull {
            category: Category::Shape,
            chosen: names.len(),
            names,
        });
    }
    let id = names.first().ok_or(LayerError::ShapeMissing)?;
    Shape::from_id(id).ok_or_else(|| LayerError::UnknownShape { id: id.clone() })
}

/// How many layers of one category a selection holds.
pub fn count_in<'a, I>(manifests: I, category: Category) -> usize
where
    I: IntoIterator<Item = &'a Manifest>,
{
    manifests
        .into_iter()
        .filter(|manifest| manifest.category == category)
        .count()
}

/// The bare ids of one category's layers, sorted so an error message reads the same on every run.
pub fn names_in<'a, I>(manifests: I, category: Category) -> Vec<String>
where
    I: IntoIterator<Item = &'a Manifest>,
{
    let mut names: Vec<String> = manifests
        .into_iter()
        .filter(|manifest| manifest.category == category)
        .map(|manifest| manifest.id.clone())
        .collect();
    names.sort();
    names
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str, category: &str) -> Manifest {
        Manifest::parse(
            "layer.toml",
            &format!("id = \"{id}\"\ncategory = \"{category}\"\n"),
        )
        .unwrap()
    }

    #[test]
    fn the_shape_layer_names_the_shape() {
        let selection = vec![manifest("monorepo", "shape"), manifest("nuxt", "frontend")];
        assert_eq!(shape_of(&selection), Ok(Shape::Monorepo));
    }

    #[test]
    fn a_selection_without_a_shape_has_none_to_report() {
        let selection = vec![manifest("nuxt", "frontend")];
        assert_eq!(shape_of(&selection), Err(LayerError::ShapeMissing));
    }

    #[test]
    fn two_shapes_are_reported_together_in_a_stable_order() {
        let selection = vec![
            manifest("monorepo", "shape"),
            manifest("single-app", "shape"),
        ];
        assert_eq!(
            shape_of(&selection),
            Err(LayerError::CategoryOverfull {
                category: Category::Shape,
                chosen: 2,
                names: vec!["monorepo".to_owned(), "single-app".to_owned()],
            })
        );
    }

    #[test]
    fn names_come_back_sorted_whatever_order_they_went_in() {
        let selection = vec![manifest("nuxt", "frontend"), manifest("next", "frontend")];
        assert_eq!(
            names_in(&selection, Category::Frontend),
            vec!["next", "nuxt"]
        );
        assert_eq!(count_in(&selection, Category::Frontend), 2);
        assert_eq!(count_in(&selection, Category::Backend), 0);
    }
}
