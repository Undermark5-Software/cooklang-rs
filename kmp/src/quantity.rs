//! Quantities, their formatting, and combining ingredient amounts.

use std::collections::HashMap;
use std::mem::discriminant;

use cooklang::quantity::{Quantity as CoreQuantity, Value as CoreValue};

use crate::recipe::Ingredient;

/// A quantity with an optional unit, like `2 cups` or `a pinch`.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Amount {
    pub quantity: Value,
    pub unit: Option<String>,
}

#[uniffi::export]
impl Amount {
    /// Formats the amount for display, e.g. `1 1/2 tsp` or `2/3 cups`.
    pub fn format(&self) -> String {
        match (self.quantity.format(), &self.unit) {
            (Some(quantity), Some(unit)) => format!("{quantity} {unit}"),
            (Some(quantity), None) => quantity,
            (None, unit) => unit.clone().unwrap_or_default(),
        }
    }
}

impl From<&CoreQuantity> for Amount {
    fn from(quantity: &CoreQuantity) -> Self {
        Amount {
            quantity: quantity.value().into(),
            unit: quantity.unit().map(|u| u.to_string()),
        }
    }
}

/// The value part of an [`Amount`].
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Value {
    Number {
        value: f64,
    },
    Range {
        start: f64,
        end: f64,
    },
    Text {
        value: String,
    },
    /// No value was given, e.g. `@salt{}` or an ingredient without an amount.
    Empty,
}

#[uniffi::export]
impl Value {
    /// Formats the value for display, turning common decimals into fractions
    /// (`0.5` → `1/2`) and hiding float noise from scaling (`0.8999999` →
    /// `0.9`). Returns `null` for [`Value::Empty`].
    pub fn format(&self) -> Option<String> {
        match self {
            Value::Number { value } => Some(format_number(*value)),
            Value::Range { start, end } => Some(format!(
                "{} - {}",
                format_number(*start),
                format_number(*end)
            )),
            Value::Text { value } => Some(value.clone()),
            Value::Empty => None,
        }
    }
}

impl From<&CoreValue> for Value {
    fn from(value: &CoreValue) -> Self {
        match value {
            CoreValue::Number(n) => Value::Number { value: n.value() },
            CoreValue::Range { start, end } => Value::Range {
                start: start.value(),
                end: end.value(),
            },
            CoreValue::Text(t) => Value::Text {
                value: t.to_string(),
            },
        }
    }
}

/// Parses text into a [`Value`].
///
/// Understands numbers (`1.5`), fractions (`1/2`), mixed numbers (`1 1/2`) and
/// ranges (`1/2 - 3/4`). Anything else becomes [`Value::Text`].
#[uniffi::export]
pub fn parse_value(text: String) -> Value {
    if let Some(value) = parse_number_or_fraction(text.trim()) {
        return Value::Number { value };
    }
    if let Some((start, end)) = text.split_once(" - ") {
        if let (Some(start), Some(end)) = (
            parse_number_or_fraction(start.trim()),
            parse_number_or_fraction(end.trim()),
        ) {
            return Value::Range { start, end };
        }
    }
    Value::Text { value: text }
}

/// All the amounts of one ingredient across a recipe (or several), combined.
///
/// Amounts that share a unit and kind of value are added together; the rest
/// are kept side by side, e.g. `[2 cups, 1 tbsp]`. An ingredient used without
/// an amount contributes an [`Value::Empty`] amount.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct CombinedIngredient {
    pub name: String,
    pub amounts: Vec<Amount>,
}

/// Combines ingredients with the same name, adding up their amounts.
///
/// The result keeps the order in which each name first appears.
#[uniffi::export]
pub fn combine_ingredients(ingredients: Vec<Ingredient>) -> Vec<CombinedIngredient> {
    let mut combiner = Combiner::default();
    for ingredient in ingredients {
        let amount = ingredient.amount.unwrap_or(Amount {
            quantity: Value::Empty,
            unit: None,
        });
        combiner.add(ingredient.name, std::iter::once(amount));
    }
    combiner.finish()
}

/// Groups amounts by name, preserving first-seen order.
#[derive(Default)]
pub(crate) struct Combiner {
    entries: Vec<CombinedIngredient>,
    positions: HashMap<String, usize>,
}

impl Combiner {
    pub(crate) fn add(&mut self, name: String, amounts: impl IntoIterator<Item = Amount>) {
        let position = *self.positions.entry(name.clone()).or_insert_with(|| {
            self.entries.push(CombinedIngredient {
                name,
                amounts: Vec::new(),
            });
            self.entries.len() - 1
        });
        let existing = &mut self.entries[position].amounts;
        for amount in amounts {
            merge_amount(existing, amount);
        }
    }

