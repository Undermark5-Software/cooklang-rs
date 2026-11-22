import CooklangParser
import XCTest

final class RecipeParsingTests: XCTestCase {
  /// The original Demo.swift recipe, against the current API.
  func testParsesDemoRecipe() {
    let input = """
      Preheat the oven to 180C.

      Mix @flour{2%cups} with @baking powder{1%tsp}.

      Add @eggs{2} and stir for ~{2%minutes}.
      """

    let recipe = parseRecipe(input: input, scalingFactor: 1.0)
    let sections = recipe.sections()
    XCTAssertEqual(sections.count, 1)
    XCTAssertNil(sections[0].title)

    XCTAssertEqual(
      steps(sections[0]).map(\.items),
      [
        [.text(value: "Preheat the oven to 180C.")],
        [
          .text(value: "Mix "),
          .ingredientRef(index: 0),
          .text(value: " with "),
          .ingredientRef(index: 1),
          .text(value: "."),
        ],
        [
          .text(value: "Add "),
          .ingredientRef(index: 2),
          .text(value: " and stir for "),
          .timerRef(index: 0),
          .text(value: "."),
        ],
      ]
    )

    XCTAssertEqual(
      recipe.ingredients(),
      [
        ingredient("flour", Amount(quantity: .number(value: 2), units: "cups")),
        ingredient("baking powder", Amount(quantity: .number(value: 1), units: "tsp")),
        ingredient("eggs", Amount(quantity: .number(value: 2), units: nil)),
      ]
    )
    XCTAssertEqual(
      recipe.timers(),
      [CooklangParser.Timer(name: unnamedTimer, amount: Amount(quantity: .number(value: 2), units: "minutes"))]
    )
    XCTAssertEqual(recipe.cookware(), [])
  }

  func testTracksComponentRefsPerStepAndSection() {
    let recipe = parseRecipe(
      input: "Put @butter{1%tbsp} in a #pan{}.\n\nFry @eggs{2} for ~{3%minutes}.", scalingFactor: 1.0)
    let section = recipe.sections()[0]
    XCTAssertEqual(section.ingredientRefs, [0, 1])
    XCTAssertEqual(section.cookwareRefs, [0])
    XCTAssertEqual(section.timerRefs, [0])

    let stepList = steps(section)
    XCTAssertEqual(stepList.count, 2)
    XCTAssertEqual(stepList[0].ingredientRefs, [0])
    XCTAssertEqual(stepList[0].cookwareRefs, [0])
    XCTAssertEqual(stepList[0].timerRefs, [])
    XCTAssertEqual(stepList[1].ingredientRefs, [1])
    XCTAssertEqual(stepList[1].timerRefs, [0])
  }

  func testDerefsComponents() {
    let recipe = parseRecipe(
      input: "Put @butter{1%tbsp} in a #pan{} for ~{3%minutes}.", scalingFactor: 1.0)
    let items = steps(recipe.sections()[0])[0].items

    XCTAssertEqual(
      items.map { derefComponent(recipe: recipe, item: $0) },
      [
        .textComponent("Put "),
        .ingredientComponent(ingredient("butter", Amount(quantity: .number(value: 1), units: "tbsp"))),
        .textComponent(" in a "),
        .cookwareComponent(Cookware(name: "pan", amount: nil, optional: false)),
        .textComponent(" for "),
        .timerComponent(
          CooklangParser.Timer(name: unnamedTimer, amount: Amount(quantity: .number(value: 3), units: "minutes"))),
        .textComponent("."),
      ]
    )

    XCTAssertEqual(derefIngredient(recipe: recipe, index: 0).name, "butter")
    XCTAssertEqual(derefCookware(recipe: recipe, index: 0).name, "pan")
    XCTAssertEqual(
      derefTimer(recipe: recipe, index: 0).amount,
      Amount(quantity: .number(value: 3), units: "minutes"))
  }

  func testParsesNotes() {
    let input = """
      > This dish is even better the next day.

      Cook @onions{3%large} until brown
      """
    let blocks = parseRecipe(input: input, scalingFactor: 1.0).sections()[0].blocks
    XCTAssertEqual(
      blocks,
      [
        .noteBlock(BlockNote(text: "This dish is even better the next day.")),
        .stepBlock(
          Step(
            items: [.text(value: "Cook "), .ingredientRef(index: 0), .text(value: " until brown")],
            ingredientRefs: [0], cookwareRefs: [], timerRefs: [])),
      ]
    )
  }

