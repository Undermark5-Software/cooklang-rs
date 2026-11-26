#![cfg(feature = "bundled_units")]

use cooklang::{convert::System, Converter, Quantity, Value};
use test_case::test_case;

#[test_case(2.0, "tsp" => "2 tsp")]
#[test_case(3.0, "tsp" => "1 tbsp")]
#[test_case(3.5, "tsp" => "3 1/2 tsp")]
#[test_case(15.0, "tsp" => "5 tbsp")]
#[test_case(16.0, "tsp" => "1/3 c")]
#[test_case(180.0, "C" => "356 °F")]
// Differences too small to measure are rounded away; real ones never are (see `fit`).
#[test_case(499.999, "lb" => "500 lb")]
#[test_case(1.5, "F" => "1.5 °F")]
fn imperial(value: f64, unit: &str) -> String {
    let converter = Converter::bundled();
    let mut q = Quantity::new(Value::from(value), Some(unit.to_string()));
    let _ = q.convert(System::Imperial, &converter);
    q.to_string()
}

// Fitting picks the largest unit that gives an amount a cook would write.
#[test_case(1.0, "c", 0.25 => "1/4 c"; "quarter cup stays a cup")]
#[test_case(4.0, "tbsp", 1.0 => "1/4 c"; "4 tbsp is a quarter cup")]
#[test_case(8.0, "tbsp", 1.0 => "1/2 c")]
#[test_case(12.0, "tbsp", 1.0 => "3/4 c")]
#[test_case(1.0, "c", 1.0 / 3.0 => "1/3 c")]
#[test_case(1.0, "c", 2.0 / 3.0 => "2/3 c")]
#[test_case(2.0, "tbsp", 1.0 => "1/8 c"; "cups go down to an eighth")]
#[test_case(1.0, "c", 1.0 / 6.0 => "1/6 c"; "a third cup halved")]
#[test_case(6.0, "tbsp", 1.0 => "3/8 c")]
#[test_case(1.0, "c", 1.125 => "1 1/8 c")]
#[test_case(1.0, "tbsp", 1.0 => "1 tbsp"; "a sixteenth cup isn't a cup fraction")]
#[test_case(1.0, "c", 0.2 => "3.2 tbsp"; "fifths of a cup aren't allowed")]
#[test_case(1.0, "tsp", 1.0 / 3.0 => "1/3 tsp"; "thirds of a teaspoon are allowed")]
#[test_case(5.0, "tbsp", 1.0 => "5 tbsp")]
#[test_case(3.0, "tsp", 1.0 => "1 tbsp")]
#[test_case(1.0, "tsp", 1.0 => "1 tsp"; "a third tablespoon stays in teaspoons")]
#[test_case(1.5, "tsp", 1.0 => "1 1/2 tsp")]
#[test_case(4.0, "tsp", 1.0 => "4 tsp")]
#[test_case(1.5, "tbsp", 1.0 => "1 1/2 tbsp")]
#[test_case(4.0, "oz", 1.0 => "4 oz"; "a quarter pound stays in ounces")]
#[test_case(24.0, "oz", 1.0 => "1 1/2 lb")]
#[test_case(6.0, "in", 1.0 => "6 in")]
#[test_case(5.25, "c", 1.0 => "5 1/4 c"; "a quarter cup isn't rounded away")]
#[test_case(13.5, "tbsp", 1.0 => "13 1/2 tbsp")]
#[test_case(45.0, "oz", 1.0 => "45 oz"; "2 13/16 lb isn't a pound fraction")]
fn fit(value: f64, unit: &str, scale: f64) -> String {
    let converter = Converter::bundled();
    let mut q = Quantity::new(Value::from(value * scale), Some(unit.to_string()));
    let _ = q.fit(&converter);
    q.to_string()
}

#[test_case(4.0, 6.0, "tbsp" => "1/4-3/8 c"; "both ends are cup fractions")]
#[test_case(4.0, 5.0, "tbsp" => "4-5 tbsp"; "5/16 cup isn't, so the range stays in tablespoons")]
#[test_case(4.0, 8.0, "tbsp" => "1/4-1/2 c")]
fn fit_range(start: f64, end: f64, unit: &str) -> String {
    let converter = Converter::bundled();
    let mut q = Quantity::new(
        Value::Range { start: start.into(), end: end.into() },
        Some(unit.to_string()),
    );
    let _ = q.fit(&converter);
    q.to_string()
}

