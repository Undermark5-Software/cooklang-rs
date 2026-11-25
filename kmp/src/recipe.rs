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
    /// The recipe's ingredient list: each ingredient once, in recipe order, with the
    /// amounts of every place it's used added up. Leaves out hidden ingredients
    /// (`@-name`), and references to other ingredients or to what a step made.
    pub ingredient_list: Vec<ListedIngredient>,
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
    /// Marked hidden with `@-name`: shown in its step, but not in ingredient lists.
    pub hidden: bool,
    pub relation: IngredientRelation,
}

/// How an ingredient relates to the recipe's other ingredients, steps and sections.
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum IngredientRelation {
    /// The ingredient's first use. `referenced_from` holds the indexes, in
    /// [`Recipe::ingredients`], of later uses that refer back to it (`@&name`).
    Definition { referenced_from: Vec<i32> },
    /// The same ingredient as [`Recipe::ingredients`] at `index`, used again (`@&name`).
    Ingredient { index: i32 },
    /// What a step made, like `@&(~1)mixture`: the step at `block` in the blocks of
    /// [`Recipe::sections`] at `section`.
    Step { section: i32, block: i32 },
    /// What a whole section made, like `@&(=2)sauce`: [`Recipe::sections`] at `index`.
    Section { index: i32 },
}

/// One entry of [`Recipe::ingredient_list`].
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct ListedIngredient {
    /// The ingredient's first use, in [`Recipe::ingredients`].
    pub index: i32,
    /// The total amount. More than one when the amounts can't be added, like
    /// `2 cups` and `a pinch`. Empty if no use gives an amount.
    pub amounts: Vec<Amount>,
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
    /// Marked hidden with `#-name`: shown in its step, but not in cookware lists.
    pub hidden: bool,
    /// For a later use of cookware (`#&pan`), the index of its first use in
    /// [`Recipe::cookware`].
    pub references: Option<i32>,
}

/// A timer in the recipe.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Timer {
    pub name: Option<String>,
    pub amount: Option<Amount>,
    /// How long the timer runs, if its amount is a number or range in a time unit.
    pub duration: Option<TimerDuration>,
}

/// A timer's length in seconds, with the ends in the order the recipe gives them. They are
/// equal unless the recipe gives a range, like `~{1h20m-1h25m}`. A backwards range keeps
/// its order, and warns at parse time.
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq)]
pub struct TimerDuration {
    pub start_seconds: f64,
    pub end_seconds: f64,
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
    converter: &cooklang::Converter,
    liquids: &crate::parser::Liquids,
    metadata: Metadata,
    warnings: Vec<Diagnostic>,
) -> Recipe {
    // The section each ingredient appears in, to place references to its steps.
    let mut ingredient_sections = vec![0; recipe.ingredients.len()];
    for (section_index, section) in recipe.sections.iter().enumerate() {
        for content in &section.content {
            if let core::Content::Step(step) = content {
                for item in &step.items {
                    if let core::Item::Ingredient { index } = item {
                        ingredient_sections[*index] = section_index;
                    }
                }
            }
        }
    }
    Recipe {
        metadata,
        sections: recipe.sections.iter().map(section).collect(),
        ingredients: recipe
            .ingredients
            .iter()
            .zip(&ingredient_sections)
            .map(|(i, section)| ingredient(i, *section, converter, liquids))
            .collect(),
        ingredient_list: recipe
            .group_ingredients(converter)
            .into_iter()
            .filter(|grouped| grouped.ingredient.modifiers().should_be_listed())
            .map(|grouped| {
                let liquid = is_liquid(grouped.ingredient, liquids);
                ListedIngredient {
                    index: crate::int(grouped.index),
                    amounts: grouped.quantity.iter().map(|q| Amount::new(q, converter, liquid)).collect(),
                }
            })
            .collect(),
        cookware: recipe.cookware.iter().map(|c| cookware(c, converter)).collect(),
        timers: recipe.timers.iter().map(|t| timer(t, converter)).collect(),
        inline_quantities: recipe.inline_quantities.iter().map(|q| Amount::new(q, converter, false)).collect(),
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

/// Whether the ingredient is a liquid, by its name or its alias.
fn is_liquid(ingredient: &core::Ingredient, liquids: &crate::parser::Liquids) -> bool {
    liquids.contains(&ingredient.name)
        || ingredient.alias.as_deref().is_some_and(|alias| liquids.contains(alias))
}

fn ingredient(
    ingredient: &core::Ingredient,
    section: usize,
    converter: &cooklang::Converter,
    liquids: &crate::parser::Liquids,
) -> Ingredient {
    let liquid = is_liquid(ingredient, liquids);
    use core::IngredientReferenceTarget as Target;
    let relation = match ingredient.relation.references_to() {
        None => IngredientRelation::Definition {
            referenced_from: ingredient
                .relation
                .referenced_from()
                .iter()
                .map(|i| crate::int(*i))
                .collect(),
        },
        Some((index, Target::Ingredient)) => IngredientRelation::Ingredient {
            index: crate::int(index),
        },
        Some((block, Target::Step)) => IngredientRelation::Step {
            section: crate::int(section),
            block: crate::int(block),
        },
        Some((index, Target::Section)) => IngredientRelation::Section {
            index: crate::int(index),
        },
    };
    Ingredient {
        name: ingredient.name.clone(),
        alias: ingredient.alias.clone(),
        display_name: ingredient.display_name().into_owned(),
        amount: ingredient.quantity.as_ref().map(|q| Amount::new(q, converter, liquid)),
        note: ingredient.note.clone(),
        reference: ingredient.reference.as_ref().map(|r| RecipeReference {
            name: r.name.clone(),
            components: r.components.clone(),
        }),
        optional: ingredient.modifiers().is_optional(),
        hidden: ingredient.modifiers().is_hidden(),
        relation,
    }
}

fn cookware(cookware: &core::Cookware, converter: &cooklang::Converter) -> Cookware {
    Cookware {
        name: cookware.name.clone(),
        alias: cookware.alias.clone(),
        display_name: cookware.display_name().to_string(),
        amount: cookware.quantity.as_ref().map(|q| Amount::new(q, converter, false)),
        note: cookware.note.clone(),
        optional: cookware.modifiers().is_optional(),
        hidden: cookware.modifiers().is_hidden(),
        references: cookware.relation.references_to().map(crate::int),
    }
}

fn timer(timer: &core::Timer, converter: &cooklang::Converter) -> Timer {
    Timer {
        name: timer.name.clone(),
        amount: timer.quantity.as_ref().map(|q| Amount::new(q, converter, false)),
        duration: timer.quantity.as_ref().and_then(|q| duration(q, converter)),
    }
}

/// The timer's length, using the parser's own units, so it agrees with the parser about
/// which units are times.
fn duration(
    quantity: &cooklang::quantity::Quantity,
    converter: &cooklang::Converter,
) -> Option<TimerDuration> {
    use cooklang::convert::PhysicalQuantity;
    use cooklang::quantity::Value as CoreValue;
    // A timer is a time, so a shared key like `m` means minutes here.
    let unit = converter.find_unit_in(quantity.unit()?, PhysicalQuantity::Time)?;
    let (start, end) = match quantity.value() {
        CoreValue::Number(n) => (n.value(), n.value()),
        CoreValue::Range { start, end } => (start.value(), end.value()),
        CoreValue::Text(_) => return None,
    };
    // Time units are defined in seconds.
    Some(TimerDuration {
        start_seconds: start * unit.ratio,
        end_seconds: end * unit.ratio,
    })
}
