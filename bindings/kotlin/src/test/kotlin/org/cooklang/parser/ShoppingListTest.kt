package org.cooklang.parser

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

class ShoppingListTest {

    @Test
    fun parsesNestedRecipes() {
        val list = parseShoppingList("./Breakfast/Easy Pancakes{2}\n  ./Shared/Guacamole\n./Thai Green Curry\n")

        assertEquals(
            listOf(
                ShoppingListItem.Recipe(
                    path = "Breakfast/Easy Pancakes",
                    multiplier = 2.0,
                    children = listOf(ShoppingListItem.Recipe("Shared/Guacamole", null, emptyList(), false)),
                    optional = false,
                ),
                ShoppingListItem.Recipe("Thai Green Curry", null, emptyList(), false),
            ),
            list.items,
        )
    }

    @Test
    fun writesShoppingList() {
        val list = ShoppingList(
            listOf(
                ShoppingListItem.Recipe(
                    path = "Breakfast/Pancakes",
                    multiplier = 2.0,
                    children = listOf(ShoppingListItem.Recipe("Shared/Syrup", null, emptyList(), false)),
                    optional = false,
                ),
                ShoppingListItem.Ingredient("salt", "2%tsp", false),
            ),
        )
        assertEquals("./Breakfast/Pancakes{2}\n  ./Shared/Syrup\nsalt{2%tsp}\n", writeShoppingList(list))
    }

    @Test
    fun roundTrips() {
        val input = "./Meal Plan.menu\n  ./Breakfast/Pancakes{2}\n    ./Shared/Guacamole\n  ./Dinner/Curry\n"
        assertEquals(input, writeShoppingList(parseShoppingList(input)))
    }

    @Test
    fun handlesOptionalIngredients() {
        val input = "./Breakfast/Eggs on toast{2}\n  ? chives\n"
        val list = parseShoppingList(input)
        val recipe = assertIs<ShoppingListItem.Recipe>(list.items.single())
        assertEquals(ShoppingListItem.Ingredient("chives", null, true), recipe.children.single())
        assertEquals(input, writeShoppingList(list))
    }

    @Test
    fun parsesCheckedEntries() {
        assertEquals(
            listOf(
                CheckEntry.Checked("salt"),
                CheckEntry.Checked("pepper"),
                CheckEntry.Unchecked("salt"),
                CheckEntry.Checked("garlic"),
            ),
            parseShoppingChecked("+ salt\n+ pepper\n- salt\n+ garlic\n"),
        )
    }

    @Test
    fun computesCheckedSet() {
        val entries = listOf(CheckEntry.Checked("salt"), CheckEntry.Checked("pepper"), CheckEntry.Unchecked("salt"))
        assertEquals(setOf("pepper"), shoppingCheckedSet(entries).toSet())
    }

    @Test
    fun writesCheckEntries() {
        assertEquals("+ salt\n", writeShoppingCheckEntry(CheckEntry.Checked("salt")))
        assertEquals("- pepper\n", writeShoppingCheckEntry(CheckEntry.Unchecked("pepper")))
    }

    @Test
    fun compactsCheckedEntries() {
        val entries = listOf(
            CheckEntry.Checked("salt"),
            CheckEntry.Checked("pepper"),
            CheckEntry.Checked("removed ingredient"),
        )
        assertEquals(listOf(CheckEntry.Checked("salt")), compactShoppingChecked(entries, listOf("salt")))
    }

    @Test
    fun throwsTypedExceptionOnInvalidInput() {
        // Kotlin-specific: ShoppingListError is surfaced as a checked ShoppingListException
        // (its field was renamed to avoid clashing with Exception.message).
        val error = kotlin.runCatching { parseShoppingList("  ./indented first line\n") }.exceptionOrNull()
        val parse = assertIs<ShoppingListException.Parse>(error)
        assertTrue(parse.reason.isNotEmpty())
    }
}