    pub(crate) fn finish(self) -> Vec<CombinedIngredient> {
        self.entries
    }
}

fn merge_amount(amounts: &mut Vec<Amount>, amount: Amount) {
    let same_kind = amounts.iter_mut().find(|a| {
        a.unit == amount.unit && discriminant(&a.quantity) == discriminant(&amount.quantity)
    });
    let Some(existing) = same_kind else {
        amounts.push(amount);
        return;
    };
    match (&mut existing.quantity, amount.quantity) {
        (Value::Number { value: total }, Value::Number { value }) => *total += value,
        (Value::Range { start: s, end: e }, Value::Range { start, end }) => {
            *s += start;
            *e += end;
        }
        (Value::Text { value: total }, Value::Text { value }) if *total != value => {
            total.push_str(", ");
            total.push_str(&value);
        }
        // Identical text is kept once; Empty + Empty is still "some".
        _ => {}
    }
}

fn format_number(value: f64) -> String {
    // Hide floating point noise, e.g. 0.89999999999 -> 0.9.
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    if rounded.fract().abs() < 0.000_000_1 {
        return format!("{rounded:.0}");
    }
    if let Some(fraction) = decimal_to_fraction(rounded) {
        return fraction;
    }
    let rounded = (rounded * 1000.0).round() / 1000.0;
    let text = format!("{rounded:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn decimal_to_fraction(value: f64) -> Option<String> {
    const EPSILON: f64 = 0.0001;
    const FRACTIONS: [(f64, &str); 9] = [
        (0.125, "1/8"),
        (0.25, "1/4"),
        (1.0 / 3.0, "1/3"),
        (0.375, "3/8"),
        (0.5, "1/2"),
        (0.625, "5/8"),
        (2.0 / 3.0, "2/3"),
        (0.75, "3/4"),
        (0.875, "7/8"),
    ];
    let whole = value.floor();
    let fract = value - whole;
    FRACTIONS
        .iter()
        .find(|(decimal, _)| (fract - decimal).abs() < EPSILON)
        .map(|(_, fraction)| {
            if whole > 0.0 {
                format!("{whole:.0} {fraction}")
            } else {
                fraction.to_string()
            }
        })
}

fn parse_number_or_fraction(s: &str) -> Option<f64> {
    if let Ok(num) = s.parse::<f64>() {
        return Some(num);
    }
    if let Some((whole, fract)) = s.split_once(' ') {
        if let (Ok(whole), Some(fract)) = (whole.parse::<f64>(), parse_fraction(fract)) {
            return Some(whole + fract);
        }
    }
    parse_fraction(s)
}

fn parse_fraction(s: &str) -> Option<f64> {
    let (numerator, denominator) = s.split_once('/')?;
    let numerator = numerator.trim().parse::<f64>().ok()?;
    let denominator = denominator.trim().parse::<f64>().ok()?;
    (denominator != 0.0).then(|| numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: f64, unit: Option<&str>) -> Amount {
        Amount {
            quantity: Value::Number { value },
            unit: unit.map(str::to_string),
        }
    }

    #[test]
    fn formats_fractions_and_noise() {
        assert_eq!(number(1.5, Some("tsp")).format(), "1 1/2 tsp");
        assert_eq!(number(2.0 / 3.0, Some("cups")).format(), "2/3 cups");
        assert_eq!(
            Value::Number {
                value: 0.8999999999
            }
            .format()
            .unwrap(),
            "0.9"
        );
        assert_eq!(Value::Number { value: 1.234 }.format().unwrap(), "1.234");
        assert_eq!(Value::Empty.format(), None);
    }

    #[test]
    fn parses_values() {
        assert_eq!(parse_value("1 1/2".into()), Value::Number { value: 1.5 });
        assert_eq!(
            parse_value("1/2 - 3/4".into()),
            Value::Range {
                start: 0.5,
                end: 0.75
            }
        );
        assert_eq!(
            parse_value("a pinch".into()),
            Value::Text {
                value: "a pinch".into()
            }
        );
    }

    #[test]
    fn merges_matching_units_and_keeps_order() {
        let mut combiner = Combiner::default();
        combiner.add("flour".into(), [number(1.0, Some("cup"))]);
        combiner.add("salt".into(), [number(1.0, None)]);
        combiner.add(
            "flour".into(),
            [number(2.0, Some("cup")), number(1.0, Some("tbsp"))],
        );
        let combined = combiner.finish();
        assert_eq!(combined[0].name, "flour");
        assert_eq!(
            combined[0].amounts,
            vec![number(3.0, Some("cup")), number(1.0, Some("tbsp"))]
        );
        assert_eq!(combined[1].name, "salt");
    }
}
