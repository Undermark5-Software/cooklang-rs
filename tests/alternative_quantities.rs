use cooklang::{
    convert::System, Content, Converter, CooklangParser, Extensions, Item, Quantity, Recipe,
};
use test_case::test_case;

fn parse_with(extensions: Extensions, input: &str) -> (Recipe, Vec<String>) {
    let parser = CooklangParser::new(extensions, Converter::bundled());
    let result = parser.parse(input);
    // a lock on a numeric ingredient warns even though it works; not about alternatives
    let messages = result
        .report()
        .iter()
        .map(|d| d.message.to_string())
        .filter(|m| m != "Unnecessary scaling lock modifier")
        .collect();
    let recipe = result.output().expect("recipe").clone();
    (recipe, messages)
}

fn parse(input: &str) -> (Recipe, Vec<String>) {
    parse_with(Extensions::all(), input)
}

fn written(q: &Quantity) -> String {
    q.to_string()
}

/// The first ingredient's main quantity and alternatives, as text.
fn measures(recipe: &Recipe) -> (Option<String>, Vec<String>) {
    let igr = &recipe.ingredients[0];
    (
        igr.quantity.as_ref().map(written),
        igr.alternative_quantities.iter().map(written).collect(),
    )
}

/// The first step's text, with components replaced by their names.
fn step_text(recipe: &Recipe) -> String {
    let Content::Step(step) = &recipe.sections[0].content[0] else {
        panic!("not a step");
    };
    step.items
        .iter()
        .map(|item| match item {
            Item::Text { value } => value.clone(),
            Item::Ingredient { index } => recipe.ingredients[*index].name.clone(),
            Item::Cookware { index } => recipe.cookware[*index].name.clone(),
            Item::Timer { .. } => "<timer>".to_string(),
            Item::InlineQuantity { index } => {
                format!("<{}>", recipe.inline_quantities[*index])
            }
        })
        .collect()
}

#[test]
fn braces_after_the_quantity_are_alternatives() {
    let (recipe, messages) = parse("Add @plain flour{125%g}{1/2%cup}(sifted) slowly.");
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(
        measures(&recipe),
        (Some("125 g".into()), vec!["1/2 cup".into()])
    );
    let igr = &recipe.ingredients[0];
    assert_eq!(igr.name, "plain flour");
    assert_eq!(igr.note.as_deref(), Some("sifted"));
    assert_eq!(step_text(&recipe), "Add plain flour slowly.");
    assert!(recipe.inline_quantities.is_empty());
}

#[test]
fn any_number_of_alternatives() {
    let (recipe, messages) = parse("Soften @butter{1%stick}{8%tbsp}{113%g}.");
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(
        measures(&recipe),
        (
            Some("1 stick".into()),
            vec!["8 tbsp".into(), "113 g".into()]
        )
    );
    assert_eq!(step_text(&recipe), "Soften butter.");
}

#[test]
fn alternatives_take_every_quantity_form() {
    let (recipe, messages) = parse("Add @milk{2-3%cups}{500 ml}{a splash}.");
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(
        measures(&recipe),
        (
            Some("2-3 cups".into()),
            vec!["500 ml".into(), "a splash".into()]
        )
    );
}

#[test_case("Add @flour{125%g} {1/2%cup}." ; "space before the braces")]
#[test_case("Add @flour{125%g}{1/2%cup" ; "unclosed braces")]
fn braces_that_are_not_alternatives_stay_text(input: &str) {
    let (recipe, messages) = parse(input);
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(measures(&recipe), (Some("125 g".into()), vec![]));
    assert!(step_text(&recipe).contains("{"), "{}", step_text(&recipe));
}

#[test]
fn off_unless_enabled() {
    let (recipe, messages) = parse_with(
        Extensions::all() - Extensions::ALTERNATIVE_QUANTITIES,
        "Add @flour{125%g}{1/2%cup}.",
    );
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(measures(&recipe), (Some("125 g".into()), vec![]));
    // the braces are text
    assert!(
        step_text(&recipe).starts_with("Add flour{"),
        "{}",
        step_text(&recipe)
    );
}

#[test]
fn not_part_of_compat() {
    assert!(!Extensions::COMPAT.contains(Extensions::ALTERNATIVE_QUANTITIES));
}

#[test]
fn ignored_without_a_main_quantity() {
    let (recipe, messages) = parse("Add @salt{}{1%pinch}.");
    assert_eq!(
        messages,
        ["Alternative quantities need a main quantity, they will be ignored"]
    );
    assert_eq!(measures(&recipe), (None, vec![]));
    assert_eq!(step_text(&recipe), "Add salt.");
}

#[test]
fn empty_alternative_is_ignored() {
    let (recipe, messages) = parse("Add @flour{125%g}{ }{1/2%cup}.");
    assert_eq!(messages, ["Empty alternative quantity, it will be ignored"]);
    assert_eq!(
        measures(&recipe),
        (Some("125 g".into()), vec!["1/2 cup".into()])
    );
}

