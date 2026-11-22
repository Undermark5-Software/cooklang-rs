//! The parsed recipe, as plain immutable records.

use cooklang::model as core;

use crate::diagnostics::Diagnostic;
use crate::metadata::Metadata;
use crate::quantity::Amount;

/// A parsed Cooklang recipe.
///
/// Steps refer to ingredients, cookware, timers and inline quantities by their
/// index in the matching list, see [`Item`].
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Recipe {
    pub metadata: Metadata,
    /// The recipe's sections. A recipe without `= Section` headers has a single
    /// section without a name.
    pub sections: Vec<Section>,
    pub ingredients: Vec<Ingredient>,
    pub cookware: Vec<Cookware>,
    pub timers: Vec<Timer>,
    /// Quantities written inline in step text.
    pub inline_quantities: Vec<Amount>,
    /// Problems that did not stop the recipe from being parsed.
    pub warnings: Vec<Diagnostic>,
}

/// A section of a recipe, optionally with a name.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Section {
    pub name: Option<String>,
    pub blocks: Vec<Block>,
}

/// A paragraph of a section: either a step or a note.
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Block {
    /// A cooking step. `number` starts at 1 in each section.
    Step { number: i32, items: Vec<Item> },
    /// A note (a `>` line), not counted as a step.
    Note { text: String },
}

/// A piece of a step: text, or a reference into one of the recipe's lists.
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Item {
    Text {
        value: String,
    },
    /// Index into [`Recipe::ingredients`].
    IngredientRef {
        index: i32,
    },
    /// Index into [`Recipe::cookware`].
    CookwareRef {
        index: i32,
    },
    /// Index into [`Recipe::timers`].
    TimerRef {
        index: i32,
    },
    /// Index into [`Recipe::inline_quantities`].
    InlineQuantityRef {
        index: i32,
    },
}

/// An ingredient used in the recipe.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Ingredient {
    pub name: String,
    /// The name to show, if different from `name` (`@name|alias{}`).
    pub alias: Option<String>,
    /// `alias` if there is one, otherwise `name`.
    pub display_name: String,
    pub amount: Option<Amount>,
    /// Text in parentheses after the ingredient, e.g. `@onion{1}(diced)`.
    pub note: Option<String>,
    /// Set if this ingredient is another recipe, e.g. `@./sauces/pesto{}`.
    pub reference: Option<RecipeReference>,
    /// Marked optional with `@?name`.
    pub optional: bool,
}

/// A piece of cookware used in the recipe.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Cookware {
    pub name: String,
    pub alias: Option<String>,
    pub display_name: String,
    pub amount: Option<Amount>,
    pub note: Option<String>,
    /// Marked optional with `#?name`.
    pub optional: bool,
}

/// A timer in the recipe.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Timer {
    pub name: Option<String>,
    pub amount: Option<Amount>,
}

/// A reference to another recipe file.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct RecipeReference {
    /// The recipe file name, without its directory.
    pub name: String,
    /// The directory components, e.g. `[".", "pasta"]` for `./pasta/recipe`.
    pub components: Vec<String>,
}

#[uniffi::export]
impl RecipeReference {
    /// The full path, joined with `separator`.
    #[uniffi::method(default(separator = "/"))]
    pub fn path(&self, separator: String) -> String {
        self.components
            .iter()
            .chain(std::iter::once(&self.name))
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(&separator)
    }
}

pub(crate) fn convert(
    recipe: &cooklang::Recipe,
    metadata: Metadata,
    warnings: Vec<Diagnostic>,
) -> Recipe {
    Recipe {
        metadata,
        sections: recipe.sections.iter().map(section).collect(),
        ingredients: recipe.ingredients.iter().map(ingredient).collect(),
        cookware: recipe.cookware.iter().map(cookware).collect(),
        timers: recipe.timers.iter().map(timer).collect(),
        inline_quantities: recipe.inline_quantities.iter().map(Amount::from).collect(),
        warnings,
    }
}

fn section(section: &core::Section) -> Section {
    Section {
        name: section.name.clone(),
        blocks: section
            .content
            .iter()
            .map(|content| match content {
                core::Content::Step(step) => Block::Step {
                    number: crate::int(step.number),
                    items: step.items.iter().map(item).collect(),
                },
                core::Content::Text(text) => Block::Note { text: text.clone() },
            })
            .collect(),
    }
}

fn item(item: &core::Item) -> Item {
    match item {
        core::Item::Text { value } => Item::Text {
            value: value.clone(),
        },
        core::Item::Ingredient { index } => Item::IngredientRef {
            index: crate::int(*index),
        },
        core::Item::Cookware { index } => Item::CookwareRef {
            index: crate::int(*index),
        },
        core::Item::Timer { index } => Item::TimerRef {
            index: crate::int(*index),
        },
        core::Item::InlineQuantity { index } => Item::InlineQuantityRef {
            index: crate::int(*index),
        },
    }
}

fn ingredient(ingredient: &core::Ingredient) -> Ingredient {
    Ingredient {
        name: ingredient.name.clone(),
        alias: ingredient.alias.clone(),
        display_name: ingredient.display_name().into_owned(),
        amount: ingredient.quantity.as_ref().map(Amount::from),
        note: ingredient.note.clone(),
        reference: ingredient.reference.as_ref().map(|r| RecipeReference {
            name: r.name.clone(),
            components: r.components.clone(),
        }),
        optional: ingredient.modifiers().is_optional(),
    }
}

fn cookware(cookware: &core::Cookware) -> Cookware {
    Cookware {
        name: cookware.name.clone(),
        alias: cookware.alias.clone(),
        display_name: cookware.display_name().to_string(),
        amount: cookware.quantity.as_ref().map(Amount::from),
        note: cookware.note.clone(),
        optional: cookware.modifiers().is_optional(),
    }
}

fn timer(timer: &core::Timer) -> Timer {
    Timer {
        name: timer.name.clone(),
        amount: timer.quantity.as_ref().map(Amount::from),
    }
}
