//! Kotlin Multiplatform bindings for cooklang, generated with UniFFI + oxikt.
//!
//! This crate is separate from `bindings/` (upstream's Swift/Android bindings)
//! so it can shape the API for Kotlin without diverging from upstream:
//!
//! - parsing returns plain immutable records instead of native objects that
//!   have to be closed, so Kotlin sees `data class`es with properties;
//! - failures throw `CooklangException` with diagnostics instead of panicking;
//! - counts and indices are `Int`, not `UInt`;
//! - source positions are UTF-16 offsets, matching Kotlin strings.

mod aisle;
mod diagnostics;
mod metadata;
mod quantity;
mod recipe;
mod shopping_list;

pub use aisle::*;
pub use diagnostics::*;
pub use metadata::*;
pub use quantity::*;
pub use recipe::*;
pub use shopping_list::*;

/// Parses a Cooklang recipe, scaling its quantities by `scale`.
///
/// Throws `CooklangException.Parse` if the recipe has errors; warnings are in
/// [`Recipe::warnings`].
///
/// Parses standard Cooklang, plus timers written as durations, like `~{1h20m}` or
/// `~{1-3 seconds}`. Every timer in a time unit has a [`recipe::TimerDuration`].
#[uniffi::export(default(scale = 1.0))]
pub fn parse_recipe(input: String, scale: f64) -> Result<Recipe, CooklangError> {
    let parser = cooklang::CooklangParser::new(
        cooklang::Extensions::TIMER_DURATIONS,
        cooklang::Converter::empty(),
    );
    let (mut parsed, report) = parser
        .parse(&input)
        .into_result()
        .map_err(|report| CooklangError::from_report(&report, &input))?;
    parsed.scale(scale, parser.converter());

    let metadata = metadata::convert(&parsed.metadata, parser.converter());
    let warnings = diagnostics::diagnostics(&report, &input);
    Ok(recipe::convert(&parsed, metadata, warnings))
}

/// Converts a count or index to Kotlin's `Int`, saturating on overflow.
pub(crate) fn int<T: TryInto<i32>>(value: T) -> i32 {
    value.try_into().unwrap_or(i32::MAX)
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_recipe() {
        let recipe = parse_recipe(
            "---\ntitle: Eggs\nservings: 2\nrating: 5\n---\n\
             Crack @eggs{3} into a #bowl{}, whisk for ~{2%min}.\n\n> Use fresh eggs."
                .into(),
            2.0,
        )
        .unwrap();

        assert_eq!(recipe.metadata.title.as_deref(), Some("Eggs"));
        // Scaling also scales the servings.
        assert_eq!(
            recipe.metadata.servings,
            Some(Servings::Number { value: 4 })
        );
        assert_eq!(
            recipe.metadata.custom.get("rating").map(String::as_str),
            Some("5")
        );
        assert_eq!(recipe.ingredients[0].name, "eggs");
        assert_eq!(recipe.ingredients[0].amount.as_ref().unwrap().format(), "6");

        let blocks = &recipe.sections[0].blocks;
        let Block::Step { number, items } = &blocks[0] else {
            panic!("expected a step, got {:?}", blocks[0]);
        };
        assert_eq!(*number, 1);
        assert!(items.contains(&Item::IngredientRef { index: 0 }));
        assert!(items.contains(&Item::CookwareRef { index: 0 }));
        assert!(items.contains(&Item::TimerRef { index: 0 }));
        assert_eq!(
            blocks[1],
            Block::Note {
                text: "Use fresh eggs.".into()
            }
        );
    }

    #[test]
    fn parse_errors_are_returned_not_panicked() {
        let input = "Wait for it.\nThen ~{}.";
        let err = parse_recipe(input.into(), 1.0).unwrap_err();
        let CooklangError::Parse {
            reason,
            diagnostics,
        } = err;
        assert_eq!(reason, "Invalid timer: neither quantity nor name");
        let error = &diagnostics[0];
        assert_eq!(error.severity, Severity::Error);
        let span = error.labels[0].span;
        assert_eq!((span.line, span.column), (2, 7));
        assert_eq!(&input[span.start as usize..span.end as usize], "{}");
    }

    #[test]
    fn warnings_do_not_fail_parsing() {
        let recipe = parse_recipe("---\ntitle: [unclosed\n---\nA step.".into(), 1.0).unwrap();
        assert!(recipe
            .warnings
            .iter()
            .any(|w| w.severity == Severity::Warning && w.message.contains("YAML")));
    }

    #[test]
    fn recipe_reference_path() {
        let recipe = parse_recipe("Add @./sauces/pesto{1%cup}.".into(), 1.0).unwrap();
        let reference = recipe.ingredients[0].reference.clone().unwrap();
        assert_eq!(reference.path("/".into()), "./sauces/pesto");
    }

    #[test]
    fn aisle_lookups_are_case_insensitive() {
        let aisle = AisleConfig::parse("[produce]\ntomatoes|tomato\n".into()).unwrap();
        assert_eq!(
            aisle.category_for("Tomato".into()).as_deref(),
            Some("produce")
        );
        assert_eq!(aisle.common_name_for("TOMATO".into()), "tomatoes");
        assert_eq!(aisle.common_name_for("basil".into()), "basil");
    }

    #[test]
    fn shopping_list_round_trips() {
        let text = "./Pasta{2}\nmilk{1 l}\n";
        let list = parse_shopping_list(text.into()).unwrap();
        assert_eq!(parse_shopping_list(list.to_text()).unwrap(), list);
        assert_eq!(
            CheckEntry::Checked {
                name: "milk".into()
            }
            .to_text(),
            "+ milk\n"
        );
    }

    #[test]
    fn timers_have_durations() {
        let recipe = parse_recipe(
            "Bake for ~{1h20m-1h25m}, rest for ~{10%min}, then ~{30%sec}. Wait ~{a while}.".into(),
            1.0,
        )
        .unwrap();
        // Only the text timer warns.
        let warnings: Vec<_> = recipe.warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(warnings, ["Invalid timer quantity: missing unit"]);
        let durations: Vec<_> = recipe.timers.iter().map(|t| t.duration).collect();
        let d = |min_seconds, max_seconds| {
            Some(recipe::TimerDuration {
                min_seconds,
                max_seconds,
            })
        };
        assert_eq!(
            durations[..3],
            [d(4800.0, 5100.0), d(600.0, 600.0), d(30.0, 30.0)]
        );
        // Text with no unit stays text, with no duration.
        assert_eq!(durations[3], None);
    }
}
