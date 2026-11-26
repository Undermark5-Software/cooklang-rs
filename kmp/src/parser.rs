//! A parser with settings: unit overrides and which ingredients are liquids.

use std::sync::Arc;

use serde::Deserialize;

use crate::{diagnostics, metadata, recipe, CooklangError, Recipe};

/// Parses recipes with settings read from a config file, so they can change without a
/// rebuild.
///
/// The config is TOML with two optional sections:
///
/// ```toml
/// # Same format as cooklang's units.toml, layered over the bundled units: change
/// # fractions, extend units, or add new ones.
/// [units.fractions.unit]
/// cup = { denominators = [2, 3, 4] }
///
/// # Ingredients whose name contains one of `words` (whole words, plural allowed) are
/// # liquids, which are written in quarts from 1 quart up. `except` wins over `words`.
/// # Either list replaces the default one.
/// [liquids]
/// words = ["water", "broth", "stock", "milk"]
/// except = ["sour cream"]
/// ```
#[derive(uniffi::Object)]
pub struct RecipeParser {
    parser: cooklang::CooklangParser,
    liquids: Liquids,
}

#[uniffi::export]
impl RecipeParser {
    /// A parser with the settings in `config` (the TOML above), or the defaults if null.
    /// Throws `CooklangException.Config` if the config is invalid.
    #[uniffi::constructor]
    pub fn new(config: Option<String>) -> Result<Arc<Self>, CooklangError> {
        let config: Config = match config {
            Some(text) => toml::from_str(&text).map_err(|e| CooklangError::Config {
                reason: e.to_string(),
            })?,
            None => Config::default(),
        };
        let mut builder = cooklang::convert::ConverterBuilder::new()
            .with_units_file(cooklang::convert::UnitsFile::bundled())
            .map_err(config_error)?;
        if let Some(units) = config.units {
            builder = builder.with_units_file(units).map_err(config_error)?;
        }
        let converter = builder.finish().map_err(config_error)?;
        let liquids = config.liquids.unwrap_or_default();
        Ok(Arc::new(Self {
            parser: cooklang::CooklangParser::new(cooklang::Extensions::all(), converter),
            liquids: Liquids {
                words: liquids.words.unwrap_or_else(|| strings(DEFAULT_LIQUIDS)),
                except: liquids.except.unwrap_or_else(|| strings(DEFAULT_NOT_LIQUIDS)),
            },
        }))
    }

    /// Parses a recipe, scaling its quantities by `scale` (see [`crate::parse_recipe`]).
    pub fn parse(&self, input: String, scale: f64) -> Result<Recipe, CooklangError> {
        let (mut parsed, report) = self
            .parser
            .parse(&input)
            .into_result()
            .map_err(|report| CooklangError::from_report(&report, &input))?;
        // Unscaled, amounts stay as the recipe wrote them: no fitting, no written form.
        let as_written = scale == 1.0;
        if !as_written {
            parsed.scale(scale, self.parser.converter());
        }
        let metadata = metadata::convert(&parsed.metadata, self.parser.converter());
        let warnings = diagnostics::diagnostics(&report, &input);
        Ok(recipe::convert(
            &parsed,
            self.parser.converter(),
            &self.liquids,
            metadata,
            warnings,
            as_written,
        ))
    }

    /// Whether an ingredient with this name counts as a liquid.
    pub fn is_liquid(&self, name: String) -> bool {
        self.liquids.contains(&name)
    }
}

