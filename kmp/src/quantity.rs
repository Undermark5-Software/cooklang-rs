//! Quantities, their formatting, and combining ingredient amounts.

use std::collections::HashMap;
use std::mem::discriminant;

use cooklang::quantity::{Quantity as CoreQuantity, Value as CoreValue};

use crate::recipe::Ingredient;

/// A quantity with an optional unit, like `2 cups` or `a pinch`.
///
/// `quantity` and `unit` are the exact amount, for adding amounts up. [`Amount::format`]
/// is how a cook would write it, which can differ: 3/16 tsp is written `1/8 tsp`.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Amount {
    pub quantity: Value,
    pub unit: Option<String>,
    /// The amount written in more than one unit or by name, like `2 lb` and `13 oz`,
    /// `2 tbsp` and `3/4 tsp`, or `a pinch`. Empty when one plain amount writes it.
    pub parts: Vec<AmountPart>,
    /// The written form, when it was worked out from the parsed recipe. Amounts added
    /// up outside a recipe have none and are formatted from `quantity`.
    pub display: Option<String>,
}

/// One part of an [`Amount`] as written.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct AmountPart {
    /// The value as written, like `0.75` for `3/4 tsp`.
    pub value: f64,
    pub unit: String,
    /// Ready to show: `3/4 tsp`, or a name like `a pinch`.
    pub text: String,
}

#[uniffi::export]
impl Amount {
    /// The amount as a cook would write it, e.g. `1 1/2 tsp`, `2 tbsp + 3/4 tsp`,
    /// `2 lb 13 oz` or `a pinch`.
    pub fn format(&self) -> String {
        if let Some(display) = &self.display {
            return display.clone();
        }
        format_value(&self.quantity, self.unit.as_deref())
    }
}

fn format_value(value: &Value, unit: Option<&str>) -> String {
    match (value.format(), unit) {
        (Some(quantity), Some(unit)) => format!("{quantity} {unit}"),
        (Some(quantity), None) => quantity,
        (None, unit) => unit.unwrap_or_default().to_string(),
    }
}

impl Amount {
    /// The amount of a parsed quantity, with its written form (see
    /// [`cooklang::quantity::Quantity::written`]).
    pub(crate) fn new(quantity: &CoreQuantity, converter: &cooklang::Converter, liquid: bool) -> Self {
        let exact = Amount::from(quantity);
        let Some(written) = quantity.written(converter, liquid) else {
            // Shown as fitted: the fraction as written, or a decimal if none fits. Known
            // units use their symbol (`3 fl oz`, not `3 fluid ounce`); counts like
            // `cloves` keep the recipe's wording, which follows the amount.
            let unit = quantity
                .unit_info(converter)
                .filter(|u| u.physical_quantity != cooklang::convert::PhysicalQuantity::Count)
                .map(|u| u.symbol().to_string())
                .or_else(|| exact.unit.clone());
            let display = format_value(&written_value(quantity.value()), unit.as_deref());
            return Amount { display: Some(display), ..exact };
        };
        let parts: Vec<AmountPart> = written
            .parts
            .iter()
            .map(|part| AmountPart {
                value: part.value,
                unit: part.unit.symbol().to_string(),
                text: match (part.name, part.end) {
                    (Some(name), _) => name.to_string(),
                    (None, Some(end)) => format!(
                        "{} - {} {}",
                        format_number(part.value),
                        format_number(end),
                        part.unit.symbol()
                    ),
                    (None, None) => format!("{} {}", format_number(part.value), part.unit.symbol()),
                },
            })
            .collect();
        let display = parts.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join(written.separator);
        let named = written.parts.iter().any(|p| p.name.is_some());
        Amount {
            parts: if parts.len() > 1 || named { parts } else { Vec::new() },
            display: Some(display),
            ..exact
        }
    }
}

impl From<&CoreQuantity> for Amount {
    fn from(quantity: &CoreQuantity) -> Self {
        Amount {
            quantity: quantity.value().into(),
            unit: quantity.unit().map(|u| u.to_string()),
            parts: Vec::new(),
            display: None,
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

/// The exact value, including any error from fitting it to a fraction.
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

/// The value as written: fractions without their fitting error.
fn written_value(value: &CoreValue) -> Value {
    match value {
        CoreValue::Number(n) => Value::Number { value: written(*n) },
        CoreValue::Range { start, end } => Value::Range {
            start: written(*start),
            end: written(*end),
        },
        CoreValue::Text(t) => Value::Text {
            value: t.to_string(),
        },
    }
}

/// The number as written: a fraction that unit fitting rounded, like `2 7/8` for
/// 2.8125 lb, without the rounding error, so it formats as the fraction.
fn written(number: cooklang::quantity::Number) -> f64 {
    match number {
        cooklang::quantity::Number::Regular(value) => value,
        cooklang::quantity::Number::Fraction { whole, num, den, .. } => {
            whole as f64 + num as f64 / den as f64
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
            parts: Vec::new(),
            display: None,
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
    // The total is a new amount, so the old written form no longer applies.
    existing.parts.clear();
    existing.display = None;
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
    const FRACTIONS: [(f64, &str); 11] = [
        (0.125, "1/8"),
        (1.0 / 6.0, "1/6"),
        (0.25, "1/4"),
        (1.0 / 3.0, "1/3"),
        (0.375, "3/8"),
        (0.5, "1/2"),
        (0.625, "5/8"),
        (2.0 / 3.0, "2/3"),
        (0.75, "3/4"),
        (5.0 / 6.0, "5/6"),
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
            parts: Vec::new(),
            display: None,
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
    fn keeps_the_exact_amount_and_writes_it_for_a_cook() {
        let converter = cooklang::Converter::bundled();
        let amount = |value: f64, unit: &str| {
            Amount::new(&CoreQuantity::new(CoreValue::from(value), Some(unit.into())), &converter, false)
        };
        let salt = amount(3.0 / 16.0, "tsp");
        assert_eq!(salt.format(), "1/8 tsp");
        assert_eq!(salt.quantity, Value::Number { value: 3.0 / 16.0 });

        let beef = amount(45.0, "oz");
        assert_eq!(beef.format(), "2 lb 13 oz");
        assert_eq!(beef.quantity, Value::Number { value: 45.0 });
        assert_eq!(beef.parts.len(), 2);
        assert_eq!(beef.parts[1].text, "13 oz");

        let oil = amount(2.25, "tbsp");
        assert_eq!(oil.format(), "2 tbsp + 3/4 tsp");
        assert_eq!(amount(1.0 / 16.0, "tsp").parts[0].text, "a pinch");
        assert_eq!(amount(1.0 / 6.0, "c").format(), "2 tbsp + 2 tsp");
        assert!(amount(1.5, "tbsp").parts.is_empty());
        assert_eq!(amount(3.0, "fluid ounce").format(), "3 fl oz");
    }

    #[test]
    fn totals_drop_the_written_form() {
        let mut combiner = Combiner::default();
        let written = Amount { display: Some("1/8 tsp".into()), ..number(3.0 / 16.0, Some("tsp")) };
        combiner.add("salt".into(), [written.clone(), written]);
        let total = &combiner.finish()[0].amounts[0];
        assert_eq!(total.quantity, Value::Number { value: 3.0 / 8.0 });
        assert_eq!(total.format(), "3/8 tsp");
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