fn written(value: f64, unit: &str) -> String {
    let converter = Converter::bundled();
    let q = Quantity::new(Value::from(value), Some(unit.to_string()));
    q.written(&converter, false).map(|w| w.to_string()).unwrap_or_else(|| "none".into())
}

fn written_liquid(value: f64, unit: &str) -> String {
    let converter = Converter::bundled();
    let mut q = Quantity::new(Value::from(value), Some(unit.to_string()));
    let _ = q.fit(&converter);
    q.written(&converter, true).map(|w| w.to_string()).unwrap_or_else(|| "none".into())
}

fn fitted_then_written(value: f64, unit: &str) -> String {
    let converter = Converter::bundled();
    let mut q = Quantity::new(Value::from(value), Some(unit.to_string()));
    let _ = q.fit(&converter);
    q.written(&converter, false).map(|w| w.to_string()).unwrap_or_else(|| q.to_string())
}

// Liquids get quarts from 1 quart up, gallons in quarters from 1 gallon; below, cups.
#[test_case(8.0, "c" => "2 qt")]
#[test_case(6.0, "c" => "1 1/2 qt")]
#[test_case(4.5, "c" => "4 1/2 c"; "1 1/8 qt isn't a quart fraction")]
#[test_case(3.0, "c" => "3 c")]
#[test_case(12.0, "qt" => "3 gal")]
#[test_case(0.75, "qt" => "3 c"; "under a quart, cups")]
#[test_case(24.0, "c" => "1 1/2 gal")]
#[test_case(16.0, "c" => "1 gal")]
#[test_case(18.0, "c" => "4 1/2 qt"; "1 1/8 gal isn't a quarter gallon")]
#[test_case(5.0, "gal" => "5 gal")]
fn liquids(value: f64, unit: &str) -> String {
    written_liquid(value, unit)
}

// A recipe's own pints and quarts stay while there is at least one.
#[test_case(12.0, "qt" => "12 qt"; "3 quarts of stock times 4")]
#[test_case(0.75, "qt" => "3 c"; "3 quarts times a quarter")]
#[test_case(2.0, "pt" => "2 pt")]
#[test_case(0.5, "pt" => "1 c"; "half a pint of berries")]
#[test_case(8.0, "c" => "8 c"; "not a liquid, so cups stay cups")]
#[test_case(24.0, "c" => "24 c"; "and never gallons")]
#[test_case(8.0, "fl oz" => "1 c"; "fluid ounces that are cup quarters or thirds become cups")]
#[test_case(6.0, "fl oz" => "3/4 c")]
#[test_case(12.0, "fl oz" => "1 1/2 c")]
#[test_case(2.0, "fl oz" => "1/4 c")]
#[test_case(1.0, "fl oz" => "1 fl oz"; "an eighth cup stays in fluid ounces")]
#[test_case(5.0, "fl oz" => "5 fl oz")]
fn recipe_units(value: f64, unit: &str) -> String {
    fitted_then_written(value, unit)
}

// Weights: pounds when a pound fraction writes it, otherwise whole pounds and ounces.
#[test_case(45.0, "oz" => "2 lb 13 oz")]
#[test_case(27.0, "oz" => "1 lb 11 oz")]
#[test_case(17.5, "oz" => "1 lb 1 1/2 oz")]
#[test_case(2.8125, "lb" => "2 lb 13 oz"; "from pounds too")]
#[test_case(24.0, "oz" => "1 1/2 lb")]
#[test_case(48.0, "oz" => "3 lb")]
#[test_case(12.0, "oz" => "12 oz"; "under a pound")]
#[test_case(1000.0, "g" => "none"; "metric is left alone")]
fn weights(value: f64, unit: &str) -> String {
    written(value, unit)
}

