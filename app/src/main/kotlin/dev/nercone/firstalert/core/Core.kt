package dev.nercone.firstalert.core

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/**
 * One instance of the Rust core. Every call runs on one thread (`dispatcher`), in order, so the
 * core, which is not thread-safe, is never entered twice at once.
 */
class Core(private val dispatcher: CoroutineDispatcher = Dispatchers.IO.limitedParallelism(1)) {
    private val handle: Long = NativeCore.create()

    /** Hands `input` to the core. A response that cannot be read becomes an error response. */
    suspend fun handle(input: JsonObject): Response = withContext(dispatcher) {
        val bytes = NativeCore.handle(handle, input.toString().toByteArray(Charsets.UTF_8))
            ?: return@withContext Response(error = "the core returned nothing")
        decode(bytes.toString(Charsets.UTF_8))
    }

    companion object {
        /**
         * Reads a response. Each effect is read on its own and the snapshot apart from them, so
         * that a part this version cannot read never costs the others: an alert is never lost
         * because a field of the snapshot could not be read.
         */
        fun decode(text: String): Response {
            val root = try {
                CoreJson.json.parseToJsonElement(text) as? JsonObject
            } catch (error: SerializationException) {
                null
            } ?: return Response(error = "unreadable response of the core")
            val problems = mutableListOf<String>()
            fun <T> part(name: String, read: () -> T?): T? = try {
                read()
            } catch (error: SerializationException) {
                problems += "$name: ${error.message}"
                null
            } catch (error: IllegalArgumentException) {
                problems += "$name: ${error.message}"
                null
            }
            val effects = (root["effects"] as? JsonArray).orEmpty().mapNotNull { element ->
                part("effect") { CoreJson.json.decodeFromJsonElement(Effect.serializer(), element) }
            }
            fun <T> field(name: String, serializer: KSerializer<T>): T? =
                root[name]?.takeIf { it !is JsonNull }?.let { element -> part(name) { CoreJson.json.decodeFromJsonElement(serializer, element) } }
            val error = (root["error"] as? JsonPrimitive)?.takeIf { it.isString }?.content
            return Response(
                effects = effects,
                snapshot = field("snapshot", Snapshot.serializer()),
                detail = field("detail", EventDetail.serializer()),
                waves = field("waves", ListSerializer(Waves.serializer())),
                map = field("map", ListSerializer(MapShape.serializer())),
                areas = field("areas", PrefectureAreas.serializer()),
                error = listOfNotNull(error, problems.takeIf { it.isNotEmpty() }?.joinToString("; ")).joinToString("; ").ifEmpty { null },
            )
        }
    }
}