fn config_error(error: cooklang::convert::ConverterBuilderError) -> CooklangError {
    CooklangError::Config {
        reason: error.to_string(),
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Config {
    units: Option<cooklang::convert::UnitsFile>,
    liquids: Option<LiquidsConfig>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct LiquidsConfig {
    words: Option<Vec<String>>,
    except: Option<Vec<String>>,
}

const DEFAULT_LIQUIDS: &[&str] = &[
    "water",
    "broth",
    "stock",
    "milk",
    "buttermilk",
    "cream",
    "half and half",
];

const DEFAULT_NOT_LIQUIDS: &[&str] = &[
    "sour cream",
    "cream cheese",
    "ice cream",
    "whipped cream",
    "cream of",
    "milk chocolate",
    "powdered milk",
    "dry milk",
    "stock cube",
];

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Which ingredients are liquids, by words in their names.
pub(crate) struct Liquids {
    words: Vec<String>,
    except: Vec<String>,
}

impl Liquids {
    pub(crate) fn defaults() -> Self {
        Self {
            words: strings(DEFAULT_LIQUIDS),
            except: strings(DEFAULT_NOT_LIQUIDS),
        }
    }

    /// Whether `name` contains one of the words (or phrases), as whole words, and none
    /// of the exceptions. Case, hyphens and a plural `s` don't matter.
    pub(crate) fn contains(&self, name: &str) -> bool {
        let name = words_of(name);
        let has = |phrase: &String| {
            let phrase = words_of(phrase);
            !phrase.is_empty()
                && name.windows(phrase.len()).any(|window| {
                    window.iter().zip(&phrase).enumerate().all(|(i, (word, want))| {
                        word == want || (i == phrase.len() - 1 && *word == format!("{want}s"))
                    })
                })
        };
        !self.except.iter().any(has) && self.words.iter().any(has)
    }
}

fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_liquids_by_name() {
        let liquids = Liquids::defaults();
        for name in ["water", "low-sodium chicken broth", "bone broth", "beef broth", "vegetable stock/broth", "Whole Milk", "half-and-half", "heavy whipping cream", "vegetable stocks"] {
            assert!(liquids.contains(name), "{name}");
        }
        for name in ["apple juice", "fresh lime juice", "sour cream", "cream cheese", "olive oil", "maple syrup", "milk chocolate chips", "watermelon", "stockpot"] {
            assert!(!liquids.contains(name), "{name}");
        }
    }

    #[test]
    fn liquids_get_quarts_and_config_changes_the_rules() {
        let source = "Add @chicken broth{4%cups} and @flour{4%cups}.".to_string();
        let parser = RecipeParser::new(None).unwrap();
        let recipe = parser.parse(source.clone(), 2.0).unwrap();
        let shown: Vec<_> = recipe.ingredients.iter().map(|i| i.amount.as_ref().unwrap().format()).collect();
        assert_eq!(shown, ["2 qt", "8 c"]);

        let config = r#"
            [liquids]
            words = ["flour"]
        "#;
        let parser = RecipeParser::new(Some(config.into())).unwrap();
        let recipe = parser.parse(source, 2.0).unwrap();
        let shown: Vec<_> = recipe.ingredients.iter().map(|i| i.amount.as_ref().unwrap().format()).collect();
        assert_eq!(shown, ["8 c", "2 qt"]);
    }

    #[test]
    fn unit_overrides_apply_without_a_rebuild() {
        let source = "Add @sugar{5/12%cup}.".to_string();
        let default = RecipeParser::new(None).unwrap().parse(source.clone(), 2.0).unwrap();
        assert_eq!(default.ingredients[0].amount.as_ref().unwrap().format(), "5/6 c");
        let config = r#"
            [units.fractions.unit]
            cup = { denominators = [2, 3, 4, 8] }
        "#;
        let parser = RecipeParser::new(Some(config.into())).unwrap();
        let recipe = parser.parse(source, 2.0).unwrap();
        assert_eq!(recipe.ingredients[0].amount.as_ref().unwrap().format(), "3/4 c + 4 tsp");
    }

    #[test]
    fn unscaled_amounts_are_shown_as_written() {
        let source = "Add @chicken broth{8%cups}, @flour{1.5%cups}, @salt{3/16%tsp} and @oil{5%tbsp}. \
            Later add @&salt{1/2%tsp}."
            .to_string();
        let parser = RecipeParser::new(None).unwrap();
        let recipe = parser.parse(source.clone(), 1.0).unwrap();
        let shown: Vec<_> = recipe.ingredients.iter().map(|i| i.amount.as_ref().unwrap().format()).collect();
        assert_eq!(shown, ["8 cups", "1.5 cups", "3/16 tsp", "5 tbsp", "1/2 tsp"]);
        assert_eq!(recipe.ingredients[1].amount.as_ref().unwrap().quantity, crate::quantity::Value::Number { value: 1.5 });
        // A total over references isn't written in the recipe, so it gets the written form.
        let listed: Vec<_> = recipe.ingredient_list.iter().map(|l| l.amounts[0].format()).collect();
        assert_eq!(listed, ["8 cups", "1.5 cups", "5/8 tsp", "5 tbsp"]);

        let doubled = parser.parse(source, 2.0).unwrap();
        let shown: Vec<_> = doubled.ingredients.iter().map(|i| i.amount.as_ref().unwrap().format()).collect();
        assert_eq!(shown, ["1 gal", "3 c", "3/8 tsp", "5/8 c", "1 tsp"]);
    }

    #[test]
    fn bad_config_is_reported() {
        assert!(matches!(RecipeParser::new(Some("[nope]".into())), Err(CooklangError::Config { .. })));
        assert!(matches!(RecipeParser::new(Some("[liquids]\nwords = 3".into())), Err(CooklangError::Config { .. })));
    }
}
