//! Recipe metadata (the YAML front matter), with the standard keys typed.

use std::collections::HashMap;

use cooklang::metadata::{
    CooklangValueExt, Metadata as CoreMetadata, NameAndUrl as CoreNameAndUrl,
    RecipeTime as CoreRecipeTime, Servings as CoreServings, StdKey,
};
use cooklang::Converter;

/// The recipe's metadata.
///
/// Standard keys from the Cooklang spec are typed fields; every other key is in
/// `custom`.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Metadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<NameAndUrl>,
    pub source: Option<NameAndUrl>,
    pub time: Option<RecipeTime>,
    pub servings: Option<Servings>,
    pub course: Option<String>,
    pub difficulty: Option<String>,
    pub cuisine: Option<String>,
    pub diet: Vec<String>,
    pub locale: Option<Locale>,
    /// Keys that are not part of the Cooklang standard. Values that are not
    /// plain text (lists, maps) are given as YAML.
    pub custom: HashMap<String, String>,
}

/// A name with an optional URL, used for the author and source.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct NameAndUrl {
    pub name: Option<String>,
    pub url: Option<String>,
}

/// How long the recipe takes, in minutes.
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum RecipeTime {
    Total {
        minutes: i32,
    },
    Composed {
        prep_minutes: Option<i32>,
        cook_minutes: Option<i32>,
    },
}

/// How many servings the recipe makes.
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Servings {
    Number { value: i32 },
    Text { value: String },
}

/// The recipe's locale, e.g. `es` or `en_GB`.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Locale {
    pub language: String,
    pub region: Option<String>,
}

pub(crate) fn convert(metadata: &CoreMetadata, converter: &Converter) -> Metadata {
    let text = |key: StdKey| metadata.get(key).and_then(yaml_to_text);
    let list = |key: StdKey| {
        metadata
            .get(key)
            .and_then(CooklangValueExt::as_tags)
            .map(|items| items.into_iter().map(|s| s.into_owned()).collect())
            .unwrap_or_default()
    };

    Metadata {
        title: metadata.title().map(str::to_string),
        description: metadata.description().map(str::to_string),
        tags: list(StdKey::Tags),
        author: metadata.author().map(name_and_url),
        source: metadata.source().map(name_and_url),
        time: metadata.time(converter).map(|time| match time {
            CoreRecipeTime::Total(minutes) => RecipeTime::Total {
                minutes: crate::int(minutes),
            },
            CoreRecipeTime::Composed {
                prep_time,
                cook_time,
            } => RecipeTime::Composed {
                prep_minutes: prep_time.map(crate::int),
                cook_minutes: cook_time.map(crate::int),
            },
        }),
        servings: metadata.servings().map(|servings| match servings {
            CoreServings::Number(value) => Servings::Number {
                value: crate::int(value),
            },
            CoreServings::Text(value) => Servings::Text { value },
        }),
        course: text(StdKey::Course),
        difficulty: text(StdKey::Difficulty),
        cuisine: text(StdKey::Cuisine),
        diet: list(StdKey::Diet),
        locale: metadata.locale().map(|(language, region)| Locale {
            language: language.to_string(),
            region: region.map(str::to_string),
        }),
        custom: metadata
            .map_filtered()
            .filter_map(|(key, value)| Some((yaml_to_text(key)?, yaml_to_text(value)?)))
            .collect(),
    }
}

fn name_and_url(value: CoreNameAndUrl) -> NameAndUrl {
    NameAndUrl {
        name: value.name().map(str::to_string),
        url: value.url().map(str::to_string),
    }
}

/// Plain text for scalars, YAML for everything else. `None` for null.
fn yaml_to_text(value: &serde_yaml::Value) -> Option<String> {
    use serde_yaml::Value as Yaml;
    match value {
        Yaml::Null => None,
        Yaml::String(s) => Some(s.clone()),
        Yaml::Bool(b) => Some(b.to_string()),
        Yaml::Number(n) => Some(n.to_string()),
        other => serde_yaml::to_string(other)
            .ok()
            .map(|s| s.trim_end().to_string()),
    }
}