  func testJoinsMultilineSteps() {
    let recipe = parseRecipe(input: "add @onions{2} to pan\nheat until golden", scalingFactor: 1.0)
    XCTAssertEqual(
      steps(recipe.sections()[0]).map(\.items),
      [[.text(value: "add "), .ingredientRef(index: 0), .text(value: " to pan heat until golden")]]
    )
  }

  func testParsesSections() {
    let input = """
      = Dough

      Mix @flour{200%g} and @water{50%ml}.

      == Filling ==

      Combine @cheese{100%g} and @spinach{50%g}.
      """
    let recipe = parseRecipe(input: input, scalingFactor: 1.0)
    let sections = recipe.sections()
    XCTAssertEqual(sections.map(\.title), ["Dough", "Filling"])
    XCTAssertEqual(sections[0].ingredientRefs, [0, 1])
    XCTAssertEqual(sections[1].ingredientRefs, [2, 3])
    XCTAssertEqual(recipe.ingredients().map(\.name), ["flour", "water", "cheese", "spinach"])
  }

  func testParsesDescriptors() {
    let recipe = parseRecipe(input: "Add @onion{1}(finely chopped).", scalingFactor: 1.0)
    XCTAssertEqual(recipe.ingredients().first?.descriptor, "finely chopped")
  }

  func testParsesRecipeReferences() {
    let recipe = parseRecipe(
      input: "Serve the @./pasta/spaghetti{1%portion} with sauce", scalingFactor: 1.0)
    let ingredient = recipe.ingredients()[0]
    XCTAssertEqual(ingredient.name, "spaghetti")
    XCTAssertEqual(
      ingredient.reference, RecipeReference(name: "spaghetti", components: [".", "pasta"]))
  }

  func testParsesOptionalComponents() {
    let recipe = parseRecipe(
      input: "Top with @?chives and @salt, use a #?splatter guard{}.", scalingFactor: 1.0)
    XCTAssertEqual(recipe.ingredients().map(\.name), ["chives", "salt"])
    XCTAssertEqual(recipe.ingredients().map(\.optional), [true, false])
    XCTAssertEqual(recipe.cookware().map(\.optional), [true])
  }

  func testParsesValueKinds() {
    let recipe = parseRecipe(input: "Use @a{1-2%cups}, @b{some%pinch}, @c{} and @d.", scalingFactor: 1.0)
    let amounts = Dictionary(uniqueKeysWithValues: recipe.ingredients().map { ($0.name, $0.amount) })
    // parseRecipe uses the canonical parser, which has the RANGE_VALUES extension
    // disabled, so "1-2" stays text. Value.range only comes from parseValue().
    XCTAssertEqual(amounts["a"], Amount(quantity: .text(value: "1-2"), units: "cups"))
    XCTAssertEqual(amounts["b"], Amount(quantity: .text(value: "some"), units: "pinch"))
    XCTAssertEqual(amounts["c"], .some(nil))
    XCTAssertEqual(amounts["d"], .some(nil))
  }

  func testAppliesScalingFactor() {
    let recipe = parseRecipe(
      input: "Mix @flour{200%g}, @eggs{2} and @milk{1/2%cups}. Bake for ~{30%minutes}.",
      scalingFactor: 2.0)
    let amounts = Dictionary(uniqueKeysWithValues: recipe.ingredients().map { ($0.name, $0.amount) })
    XCTAssertEqual(amounts["flour"], Amount(quantity: .number(value: 400), units: "g"))
    XCTAssertEqual(amounts["eggs"], Amount(quantity: .number(value: 4), units: nil))
    XCTAssertEqual(amounts["milk"], Amount(quantity: .number(value: 1), units: "cups"))
    // Timers are not scaled.
    XCTAssertEqual(recipe.timers()[0].amount, Amount(quantity: .number(value: 30), units: "minutes"))
  }

  func testParsesEmptyRecipe() {
    let recipe = parseRecipe(input: "", scalingFactor: 1.0)
    XCTAssertTrue(recipe.ingredients().isEmpty)
    XCTAssertTrue(recipe.sections().allSatisfy { $0.blocks.isEmpty })
  }
}

/// Unnamed timers (`~{2%minutes}`) currently come back with an empty name rather than nil:
/// bindings/src/model.rs converts with `Some(name.unwrap_or_default())`. Pinned here so a
/// change is deliberate; update this if that conversion is fixed to pass `None` through.
let unnamedTimer: String? = ""

func ingredient(_ name: String, _ amount: Amount?) -> Ingredient {
  Ingredient(name: name, amount: amount, descriptor: nil, reference: nil, optional: false)
}

func steps(_ section: Section) -> [Step] {
  section.blocks.compactMap { block in
    if case .stepBlock(let step) = block { return step }
    XCTFail("Expected a step block, got \(block)")
    return nil
  }
}
