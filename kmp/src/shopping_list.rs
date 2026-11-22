//! `.shopping-list` files and the `.shopping-checked` log.

use cooklang::shopping_list::{
    self as core, CheckEntry as CoreCheckEntry, IngredientItem as CoreIngredientItem,
    RecipeItem as CoreRecipeItem, ShoppingList as CoreShoppingList,
    ShoppingListItem as CoreShoppingListItem,
};

use crate::diagnostics::CooklangError;

/// A shopping list: recipes to cook and free-hand ingredients.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct ShoppingList {
    pub items: Vec<ShoppingListItem>,
}

#[uniffi::export]
impl ShoppingList {
    /// The list in `.shopping-list` format.
    pub fn to_text(&self) -> String {
        let mut buf = Vec::new();
        core::write(&CoreShoppingList::from(self), &mut buf).expect("writing to a Vec cannot fail");
        String::from_utf8(buf).expect("the writer only emits UTF-8")
    }
}

/// An entry of a [`ShoppingList`].
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum ShoppingListItem {
    /// A recipe, optionally scaled, with nested items. `optional` marks an
    /// accepted optional recipe (`? ./path`).
    Recipe {
        path: String,
        multiplier: Option<f64>,
        children: Vec<ShoppingListItem>,
        optional: bool,
    },
    /// A free-hand ingredient. `optional` marks an accepted optional ingredient
    /// of the parent recipe (`? name{quantity}`); its quantity is the final
    /// amount to buy.
    Ingredient {
        name: String,
        quantity: Option<String>,
        optional: bool,
    },
}

/// An entry of the append-only `.shopping-checked` log.
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum CheckEntry {
    /// `+ name`: the ingredient was checked off.
    Checked { name: String },
    /// `- name`: the ingredient was unchecked again.
    Unchecked { name: String },
}

#[uniffi::export]
impl CheckEntry {
    /// The entry as a log line, including the trailing newline.
    pub fn to_text(&self) -> String {
        let mut buf = Vec::new();
        core::write_check_entry(&CoreCheckEntry::from(self), &mut buf)
            .expect("writing to a Vec cannot fail");
        String::from_utf8(buf).expect("the writer only emits UTF-8")
    }
}

/// Parses a `.shopping-list` file.
#[uniffi::export]
pub fn parse_shopping_list(input: String) -> Result<ShoppingList, CooklangError> {
    core::parse(&input)
        .map(|list| ShoppingList::from(&list))
        .map_err(|e| CooklangError::from_message(e.to_string()))
}

/// Parses a `.shopping-checked` log.
#[uniffi::export]
pub fn parse_checked_log(input: String) -> Vec<CheckEntry> {
    core::parse_checked(&input)
        .iter()
        .map(CheckEntry::from)
        .collect()
}

/// Replays a checked log and returns the ingredients that are currently
/// checked, lowercased and sorted.
#[uniffi::export]
pub fn checked_ingredients(entries: Vec<CheckEntry>) -> Vec<String> {
    let entries: Vec<CoreCheckEntry> = entries.iter().map(CoreCheckEntry::from).collect();
    let mut names: Vec<String> = core::checked_set(&entries).into_iter().collect();
    names.sort();
    names
}

/// Compacts a checked log: drops entries for ingredients that are no longer on
/// the list and keeps at most one entry per ingredient.
///
/// `current_ingredients` must be the ingredient names the user sees, after
/// expanding recipes; a [`ShoppingList`] on its own usually only holds recipe
/// references.
#[uniffi::export]
pub fn compact_checked_log(
    entries: Vec<CheckEntry>,
    current_ingredients: Vec<String>,
) -> Vec<CheckEntry> {
    let entries: Vec<CoreCheckEntry> = entries.iter().map(CoreCheckEntry::from).collect();
    core::compact_checked(&entries, current_ingredients.iter().map(String::as_str))
        .iter()
        .map(CheckEntry::from)
        .collect()
}

impl From<&CoreShoppingList> for ShoppingList {
    fn from(list: &CoreShoppingList) -> Self {
        ShoppingList {
            items: list.items.iter().map(ShoppingListItem::from).collect(),
        }
    }
}

impl From<&CoreShoppingListItem> for ShoppingListItem {
    fn from(item: &CoreShoppingListItem) -> Self {
        match item {
            CoreShoppingListItem::Recipe(r) => ShoppingListItem::Recipe {
                path: r.path.clone(),
                multiplier: r.multiplier,
                children: r.children.iter().map(ShoppingListItem::from).collect(),
                optional: r.optional,
            },
            CoreShoppingListItem::Ingredient(i) => ShoppingListItem::Ingredient {
                name: i.name.clone(),
                quantity: i.quantity.clone(),
                optional: i.optional,
            },
        }
    }
}

impl From<&ShoppingList> for CoreShoppingList {
    fn from(list: &ShoppingList) -> Self {
        CoreShoppingList {
            items: list.items.iter().map(CoreShoppingListItem::from).collect(),
        }
    }
}

impl From<&ShoppingListItem> for CoreShoppingListItem {
    fn from(item: &ShoppingListItem) -> Self {
        match item {
            ShoppingListItem::Recipe {
                path,
                multiplier,
                children,
                optional,
            } => CoreShoppingListItem::Recipe(CoreRecipeItem {
                path: path.clone(),
                multiplier: *multiplier,
                children: children.iter().map(CoreShoppingListItem::from).collect(),
                optional: *optional,
            }),
            ShoppingListItem::Ingredient {
                name,
                quantity,
                optional,
            } => CoreShoppingListItem::Ingredient(CoreIngredientItem {
                name: name.clone(),
                quantity: quantity.clone(),
                optional: *optional,
            }),
        }
    }
}

impl From<&CoreCheckEntry> for CheckEntry {
    fn from(entry: &CoreCheckEntry) -> Self {
        match entry {
            CoreCheckEntry::Checked(name) => CheckEntry::Checked { name: name.clone() },
            CoreCheckEntry::Unchecked(name) => CheckEntry::Unchecked { name: name.clone() },
        }
    }
}

impl From<&CheckEntry> for CoreCheckEntry {
    fn from(entry: &CheckEntry) -> Self {
        match entry {
            CheckEntry::Checked { name } => CoreCheckEntry::Checked(name.clone()),
            CheckEntry::Unchecked { name } => CoreCheckEntry::Unchecked(name.clone()),
        }
    }
}
