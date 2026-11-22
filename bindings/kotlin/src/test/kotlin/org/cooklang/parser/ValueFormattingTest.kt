package org.cooklang.parser

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ValueFormattingTest {

    @Test
    fun formatsNumbersAsFractions() {
        val cases = mapOf(
            0.5 to "1/2",
            0.25 to "1/4",
            0.75 to "3/4",
            0.333333 to "1/3",
            0.666667 to "2/3",
            1.5 to "1 1/2",
            2.25 to "2 1/4",
            2.0 to "2",
            1.23 to "1.23",
            0.899 to "0.899",
        )
        for ((input, expected) in cases) {
            assertEquals(expected, formatValue(Value.Number(input)), "formatValue($input)")
        }
    }

    @Test
    fun cleansUpFloatingPointNoise() {
        assertEquals("0.9", formatValue(Value.Number(0.89999999999)))
        assertEquals("0.3", formatValue(Value.Number(0.30000000001)))
        assertEquals("2", formatValue(Value.Number(1.9999999999)))
    }

    @Test
    fun formatsOtherValueKinds() {
        assertEquals("1/2 - 3/4", formatValue(Value.Range(0.5, 0.75)))
        assertEquals("pinch", formatValue(Value.Text("pinch")))
        assertNull(formatValue(Value.Empty))
    }

    @Test
    fun formatsAmounts() {
        assertEquals("2/3 cups", formatAmount(Amount(Value.Number(0.666667), "cups")))
        assertEquals("1 1/2 tsp", formatAmount(Amount(Value.Number(1.5), "tsp")))
        assertEquals("0.9 cups", formatAmount(Amount(Value.Number(0.89999999999), "cups")))
    }

    @Test
    fun parsesValues() {
        assertEquals(Value.Number(0.5), parseValue("1/2"))
        assertEquals(Value.Number(0.75), parseValue("3/4"))
        assertEquals(Value.Number(1.5), parseValue("1 1/2"))
        assertEquals(Value.Number(2.75), parseValue("2 3/4"))
        assertEquals(Value.Number(5.0), parseValue("5"))
        assertEquals(Value.Number(1.5), parseValue("1.5"))
        assertEquals(Value.Range(0.5, 0.75), parseValue("1/2 - 3/4"))
        assertEquals(Value.Range(1.0, 2.0), parseValue("1 - 2"))
        assertEquals(Value.Text("pinch"), parseValue("pinch"))
    }
}
