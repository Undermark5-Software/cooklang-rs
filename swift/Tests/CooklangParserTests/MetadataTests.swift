import CooklangParser
import XCTest

final class MetadataTests: XCTestCase {
  private let input = """
    ---
    title: Test Recipe
    description: A quick test
    author: John Doe <https://johndoe.com>
    source: https://google.com
    servings: 4
    tags: easy, quick, vegetarian
    cuisine: Italian
    nutrition: 500 calories
    rating: 5 stars
    ---
    a test @salt{1%mg} more text
    """

  func testReadsStandardFields() {
    let recipe = parseRecipe(input: input, scalingFactor: 1.0)
    XCTAssertEqual(metadataTitle(recipe: recipe), "Test Recipe")
    XCTAssertEqual(metadataDescription(recipe: recipe), "A quick test")
    XCTAssertEqual(metadataServings(recipe: recipe), .number(value: 4))
    XCTAssertEqual(metadataTags(recipe: recipe), ["easy", "quick", "vegetarian"])
    XCTAssertEqual(metadataAuthor(recipe: recipe), NameAndUrl(name: "John Doe", url: "https://johndoe.com"))
    XCTAssertEqual(metadataSource(recipe: recipe)?.url, "https://google.com")
  }

  func testReadsStdKeysAndCustomKeys() {
    let recipe = parseRecipe(input: input, scalingFactor: 1.0)
    XCTAssertEqual(metadataGetStd(recipe: recipe, key: .cuisine), "Italian")
    XCTAssertEqual(metadataGetStd(recipe: recipe, key: .title), "Test Recipe")
    XCTAssertEqual(metadataGet(recipe: recipe, key: "nutrition"), "500 calories")
    XCTAssertEqual(Set(metadataCustomKeys(recipe: recipe)), ["nutrition", "rating"])
  }

  func testReadsTextServings() {
    let recipe = parseRecipe(input: "---\nservings: 2-3 portions\n---\nCook.", scalingFactor: 1.0)
    XCTAssertEqual(metadataServings(recipe: recipe), .text(value: "2-3 portions"))
  }

  func testReturnsNilForMissingMetadata() {
    let recipe = parseRecipe(input: "Just a step.", scalingFactor: 1.0)
    XCTAssertNil(metadataTitle(recipe: recipe))
    XCTAssertNil(metadataDescription(recipe: recipe))
    XCTAssertNil(metadataServings(recipe: recipe))
    XCTAssertNil(metadataTags(recipe: recipe))
    XCTAssertNil(metadataAuthor(recipe: recipe))
    XCTAssertNil(metadataSource(recipe: recipe))
    XCTAssertNil(metadataTime(recipe: recipe))
    XCTAssertNil(metadataGet(recipe: recipe, key: "nope"))
    XCTAssertNil(metadataGetStd(recipe: recipe, key: .cuisine))
    XCTAssertEqual(metadataCustomKeys(recipe: recipe), [])
  }
}