#[test_case("@salt{=1%tsp}{5%g}" ; "main locked")]
#[test_case("@salt{1%tsp}{=5%g}" ; "alternative locked")]
fn warns_when_scaling_differs(input: &str) {
    let (_, messages) = parse(input);
    assert_eq!(
        messages,
        ["Alternative quantity scales differently from the main quantity"]
    );
}

#[test_case("@salt{=1%tsp}{=5%g}" ; "both locked")]
#[test_case("@salt{1%tsp}{a pinch}" ; "text alternative")]
fn scaling_agrees(input: &str) {
    let (_, messages) = parse(input);
    assert!(messages.is_empty(), "{messages:?}");
}

#[test_case("@flour{125%g}{125%grams}" ; "other spelling of the main unit")]
#[test_case("@flour{1%cup}{125%g}{125%g}" ; "two alternatives")]
#[test_case("@garlic{2%cloves}{1%clove}" ; "count unit plural")]
#[test_case("@cheese{1%handful}{2%handful}" ; "unknown unit")]
fn warns_on_a_repeated_unit(input: &str) {
    let (_, messages) = parse(input);
    assert_eq!(messages, ["Alternative quantity repeats a unit"]);
}

#[test]
fn different_units_of_one_kind_are_fine() {
    let (_, messages) = parse("@flour{125%g}{0.125%kg}{4.4%oz}");
    assert!(messages.is_empty(), "{messages:?}");
}

#[test_case("Heat the #pan{1}{2}." ; "cookware")]
#[test_case("Wait ~{5%min}{300%s}." ; "timer")]
fn other_components_keep_the_braces_as_text(input: &str) {
    let (recipe, messages) = parse(input);
    let container = if input.contains('#') {
        "cookware"
    } else {
        "timer"
    };
    assert_eq!(
        messages,
        [format!(
            "A {container} cannot have alternative quantities, they will be text"
        )]
    );
    assert!(step_text(&recipe).contains("{"), "{}", step_text(&recipe));
}

#[test]
fn alternatives_scale_with_the_recipe() {
    let (mut recipe, _) = parse("Add @flour{1/2%cup}{125%g} and @sugar{125%g}{1/2%cup}.");
    recipe.scale(2.0, &Converter::bundled());
    let igr = &recipe.ingredients;
    assert_eq!(written(igr[0].quantity.as_ref().unwrap()), "1 c");
    assert_eq!(written(&igr[0].alternative_quantities[0]), "250 g");
    // an alternative is written the same way a main quantity is
    assert_eq!(written(igr[1].quantity.as_ref().unwrap()), "250 g");
    assert_eq!(written(&igr[1].alternative_quantities[0]), "1 c");
}

#[test]
fn locked_alternatives_keep_their_amount() {
    let (mut recipe, _) = parse("Add @salt{=1%tsp}{=5%g}.");
    recipe.scale(2.0, &Converter::bundled());
    assert_eq!(
        measures(&recipe),
        (Some("1 tsp".into()), vec!["5 g".into()])
    );
}

#[test]
fn alternatives_convert_with_the_recipe() {
    let converter = Converter::bundled();
    let (mut recipe, _) = parse("Add @butter{4%oz}{113%g}.");
    let errors = recipe.convert(System::Metric, &converter);
    assert!(errors.is_empty(), "{errors:?}");
    let igr = &recipe.ingredients[0];
    assert_eq!(igr.quantity.as_ref().unwrap().unit(), Some("g"));
    assert_eq!(igr.alternative_quantities[0].unit(), Some("g"));
}

#[test]
fn references_have_their_own_alternatives_and_totals_use_the_main_quantity() {
    let (recipe, messages) =
        parse("Add @flour{100%g}{3/4%cup}, then the rest of the @&flour{50%g}{1/3%cup}.");
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(recipe.ingredients[1].alternative_quantities.len(), 1);
    let total = recipe.ingredients[0].group_quantities(&recipe.ingredients, &Converter::bundled());
    assert_eq!(total.to_string(), "150 g");
}

#[test]
fn serialized_only_when_present() {
    let (recipe, _) = parse("Add @flour{125%g}{1/2%cup} and @salt{1%tsp}.");
    let json = serde_json::to_value(&recipe.ingredients).unwrap();
    assert!(json[0].get("alternative_quantities").is_some());
    assert!(json[1].get("alternative_quantities").is_none());

    // older output without the field still reads
    let mut salt = json[1].clone();
    salt.as_object_mut()
        .unwrap()
        .remove("alternative_quantities");
    let salt: cooklang::Ingredient = serde_json::from_value(salt).unwrap();
    assert!(salt.alternative_quantities.is_empty());
}