// Volumes: one unit if exact, else two parts; tiny teaspoon amounts.
#[test_case(4.0, "tbsp" => "1/4 c")]
#[test_case(5.0, "tbsp" => "5 tbsp"; "up to 7 tbsp stays tablespoons")]
#[test_case(4.0, "tsp" => "1 tbsp + 1 tsp"; "a tablespoon or more is tablespoons")]
#[test_case(1.0 / 3.0, "tsp" => "1/3 tsp"; "thirds of a teaspoon are allowed")]
#[test_case(4.0 / 3.0, "tsp" => "1 1/3 tsp")]
#[test_case(3.0 + 1.0 / 3.0, "tsp" => "3 1/3 tsp")]
#[test_case(6.0 + 1.0 / 3.0, "tsp" => "2.11 tbsp"; "never a third next to another unit: not 2 tbsp + 1/3 tsp")]
#[test_case(2.25, "tbsp" => "2 tbsp + 3/4 tsp"; "spoons under a quarter cup")]
#[test_case(10.0, "tsp" => "3 tbsp + 1 tsp")]
#[test_case(1.75, "tbsp" => "1 tbsp + 2 1/4 tsp")]
#[test_case(17.0, "tbsp" => "1 c + 1 tbsp"; "a cup and the rest")]
#[test_case(12.0 + 2.0 / 3.0, "tbsp" => "3/4 c + 2 tsp")]
#[test_case(25.0 + 1.0 / 3.0, "tbsp" => "1 1/2 c + 4 tsp"; "the rest can be one spoon amount")]
#[test_case(3.0 / 16.0, "tsp" => "1/8 tsp"; "sixteenths drop to the eighth below")]
#[test_case(5.0 / 16.0, "tsp" => "1/4 tsp")]
#[test_case(1.0 / 16.0, "tsp" => "a pinch")]
#[test_case(1.0 / 32.0, "tsp" => "a smidgen")]
#[test_case(1.0 / 24.0, "tsp" => "0.04 tsp"; "nothing writes it exactly")]
#[test_case(0.56, "c" => "0.56 c"; "two parts can't write it, so a decimal in cups")]
#[test_case(2.94, "tbsp" => "2.94 tbsp"; "a decimal is never shown as a near fraction")]
#[test_case(5.69, "c" => "5.69 c")]
#[test_case(0.5 + 1.0 / 3.0 / 16.0, "c" => "1/2 c + 1 tsp"; "two parts beat a decimal")]
#[test_case(6.0, "quart" => "6 qt"; "quarts stay quarts")]
#[test_case(2.0, "tablespoons" => "2 tbsp"; "spelled out units are written too")]
#[test_case(1.0 / 6.0, "c" => "2 tbsp + 2 tsp"; "under a quarter cup is spoons")]
#[test_case(1.0 / 8.0, "c" => "2 tbsp")]
#[test_case(3.0 / 8.0, "c" => "6 tbsp"; "eighths of a cup up to 7 tbsp are tablespoons")]
#[test_case(7.0, "tbsp" => "7 tbsp")]
#[test_case(1.0 / 3.0, "c" => "1/3 c"; "a third of a cup stays cups")]
#[test_case(5.0 / 8.0, "c" => "5/8 c"; "past 7 tbsp, eighths are cups")]
#[test_case(1.0 + 1.0 / 8.0, "c" => "1 1/8 c")]
#[test_case(9.0, "tbsp" => "1/2 c + 1 tbsp"; "cups past 7 tbsp")]
fn volumes(value: f64, unit: &str) -> String {
    written(value, unit)
}

fn written_range(start: f64, end: f64, unit: &str) -> String {
    let converter = Converter::bundled();
    let q = Quantity::new(Value::Range { start: start.into(), end: end.into() }, Some(unit.to_string()));
    q.written(&converter, false).map(|w| w.to_string()).unwrap_or_else(|| "none".into())
}

// Ranges take the unit for the start's size, or a smaller one that writes both ends.
#[test_case(2.0, 4.0, "tbsp" => "2-4 tbsp"; "1-2 tbsp doubled")]
#[test_case(3.0, 6.0, "tbsp" => "3-6 tbsp")]
#[test_case(4.0, 8.0, "tbsp" => "1/4-1/2 c")]
#[test_case(4.0, 5.0, "tbsp" => "4-5 tbsp"; "5/16 c isn't a cup amount")]
#[test_case(0.5, 1.0, "tsp" => "1/2-1 tsp")]
fn ranges(start: f64, end: f64, unit: &str) -> String {
    written_range(start, end, unit)
}
