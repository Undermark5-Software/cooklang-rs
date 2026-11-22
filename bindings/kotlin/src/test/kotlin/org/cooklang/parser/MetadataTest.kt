package org.cooklang.parser

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class MetadataTest {

    private val recipe = """
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
    """.trimIndent()

    @Test
    fun readsStandardFields() {
        parseRecipe(recipe, 1.0).use { r ->
            assertEquals("Test Recipe", metadataTitle(r))
            assertEquals("A quick test", metadataDescription(r))
            assertEquals(Servings.Number(4u), metadataServings(r))
            assertEquals(listOf("easy", "quick", "vegetarian"), metadataTags(r))
            assertEquals(NameAndUrl("John Doe", "https://johndoe.com"), metadataAuthor(r))
            assertEquals("https://google.com", metadataSource(r)?.url)
        }
    }

    @Test
    fun readsStdKeysAndCustomKeys() {
        parseRecipe(recipe, 1.0).use { r ->
            assertEquals("Italian", metadataGetStd(r, StdKey.CUISINE))
            assertEquals("Test Recipe", metadataGetStd(r, StdKey.TITLE))
            assertEquals("500 calories", metadataGet(r, "nutrition"))
            assertEquals(setOf("nutrition", "rating"), metadataCustomKeys(r).toSet())
        }
    }

    @Test
    fun readsTextServings() {
        parseRecipe("---\nservings: 2-3 portions\n---\nCook.", 1.0).use { r ->
            assertEquals(Servings.Text("2-3 portions"), metadataServings(r))
        }
    }

    @Test
    fun returnsNullForMissingMetadata() {
        parseRecipe("Just a step.", 1.0).use { r ->
            assertNull(metadataTitle(r))
            assertNull(metadataDescription(r))
            assertNull(metadataServings(r))
            assertNull(metadataTags(r))
            assertNull(metadataAuthor(r))
            assertNull(metadataSource(r))
            assertNull(metadataTime(r))
            assertNull(metadataGet(r, "nope"))
            assertNull(metadataGetStd(r, StdKey.CUISINE))
            assertEquals(emptyList(), metadataCustomKeys(r))
        }
    }
}
