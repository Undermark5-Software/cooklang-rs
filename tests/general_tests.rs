use cooklang::{Content, CooklangParser, Extensions, Item, Value};
use indoc::indoc;
use test_case::test_case;

#[test_case(
    indoc! {r#"
        first

        second
    "#} => vec![vec![Some(1), Some(2)]]; "basic"
)]
#[test_case(
    indoc! {r#"
        > text

        first

        second
    "#} => vec![vec![None,  Some(1), Some(2)]]; "text start"
)]
#[test_case(
    indoc! {r#"
        first

        > text

        second
    "#} => vec![vec![Some(1), None, Some(2)]]; "text middle"
)]
#[test_case(
    indoc! {r#"
        first

        second
        == sect ==
        first again
    "#} => vec![vec![Some(1), Some(2)], vec![Some(1)]]; "section reset"
)]
#[test_case(
    indoc! {r#"
        > text

        first

        second
        == sect ==
        first again
    "#} => vec![vec![None, Some(1), Some(2)], vec![Some(1)]]; "complex 1"
)]
#[test_case(
    indoc! {r#"
        first

        > text

        second
        == sect ==
        first again
    "#} => vec![vec![Some(1), None, Some(2)], vec![Some(1)]]; "complex 2"
)]
#[test_case(
    indoc! {r#"
        first

        second
        == sect ==
        > text

        first again
    "#} => vec![vec![Some(1), Some(2)], vec![None, Some(1)]]; "complex 3"
)]
#[test_case(
    indoc! {r#"
        first

        second
        == sect ==
        first again

        > text
    "#} => vec![vec![Some(1), Some(2)], vec![Some(1), None]]; "complex 4"
)]
#[test_case(
    indoc! {r#"
        > just text

        == sect ==

        > text

        first again
    "#} => vec![vec![None], vec![None, Some(1)]]; "complex 5"
)]
fn step_number(src: &str) -> Vec<Vec<Option<u32>>> {
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(src).unwrap_output();
    let numbers: Vec<Vec<Option<u32>>> = r
        .sections
        .into_iter()
        .map(|sect| {
            sect.content
                .into_iter()
                .map(|c| match c {
                    Content::Step(s) => Some(s.number),
                    Content::Text(_) => None,
                })
                .collect()
        })
        .collect();
    numbers
}

