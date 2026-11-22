import org.gradle.internal.os.OperatingSystem

// JVM test harness for the UniFFI-generated Kotlin bindings.
//
// `./gradlew test` builds the host cdylib with cargo, generates Kotlin from it
// with uniffi-bindgen, compiles that alongside the tests, and runs them through
// JNA against the freshly built library.

plugins {
    kotlin("jvm") version "2.4.21"
}

val bindingsDir: File = projectDir.parentFile
val targetDir: File = bindingsDir.parentFile.resolve("target/debug")
val generatedDir = layout.buildDirectory.dir("generated/uniffi")

val libFileName: String = when {
    OperatingSystem.current().isMacOsX -> "libcooklang_bindings.dylib"
    OperatingSystem.current().isWindows -> "cooklang_bindings.dll"
    else -> "libcooklang_bindings.so"
}

dependencies {
    implementation("net.java.dev.jna:jna:5.17.0")
    testImplementation(kotlin("test"))
}

val cargoBuild = tasks.register<Exec>("cargoBuild") {
    description = "Builds the host cooklang_bindings library."
    workingDir = bindingsDir
    commandLine("cargo", "build", "--lib")
    // cargo does its own up-to-date checking.
    doNotTrackState("cargo manages its own incremental build")
}

val generateBindings = tasks.register<Exec>("generateBindings") {
    description = "Generates Kotlin sources from the host library with uniffi-bindgen."
    dependsOn(cargoBuild)
    workingDir = bindingsDir
    outputs.dir(generatedDir)
    doFirst { generatedDir.get().asFile.deleteRecursively() }
    commandLine(
        "cargo", "run", "--quiet", "--features=uniffi/cli", "--bin", "uniffi-bindgen", "--",
        "generate",
        "--no-format",
        "--config", projectDir.resolve("uniffi-jvm.toml").absolutePath,
        "--library", targetDir.resolve(libFileName).absolutePath,
        "--language", "kotlin",
        "--out-dir", generatedDir.get().asFile.absolutePath,
    )
    doNotTrackState("depends on the Rust sources, which Gradle does not track")
}

sourceSets.main {
    kotlin.srcDir(generatedDir)
}

tasks.compileKotlin {
    dependsOn(generateBindings)
}

tasks.test {
    useJUnitPlatform()
    systemProperty("jna.library.path", targetDir.absolutePath)
    testLogging {
        events("passed", "skipped", "failed")
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}
