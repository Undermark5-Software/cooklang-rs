import CooklangParser
import XCTest

final class ValueFormattingTests: XCTestCase {
  func testFormatsNumbersAsFractions() {
    let cases: [(Double, String)] = [
      (0.5, "1/2"),
      (0.25, "1/4"),
      (0.75, "3/4"),
      (0.333333, "1/3"),
      (0.666667, "2/3"),
      (1.5, "1 1/2"),
      (2.25, "2 1/4"),
      (2.0, "2"),
      (1.23, "1.23"),
      (0.899, "0.899"),
    ]
    for (input, expected) in cases {
      XCTAssertEqual(formatValue(value: .number(value: input)), expected, "formatValue(\(input))")
    }
  }

  func testCleansUpFloatingPointNoise() {
    XCTAssertEqual(formatValue(value: .number(value: 0.89999999999)), "0.9")
    XCTAssertEqual(formatValue(value: .number(value: 0.30000000001)), "0.3")
    XCTAssertEqual(formatValue(value: .number(value: 1.9999999999)), "2")
  }

  func testFormatsOtherValueKinds() {
    XCTAssertEqual(formatValue(value: .range(start: 0.5, end: 0.75)), "1/2 - 3/4")
    XCTAssertEqual(formatValue(value: .text(value: "pinch")), "pinch")
    XCTAssertNil(formatValue(value: .empty))
  }

  func testFormatsAmounts() {
    XCTAssertEqual(formatAmount(amount: Amount(quantity: .number(value: 0.666667), units: "cups")), "2/3 cups")
    XCTAssertEqual(formatAmount(amount: Amount(quantity: .number(value: 1.5), units: "tsp")), "1 1/2 tsp")
    XCTAssertEqual(formatAmount(amount: Amount(quantity: .number(value: 0.89999999999), units: "cups")), "0.9 cups")
  }

  func testParsesValues() {
    XCTAssertEqual(parseValue(s: "1/2"), .number(value: 0.5))
    XCTAssertEqual(parseValue(s: "3/4"), .number(value: 0.75))
    XCTAssertEqual(parseValue(s: "1 1/2"), .number(value: 1.5))
    XCTAssertEqual(parseValue(s: "2 3/4"), .number(value: 2.75))
    XCTAssertEqual(parseValue(s: "5"), .number(value: 5))
    XCTAssertEqual(parseValue(s: "1.5"), .number(value: 1.5))
    XCTAssertEqual(parseValue(s: "1/2 - 3/4"), .range(start: 0.5, end: 0.75))
    XCTAssertEqual(parseValue(s: "1 - 2"), .range(start: 1, end: 2))
    XCTAssertEqual(parseValue(s: "pinch"), .text(value: "pinch"))
  }
}