#[test]
fn empty_not_empty() {
    let input = indoc! {r#"
        -- "empty" recipe

           -- with spaces

        -- that should actually be empty 
            -- and not produce empty steps   
        
        [- not even this -]
    "#};

    // should be the same with multiline and without
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert!(r.sections.is_empty());

    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert!(r.sections.is_empty());
}

#[test]
fn empty_steps() {
    let input = indoc! {r#"
        == Section name to force the section ==

        -- "empty" recipe

           -- with spaces

        -- that should actually be empty 
            -- and not produce empty steps   
        
        [- not even this -]
    "#};

    // should be the same with multiline and without
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert!(r.sections[0].content.is_empty());

    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert!(r.sections[0].content.is_empty());
}

#[test]
fn whitespace_line_block_separator() {
    let input = indoc! {r#"
        a step
                 
        another
    "#};

    // should be the same with multiline and without
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert_eq!(r.sections[0].content.len(), 2);
}

#[test]
fn single_line_no_separator() {
    let input = indoc! {r#"
        a step
        >> meta: val
        another step
        = section
    "#};
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert_eq!(r.sections.len(), 2);
    assert_eq!(r.sections[0].content.len(), 2);
    assert_eq!(r.sections[1].content.len(), 0);
    assert_eq!(r.metadata.map.len(), 1);
}

#[test]
fn multiple_temperatures() {
    let input = "text 2ºC more text 150 F end text";
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert_eq!(r.inline_quantities.len(), 2);
    assert_eq!(r.inline_quantities[0].value(), &Value::from(2.0));
    assert_eq!(r.inline_quantities[0].unit(), Some("ºC"));
    assert_eq!(r.inline_quantities[1].value(), &Value::from(150.0));
    assert_eq!(r.inline_quantities[1].unit(), Some("F"));
    let Content::Step(first_step) = &r.sections[0].content[0] else {
        panic!()
    };
    assert_eq!(
        first_step.items,
        vec![
            Item::Text {
                value: "text ".into()
            },
            Item::InlineQuantity { index: 0 },
            Item::Text {
                value: " more text ".into()
            },
            Item::InlineQuantity { index: 1 },
            Item::Text {
                value: " end text".into()
            }
        ]
    );
}

#[test]
fn no_steps_component_mode() {
    let input = indoc! {r#"
        >> [mode]: components
        @igr
        >> [mode]: steps
        = section
        step
    "#};
    let r = cooklang::parse(input).unwrap_output();
    assert_eq!(r.sections.len(), 1);
    assert_eq!(r.sections[0].name.as_deref(), Some("section"));
    assert!(matches!(
        r.sections[0].content.as_slice(),
        [Content::Step(_)]
    ));
}

#[test]
fn text_steps_extension() {
    let input = "> text";

    let r = CooklangParser::canonical().parse(input).unwrap_output();
    assert!(matches!(
        r.sections[0].content.as_slice(),
        [Content::Text(_)]
    ));
}

#[test]
fn timer_missing_unit_warning() {
    let input = "Cook for ~{30}";

    let parser = CooklangParser::canonical();
    let result = parser.parse(input);

    // Check that we get a warning, not an error
    let report = result.report();

    // Verify we have exactly one warning
    assert_eq!(report.iter().count(), 1);

    // Check that it's a warning, not an error
    let diag = report.iter().next().unwrap();
    assert_eq!(diag.severity, cooklang::error::Severity::Warning);

    // Verify the warning message
    assert!(diag
        .message
        .contains("Invalid timer quantity: missing unit"));

    // Should parse successfully (not error)
    let _r = result.unwrap_output();
}

fn timer_with(
    extensions: Extensions,
    input: &str,
) -> (Option<(Value, Option<String>)>, Vec<String>) {
    let parser = CooklangParser::new(extensions, cooklang::Converter::empty());
    let result = parser.parse(input);
    let messages = result
        .report()
        .iter()
        .map(|d| d.message.to_string())
        .collect();
    let recipe = result.into_output().expect("parses");
    let quantity = recipe.timers[0]
        .quantity
        .as_ref()
        .map(|q| (q.value().clone(), q.unit().map(str::to_string)));
    (quantity, messages)
}

fn minutes(value: f64) -> Option<(Value, Option<String>)> {
    Some((value.into(), Some("min".into())))
}

fn range(start: f64, end: f64) -> Value {
    Value::Range {
        start: start.into(),
        end: end.into(),
    }
}

fn minutes_range(start: f64, end: f64) -> Option<(Value, Option<String>)> {
    in_unit(range(start, end), "min")
}

fn in_unit(value: Value, unit: &str) -> Option<(Value, Option<String>)> {
    Some((value, Some(unit.into())))
}

#[test_case("~{1h20m}" => minutes(80.0); "hours and minutes")]
#[test_case("~{1 hour 20 minutes}" => minutes(80.0); "spelled out")]
#[test_case("~{1 hour and 15 minutes}" => minutes(75.0); "with and")]
#[test_case("~{1h20m-1h25m}" => minutes_range(80.0, 85.0); "duration range")]
#[test_case("~{1h}" => in_unit(Value::from(1.0), "h"); "one unit is kept")]
#[test_case("~{90 s}" => in_unit(Value::from(90.0), "s"); "seconds are kept")]
#[test_case("~{1 week}" => in_unit(Value::from(1.0), "week"); "weeks")]
#[test_case("~{30-40%minutes}" => in_unit(range(30.0, 40.0), "minutes"); "text range with unit")]
#[test_case("~{1-3%seconds}" => in_unit(range(1.0, 3.0), "seconds"); "seconds range")]
#[test_case("~{1-3 seconds}" => in_unit(range(1.0, 3.0), "seconds"); "unit after a range applies to both ends")]
#[test_case("~{9-10%mins}" => in_unit(range(9.0, 10.0), "mins"); "mins")]
#[test_case("~{1 1/2%hours}" => in_unit(Value::from(1.5), "hours"); "numeric value is kept")]
#[test_case("~{1 1/2-2%hours}" => in_unit(range(1.5, 2.0), "hours"); "fraction in a range")]
#[test_case("~{1/2-1 hour}" => in_unit(range(0.5, 1.0), "hour"); "fraction before the unit")]
fn timer_durations(timer: &str) -> Option<(Value, Option<String>)> {
    let (quantity, messages) =
        timer_with(Extensions::TIMER_DURATIONS, &format!("Bake for {timer}."));
    assert!(messages.is_empty(), "{messages:?}");
    quantity
}

#[test_case("~{30}"; "bare number")]
#[test_case("~{minutes}"; "unit without a number")]
#[test_case("~{a while}"; "text")]
#[test_case("~{3 eggs}"; "not a time unit")]
#[test_case("~{1-1h25m}"; "bare start with a compound end")]
fn timer_durations_still_warn_without_a_unit(timer: &str) {
    let (_, messages) = timer_with(Extensions::TIMER_DURATIONS, &format!("Wait {timer}."));
    assert_eq!(messages, ["Invalid timer quantity: missing unit"]);
}

#[test]
fn timer_durations_are_off_unless_enabled() {
    let (quantity, messages) = timer_with(Extensions::empty(), "Bake for ~{1h20m}.");
    assert_eq!(quantity, Some((Value::Text("1h20m".into()), None)));
    assert_eq!(messages, ["Invalid timer quantity: missing unit"]);
}

#[test]
fn timer_durations_with_all_extensions() {
    // Advanced units rejects text timer values, so the duration must be read first.
    let (quantity, messages) = timer_with(Extensions::all(), "Bake for ~{1h20m-1h25m}.");
    assert_eq!(quantity, minutes_range(80.0, 85.0));
    assert!(messages.is_empty(), "{messages:?}");
}

fn warnings(extensions: Extensions, input: &str) -> Vec<String> {
    let parser = CooklangParser::new(extensions, cooklang::Converter::empty());
    let result = parser.parse(input);
    let messages = result
        .report()
        .iter()
        .map(|d| d.message.to_string())
        .collect();
    result.into_output().expect("parses");
    messages
}

#[test_case(Extensions::TIMER_DURATIONS, "~{10-5%min}" => vec!["Range is backwards"]; "timer text range")]
#[test_case(Extensions::TIMER_DURATIONS, "~{1h25m-1h20m}" => vec!["Range is backwards"]; "timer compound range")]
#[test_case(Extensions::TIMER_DURATIONS, "~{5-5%min}" => vec!["Range has the same start and end"]; "timer same ends")]
#[test_case(Extensions::TIMER_DURATIONS, "~{1h-60m}" => vec!["Range has the same start and end"]; "timer same ends in other units")]
#[test_case(Extensions::TIMER_DURATIONS, "~{5-10%min}" => Vec::<String>::new(); "timer in order")]
#[test_case(Extensions::RANGE_VALUES, "@salt{3-1%tsp}" => vec!["Range is backwards"]; "ingredient backwards")]
#[test_case(Extensions::RANGE_VALUES, "@salt{2-2%tsp}" => vec!["Range has the same start and end"]; "ingredient same ends")]
#[test_case(Extensions::RANGE_VALUES, "@salt{1/2-1%tsp}" => Vec::<String>::new(); "ingredient in order")]
#[test_case(Extensions::RANGE_VALUES | Extensions::TIMER_DURATIONS, "~{10-5%min}" => vec!["Range is backwards"]; "warned once with both")]
fn range_warnings(extensions: Extensions, input: &str) -> Vec<String> {
    warnings(extensions, input)
}
