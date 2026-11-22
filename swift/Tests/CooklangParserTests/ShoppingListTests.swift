import CooklangParser
import XCTest

final class ShoppingListTests: XCTestCase {
  func testParsesNestedRecipes() throws {
    let list = try parseShoppingList(
      input: "./Breakfast/Easy Pancakes{2}\n  ./Shared/Guacamole\n./Thai Green Curry\n")
    XCTAssertEqual(
      list.items,
      [
        .recipe(
          path: "Breakfast/Easy Pancakes", multiplier: 2,
          children: [.recipe(path: "Shared/Guacamole", multiplier: nil, children: [], optional: false)],
          optional: false),
        .recipe(path: "Thai Green Curry", multiplier: nil, children: [], optional: false),
      ]
    )
  }

  func testWritesShoppingList() throws {
    let list = ShoppingList(items: [
      .recipe(
        path: "Breakfast/Pancakes", multiplier: 2,
        children: [.recipe(path: "Shared/Syrup", multiplier: nil, children: [], optional: false)],
        optional: false),
      .ingredient(name: "salt", quantity: "2%tsp", optional: false),
    ])
    XCTAssertEqual(
      try writeShoppingList(list: list), "./Breakfast/Pancakes{2}\n  ./Shared/Syrup\nsalt{2%tsp}\n")
  }

  func testRoundTrips() throws {
    let input = "./Meal Plan.menu\n  ./Breakfast/Pancakes{2}\n    ./Shared/Guacamole\n  ./Dinner/Curry\n"
    XCTAssertEqual(try writeShoppingList(list: try parseShoppingList(input: input)), input)
  }

  func testHandlesOptionalIngredients() throws {
    let input = "./Breakfast/Eggs on toast{2}\n  ? chives\n"
    let list = try parseShoppingList(input: input)
    guard case .recipe(_, _, let children, _) = list.items.first else {
      return XCTFail("Expected a recipe item, got \(list.items)")
    }
    XCTAssertEqual(children, [.ingredient(name: "chives", quantity: nil, optional: true)])
    XCTAssertEqual(try writeShoppingList(list: list), input)
  }

  func testParsesCheckedEntries() {
    XCTAssertEqual(
      parseShoppingChecked(input: "+ salt\n+ pepper\n- salt\n+ garlic\n"),
      [.checked(name: "salt"), .checked(name: "pepper"), .unchecked(name: "salt"), .checked(name: "garlic")]
    )
  }

  func testComputesCheckedSet() {
    let entries: [CheckEntry] = [.checked(name: "salt"), .checked(name: "pepper"), .unchecked(name: "salt")]
    XCTAssertEqual(Set(shoppingCheckedSet(entries: entries)), ["pepper"])
  }

  func testWritesCheckEntries() throws {
    XCTAssertEqual(try writeShoppingCheckEntry(entry: .checked(name: "salt")), "+ salt\n")
    XCTAssertEqual(try writeShoppingCheckEntry(entry: .unchecked(name: "pepper")), "- pepper\n")
  }

  func testCompactsCheckedEntries() {
    let entries: [CheckEntry] = [
      .checked(name: "salt"), .checked(name: "pepper"), .checked(name: "removed ingredient"),
    ]
    XCTAssertEqual(
      compactShoppingChecked(entries: entries, currentIngredients: ["salt"]), [.checked(name: "salt")])
  }

  func testThrowsTypedErrorOnInvalidInput() {
    XCTAssertThrowsError(try parseShoppingList(input: "  ./indented first line\n")) { error in
      guard case ShoppingListError.Parse(let reason) = error else {
        return XCTFail("Expected ShoppingListError.Parse, got \(error)")
      }
      XCTAssertFalse(reason.isEmpty)
    }
  }
}
