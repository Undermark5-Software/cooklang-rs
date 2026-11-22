import CooklangParser
import XCTest

final class IngredientListTests: XCTestCase {
  private let aisleInput = """
    [fruit and veg]
    apple gala | apples
    aubergine
    avocado | avocados

    [milk and dairy]
    butter
    egg | eggs

    [dried herbs and spices]
    bay leaves
    black pepper
    """

  func testCombinesByNameAndUnit() {
    let combined = combineIngredients(ingredients: [
      ingredient("salt", Amount(quantity: .number(value: 5), units: "g")),
      ingredient("pepper", Amount(quantity: .number(value: 5), units: "mg")),
      ingredient("salt", Amount(quantity: .number(value: 3), units: "g")),
      ingredient("salt", Amount(quantity: .number(value: 0.005), units: "kg")),
      ingredient("pepper", Amount(quantity: .text(value: "some"), units: nil)),
      ingredient("pepper", nil),
    ])

    XCTAssertEqual(
      combined["salt"],
      [
        GroupedQuantityKey(name: "g", unitType: .number): .number(value: 8),
        GroupedQuantityKey(name: "kg", unitType: .number): .number(value: 0.005),
      ]
    )
    XCTAssertEqual(
      combined["pepper"],
      [
        GroupedQuantityKey(name: "mg", unitType: .number): .number(value: 5),
        GroupedQuantityKey(name: "", unitType: .text): .text(value: "some"),
        GroupedQuantityKey(name: "", unitType: .empty): .empty,
      ]
    )
  }

  func testCombinesSelectedIndices() {
    let recipe = parseRecipe(
      input: "= One\n\nAdd @salt{1%tsp}.\n\n= Two\n\nAdd @salt{2%tsp} and @pepper{1%tsp}.",
      scalingFactor: 1.0)
    let second = recipe.sections()[1]
    let combined = combineIngredientsSelected(
      ingredients: recipe.ingredients(), indices: second.ingredientRefs)
    XCTAssertEqual(Set(combined.keys), ["salt", "pepper"])
    XCTAssertEqual(combined["salt"], [GroupedQuantityKey(name: "tsp", unitType: .number): .number(value: 2)])
  }

  func testLooksUpAisleCategories() {
    let aisle = parseAisleConfig(input: aisleInput)
    XCTAssertEqual(aisle.categoryFor(ingredientName: "bay leaves"), "dried herbs and spices")
    XCTAssertEqual(aisle.categoryFor(ingredientName: "eggs"), "milk and dairy")
    XCTAssertNil(aisle.categoryFor(ingredientName: "some weird ingredient"))

    XCTAssertEqual(
      aisle.categories().map(\.name), ["fruit and veg", "milk and dairy", "dried herbs and spices"])
    XCTAssertEqual(
      aisle.categories().first?.ingredients.first, AisleIngredient(name: "apple gala", aliases: ["apples"]))
  }

  func testResolvesCommonNames() {
    let aisle = parseAisleConfig(input: aisleInput)
    XCTAssertEqual(aisle.commonNameFor(ingredientName: "apples"), "apple gala")
    XCTAssertEqual(aisle.commonNameFor(ingredientName: "eggs"), "egg")
    XCTAssertEqual(aisle.commonNameFor(ingredientName: "salt"), "salt")

    let combined = combineIngredients(ingredients: [
      ingredient("apples", Amount(quantity: .number(value: 3), units: nil)),
      ingredient("eggs", Amount(quantity: .number(value: 2), units: nil)),
      ingredient("salt", Amount(quantity: .number(value: 1), units: "tsp")),
    ])
    XCTAssertEqual(Set(useCommonNames(list: combined, aisle: aisle).keys), ["apple gala", "egg", "salt"])
  }

  func testEmptyAisleConfigHasNoCategories() {
    let aisle = parseAisleConfig(input: "")
    XCTAssertTrue(aisle.categories().isEmpty)
    XCTAssertNil(aisle.categoryFor(ingredientName: "salt"))
  }
}
