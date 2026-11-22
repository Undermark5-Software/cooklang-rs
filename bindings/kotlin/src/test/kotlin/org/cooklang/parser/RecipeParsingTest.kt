package org.cooklang.parser

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class RecipeParsingTest {

    /** Same recipe as swift/Tests/CooklangParserTests/Demo.swift, against the current API. */
    @Test
    fun parsesDemoRecipe() {
        val input = """
            Preheat the oven to 180C.

            Mix @flour{2%cups} with @baking powder{1%tsp}.

            Add @eggs{2} and stir for ~{2%minutes}.
        """.trimIndent()

        parseRecipe(input, 1.0).use { recipe ->
            val sections = recipe.sections()
            assertEquals(1, sections.size)
            assertNull(sections[0].title)

            val steps = sections[0].blocks.map { assertIs<Block.StepBlock>(it).v1.items }
            assertEquals(
                listOf(
                    listOf(Item.Text("Preheat the oven to 180C.")),
                    listOf(
                        Item.Text("Mix "),
                        Item.IngredientRef(0u),
                        Item.Text(" with "),
                        Item.IngredientRef(1u),
                        Item.Text("."),
                    ),
                    listOf(
                        Item.Text("Add "),
                        Item.IngredientRef(2u),
                        Item.Text(" and stir for "),
                        Item.TimerRef(0u),
                        Item.Text("."),
                    ),
                ),
                steps,
            )

            assertEquals(
                listOf(
                    ingredient("flour", Amount(Value.Number(2.0), "cups")),
                    ingredient("baking powder", Amount(Value.Number(1.0), "tsp")),
                    ingredient("eggs", Amount(Value.Number(2.0), null)),
                ),
                recipe.ingredients(),
            )
            assertEquals(listOf(Timer(UNNAMED_TIMER, Amount(Value.Number(2.0), "minutes"))), recipe.timers())
            assertEquals(emptyList(), recipe.cookware())
        }
    }

    @Test
    fun tracksComponentRefsPerStepAndSection() {
        parseRecipe("Put @butter{1%tbsp} in a #pan{}.\n\nFry @eggs{2} for ~{3%minutes}.", 1.0).use { recipe ->
            val section = recipe.sections().single()
            assertEquals(listOf(0u, 1u), section.ingredientRefs)
            assertEquals(listOf(0u), section.cookwareRefs)
            assertEquals(listOf(0u), section.timerRefs)

            val (first, second) = section.blocks.map { assertIs<Block.StepBlock>(it).v1 }
            assertEquals(listOf(0u), first.ingredientRefs)
            assertEquals(listOf(0u), first.cookwareRefs)
            assertEquals(emptyList(), first.timerRefs)
            assertEquals(listOf(1u), second.ingredientRefs)
            assertEquals(listOf(0u), second.timerRefs)
        }
    }

    @Test
    fun derefsComponents() {
        parseRecipe("Put @butter{1%tbsp} in a #pan{} for ~{3%minutes}.", 1.0).use { recipe ->
            val items = assertIs<Block.StepBlock>(recipe.sections()[0].blocks[0]).v1.items

            assertEquals(
                listOf(
                    Component.TextComponent("Put "),
                    Component.IngredientComponent(ingredient("butter", Amount(Value.Number(1.0), "tbsp"))),
                    Component.TextComponent(" in a "),
                    Component.CookwareComponent(Cookware("pan", null, false)),
                    Component.TextComponent(" for "),
                    Component.TimerComponent(Timer(UNNAMED_TIMER, Amount(Value.Number(3.0), "minutes"))),
                    Component.TextComponent("."),
                ),
                items.map { derefComponent(recipe, it) },
            )

            assertEquals("butter", derefIngredient(recipe, 0u).name)
            assertEquals("pan", derefCookware(recipe, 0u).name)
            assertEquals(Amount(Value.Number(3.0), "minutes"), derefTimer(recipe, 0u).amount)
        }
    }

    @Test
    fun parsesNotes() {
        val input = """
            > This dish is even better the next day.

            Cook @onions{3%large} until brown
        """.trimIndent()

        parseRecipe(input, 1.0).use { recipe ->
            val blocks = recipe.sections()[0].blocks
            assertEquals(2, blocks.size)
            assertEquals("This dish is even better the next day.", assertIs<Block.NoteBlock>(blocks[0]).v1.text)
            assertEquals(
                listOf(Item.Text("Cook "), Item.IngredientRef(0u), Item.Text(" until brown")),
                assertIs<Block.StepBlock>(blocks[1]).v1.items,
            )
        }
    }

    @Test
    fun joinsMultilineSteps() {
        parseRecipe("add @onions{2} to pan\nheat until golden", 1.0).use { recipe ->
            assertEquals(
                listOf(Item.Text("add "), Item.IngredientRef(0u), Item.Text(" to pan heat until golden")),
                assertIs<Block.StepBlock>(recipe.sections()[0].blocks.single()).v1.items,
            )
        }
    }

    @Test
    fun parsesSections() {
        val input = """
            = Dough

            Mix @flour{200%g} and @water{50%ml}.

            == Filling ==

            Combine @cheese{100%g} and @spinach{50%g}.
        """.trimIndent()

        parseRecipe(input, 1.0).use { recipe ->
            val sections = recipe.sections()
            assertEquals(listOf("Dough", "Filling"), sections.map { it.title })
            assertEquals(listOf(0u, 1u), sections[0].ingredientRefs)
            assertEquals(listOf(2u, 3u), sections[1].ingredientRefs)
            assertEquals(
                listOf("flour", "water", "cheese", "spinach"),
                recipe.ingredients().map { it.name },
            )
        }
    }

    @Test
    fun parsesDescriptors() {
        parseRecipe("Add @onion{1}(finely chopped).", 1.0).use { recipe ->
            assertEquals("finely chopped", recipe.ingredients().single().descriptor)
        }
    }

    @Test
    fun parsesRecipeReferences() {
        parseRecipe("Serve the @./pasta/spaghetti{1%portion} with sauce", 1.0).use { recipe ->
            val ingredient = recipe.ingredients().single()
            assertEquals("spaghetti", ingredient.name)
            assertEquals(RecipeReference("spaghetti", listOf(".", "pasta")), ingredient.reference)
        }
    }

    @Test
    fun parsesOptionalComponents() {
        parseRecipe("Top with @?chives and @salt, use a #?splatter guard{}.", 1.0).use { recipe ->
            assertEquals(
                listOf("chives" to true, "salt" to false),
                recipe.ingredients().map { it.name to it.optional },
            )
            assertTrue(recipe.cookware().single().optional)
        }
    }

    @Test
    fun parsesValueKinds() {
        parseRecipe("Use @a{1-2%cups}, @b{some%pinch}, @c{} and @d.", 1.0).use { recipe ->
            val amounts = recipe.ingredients().associate { it.name to it.amount }
            // parseRecipe uses the canonical parser, which has the RANGE_VALUES extension
            // disabled, so "1-2" stays text. Value.Range only comes from parseValue().
            assertEquals(Amount(Value.Text("1-2"), "cups"), amounts["a"])
            assertEquals(Amount(Value.Text("some"), "pinch"), amounts["b"])
            assertNull(amounts["c"])
            assertNull(amounts["d"])
        }
    }

    @Test
    fun appliesScalingFactor() {
        val input = "Mix @flour{200%g}, @eggs{2} and @milk{1/2%cups}. Bake for ~{30%minutes}."

        parseRecipe(input, 2.0).use { recipe ->
            val amounts = recipe.ingredients().associate { it.name to it.amount }
            assertEquals(Amount(Value.Number(400.0), "g"), amounts["flour"])
            assertEquals(Amount(Value.Number(4.0), null), amounts["eggs"])
            assertEquals(Amount(Value.Number(1.0), "cups"), amounts["milk"])
            // Timers are not scaled.
            assertEquals(Amount(Value.Number(30.0), "minutes"), recipe.timers().single().amount)
        }
    }

    @Test
    fun parsesEmptyRecipe() {
        parseRecipe("", 1.0).use { recipe ->
            assertTrue(recipe.ingredients().isEmpty())
            assertTrue(recipe.sections().all { it.blocks.isEmpty() })
        }
    }
}

/**
 * Unnamed timers (`~{2%minutes}`) currently come back with an empty name rather than null:
 * bindings/src/model.rs converts with `Some(name.unwrap_or_default())`. Pinned here so a
 * change is deliberate; update this if that conversion is fixed to pass `None` through.
 */
internal const val UNNAMED_TIMER = ""

internal fun ingredient(name: String, amount: Amount?) =
    Ingredient(name = name, amount = amount, descriptor = null, reference = null, optional = false)
