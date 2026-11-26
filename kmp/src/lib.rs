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
mod parser;
mod quantity;
mod recipe;
mod shopping_list;

pub use aisle::*;
pub use diagnostics::*;
pub use metadata::*;
pub use parser::*;
pub use quantity::*;
pub use recipe::*;
pub use shopping_list::*;

/// Parses a Cooklang recipe, scaling its quantities by `scale`.
///
/// Throws `CooklangException.Parse` if the recipe has errors; warnings are in
/// [`Recipe::warnings`].
///
/// Parses with every cooklang-rs extension and the bundled units, so ranges, modifiers,
/// intermediate preparations and timers written as durations (`~{1h20m}`) all work.
/// Scaling fits amounts to the best unit, e.g. `3 tsp` becomes `1 tbsp`. Every timer in
/// a time unit has a [`recipe::TimerDuration`]. Uses the default settings; see
/// [`RecipeParser`] to change them.
#[uniffi::export(default(scale = 1.0))]
pub fn parse_recipe(input: String, scale: f64) -> Result<Recipe, CooklangError> {
    RecipeParser::new(None)?.parse(input, scale)
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
        } = err
        else {
            panic!("expected a parse error");
        };
        // Timers must have a time.
        assert_eq!(reason, "Invalid timer: missing quantity");
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
    fn alternative_amounts() {
        let input = "Add @plain flour{125%g}{1/2%cup}(sifted) and @salt{1%tsp}.";
        let written = |recipe: &Recipe| -> Vec<Vec<String>> {
            let amounts = |i: &Ingredient| i.alternative_amounts.iter().map(|a| a.format()).collect();
            recipe.ingredients.iter().map(amounts).collect()
        };

        let recipe = parse_recipe(input.into(), 1.0).unwrap();
        assert!(recipe.warnings.is_empty(), "{:?}", recipe.warnings);
        assert_eq!(recipe.ingredients[0].amount.as_ref().unwrap().format(), "125 g");
        assert_eq!(recipe.ingredients[0].note.as_deref(), Some("sifted"));
        assert_eq!(written(&recipe), [vec!["1/2 cup".to_string()], vec![]]);

        // Alternatives scale with the main amount, and are written the same way
        // a main amount in their unit is.
        let recipe = parse_recipe(input.into(), 2.0).unwrap();
        assert_eq!(recipe.ingredients[0].amount.as_ref().unwrap().format(), "250 g");
        let main = parse_recipe("Add @flour{1/2%cup}.".into(), 2.0).unwrap();
        let main = main.ingredients[0].amount.as_ref().unwrap().format();
        assert_eq!(written(&recipe), [vec![main], vec![]]);
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
            "Bake for ~{1h20m-1h25m}, rest for ~{10%min}, then ~{30%sec}, ~{2%hrs}, ~{5%m}, ~{1h 5m 3s}.".into(),
            1.0,
        )
        .unwrap();
        assert!(recipe.warnings.is_empty(), "{:?}", recipe.warnings);
        let durations: Vec<_> = recipe.timers.iter().map(|t| t.duration).collect();
        let d = |start_seconds, end_seconds| {
            Some(recipe::TimerDuration {
                start_seconds,
                end_seconds,
            })
        };
        assert_eq!(
            durations,
            [
                d(4800.0, 5100.0),
                d(600.0, 600.0),
                d(30.0, 30.0),
                d(7200.0, 7200.0),
                d(300.0, 300.0),
                d(3903.0, 3903.0)
            ]
        );
    }

    #[test]
    fn timers_that_arent_times_are_errors() {
        for (input, expected) in [
            ("Wait ~{a while}.", "Timer value is text: a while"),
            ("Wait ~{2%cups}.", "Timer unit is not time: c"),
            ("Wait ~{1%week}.", "Unknown timer unit: week"),
        ] {
            let CooklangError::Parse { reason, .. } = parse_recipe(input.into(), 1.0).unwrap_err() else {
                panic!("expected a parse error");
            };
            assert_eq!(reason, expected, "{input}");
        }
    }

    #[test]
    fn ranges_scale_and_units_fit() {
        let recipe =
            parse_recipe("Add @salt{1-2%tsp} and @sugar{3%teaspoons}.".into(), 2.0).unwrap();
        let amounts: Vec<_> = recipe
            .ingredients
            .iter()
            .map(|i| i.amount.as_ref().unwrap().format())
            .collect();
        assert_eq!(amounts, ["2 - 4 tsp", "2 tbsp"]);
    }

    #[test]
    fn relations_and_the_ingredient_list() {
        let recipe = parse_recipe(
            "Mix @flour{200%g} and @-salt{1%tsp}.\n\n\
             Add @&flour{50%g} and @&(~1)dough{}."
                .into(),
            1.0,
        )
        .unwrap();
        assert!(recipe.warnings.is_empty(), "{:?}", recipe.warnings);
        use recipe::IngredientRelation as R;
        let relations: Vec<_> = recipe
            .ingredients
            .iter()
            .map(|i| i.relation.clone())
            .collect();
        assert_eq!(
            relations,
            [
                R::Definition {
                    referenced_from: vec![2]
                },
                R::Definition {
                    referenced_from: vec![]
                },
                R::Ingredient { index: 0 },
                R::Step {
                    section: 0,
                    block: 0
                },
            ]
        );
        assert!(recipe.ingredients[1].hidden);
        // Flour once, with both amounts; salt is hidden and dough is what step 1 made.
        let list: Vec<_> = recipe
            .ingredient_list
            .iter()
            .map(|l| {
                let amounts: Vec<_> = l.amounts.iter().map(|a| a.format()).collect();
                (recipe.ingredients[l.index as usize].name.clone(), amounts)
            })
            .collect();
        assert_eq!(list, [("flour".to_string(), vec!["250 g".to_string()])]);
    }

    #[test]
    fn backwards_timer_warns_and_keeps_its_order() {
        let recipe = parse_recipe("Bake for ~{1h25m-1h20m}.".into(), 1.0).unwrap();
        let warnings: Vec<_> = recipe.warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(warnings, ["Range is backwards"]);
        let duration = recipe.timers[0].duration.unwrap();
        assert_eq!(
            (duration.start_seconds, duration.end_seconds),
            (5100.0, 4800.0)
        );
    }
}
