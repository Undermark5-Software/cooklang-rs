//! Aisle configuration: which shop aisle each ingredient is found in.

use std::collections::HashMap;
use std::sync::Arc;

use crate::diagnostics::{diagnostics, CooklangError, Diagnostic};
use crate::quantity::{CombinedIngredient, Combiner};

/// A category (aisle) and the ingredients in it.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct AisleCategory {
    pub name: String,
    pub ingredients: Vec<AisleIngredient>,
}

/// An ingredient in an aisle, with the other names it goes by.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct AisleIngredient {
    pub name: String,
    pub aliases: Vec<String>,
}

/// A parsed aisle configuration, with fast lookups by ingredient name.
///
/// Create one with `AisleConfig.parse(text)`.
#[derive(uniffi::Object, Debug)]
pub struct AisleConfig {
    categories: Vec<AisleCategory>,
    warnings: Vec<Diagnostic>,
    /// lowercase name or alias -> category name
    category_by_name: HashMap<String, String>,
    /// lowercase name or alias -> common (first) name
    common_names: HashMap<String, String>,
}

#[uniffi::export]
impl AisleConfig {
    /// Parses an aisle configuration file.
    ///
    /// Duplicate ingredients are reported in `warnings` instead of failing.
    #[uniffi::constructor]
    pub fn parse(input: String) -> Result<Arc<Self>, CooklangError> {
        let (parsed, report) = cooklang::aisle::parse_lenient(&input)
            .into_result()
            .map_err(|report| CooklangError::from_report(&report, &input))?;

        let mut category_by_name = HashMap::new();
        let mut common_names = HashMap::new();
        let mut categories = Vec::with_capacity(parsed.categories.len());

        for category in &parsed.categories {
            let mut ingredients = Vec::with_capacity(category.ingredients.len());
            for ingredient in &category.ingredients {
                let Some((name, aliases)) = ingredient.names.split_first() else {
                    continue;
                };
                for alias in std::iter::once(name).chain(aliases) {
                    let key = alias.to_lowercase();
                    category_by_name.insert(key.clone(), category.name.to_string());
                    common_names.insert(key, name.to_string());
                }
                ingredients.push(AisleIngredient {
                    name: name.to_string(),
                    aliases: aliases.iter().map(|a| a.to_string()).collect(),
                });
            }
            categories.push(AisleCategory {
                name: category.name.to_string(),
                ingredients,
            });
        }

        Ok(Arc::new(AisleConfig {
            categories,
            warnings: diagnostics(&report, &input),
            category_by_name,
            common_names,
        }))
    }

    /// The categories, in the order they appear in the file.
    pub fn categories(&self) -> Vec<AisleCategory> {
        self.categories.clone()
    }

    /// Problems found while parsing that did not stop it.
    pub fn warnings(&self) -> Vec<Diagnostic> {
        self.warnings.clone()
    }

    /// The category an ingredient belongs to, matching names and aliases
    /// case-insensitively.
    pub fn category_for(&self, ingredient_name: String) -> Option<String> {
        self.category_by_name
            .get(&ingredient_name.to_lowercase())
            .cloned()
    }

    /// The common name of an ingredient (the first name in its line), matching
    /// names and aliases case-insensitively. Unknown ingredients keep their
    /// name.
    pub fn common_name_for(&self, ingredient_name: String) -> String {
        self.common_names
            .get(&ingredient_name.to_lowercase())
            .cloned()
            .unwrap_or(ingredient_name)
    }

    /// Renames ingredients to their common names, combining any that end up
    /// with the same name.
    pub fn use_common_names(
        &self,
        ingredients: Vec<CombinedIngredient>,
    ) -> Vec<CombinedIngredient> {
        let mut combiner = Combiner::default();
        for ingredient in ingredients {
            combiner.add(self.common_name_for(ingredient.name), ingredient.amounts);
        }
        combiner.finish()
    }
}
