package org.cooklang.parser

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class IngredientListTest {

    private val aisleInput = """
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
    """.trimIndent()

    @Test
    fun combinesByNameAndUnit() {
        val combined = combineIngredients(
            listOf(
                ingredient("salt", Amount(Value.Number(5.0), "g")),
                ingredient("pepper", Amount(Value.Number(5.0), "mg")),
                ingredient("salt", Amount(Value.Number(3.0), "g")),
                ingredient("salt", Amount(Value.Number(0.005), "kg")),
                ingredient("pepper", Amount(Value.Text("some"), null)),
                ingredient("pepper", null),
            ),
        )

        assertEquals(
            mapOf(
                GroupedQuantityKey("g", QuantityType.NUMBER) to Value.Number(8.0),
                GroupedQuantityKey("kg", QuantityType.NUMBER) to Value.Number(0.005),
            ),
            combined["salt"],
        )
        assertEquals(
            mapOf(
                GroupedQuantityKey("mg", QuantityType.NUMBER) to Value.Number(5.0),
                GroupedQuantityKey("", QuantityType.TEXT) to Value.Text("some"),
                GroupedQuantityKey("", QuantityType.EMPTY) to Value.Empty,
            ),
            combined["pepper"],
        )
    }

    @Test
    fun combinesSelectedIndices() {
        parseRecipe("= One\n\nAdd @salt{1%tsp}.\n\n= Two\n\nAdd @salt{2%tsp} and @pepper{1%tsp}.", 1.0).use { recipe ->
            val second = recipe.sections()[1]
            val combined = combineIngredientsSelected(recipe.ingredients(), second.ingredientRefs)
            assertEquals(setOf("salt", "pepper"), combined.keys)
            assertEquals(
                mapOf(GroupedQuantityKey("tsp", QuantityType.NUMBER) to Value.Number(2.0)),
                combined["salt"],
            )
        }
    }

    @Test
    fun looksUpAisleCategories() {
        parseAisleConfig(aisleInput).use { aisle ->
            assertEquals("dried herbs and spices", aisle.categoryFor("bay leaves"))
            assertEquals("milk and dairy", aisle.categoryFor("eggs"))
            assertNull(aisle.categoryFor("some weird ingredient"))

            assertEquals(
                listOf("fruit and veg", "milk and dairy", "dried herbs and spices"),
                aisle.categories().map { it.name },
            )
            val fruit = aisle.categories().first()
            assertEquals(AisleIngredient("apple gala", listOf("apples")), fruit.ingredients.first())
        }
    }

    @Test
    fun resolvesCommonNames() {
        parseAisleConfig(aisleInput).use { aisle ->
            assertEquals("apple gala", aisle.commonNameFor("apples"))
            assertEquals("egg", aisle.commonNameFor("eggs"))
            assertEquals("salt", aisle.commonNameFor("salt"))

            val combined = combineIngredients(
                listOf(
                    ingredient("apples", Amount(Value.Number(3.0), null)),
                    ingredient("eggs", Amount(Value.Number(2.0), null)),
                    ingredient("salt", Amount(Value.Number(1.0), "tsp")),
                ),
            )
            assertEquals(setOf("apple gala", "egg", "salt"), useCommonNames(combined, aisle).keys)
        }
    }

    @Test
    fun emptyAisleConfigHasNoCategories() {
        parseAisleConfig("").use { aisle ->
            assertTrue(aisle.categories().isEmpty())
            assertNull(aisle.categoryFor("salt"))
        }
    }
}
