package dev.nercone.firstalert.core

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** The inputs of the core (`firstalert::protocol::Input`), as JSON. `now` is the time of the device. */
object Inputs {
    private fun input(type: String, now: Long, fields: Map<String, Any?> = emptyMap()): JsonObject = buildJsonObject {
        put("type", type)
        put("now", now)
        fields.forEach { (key, value) ->
            when (value) {
                null -> put(key, kotlinx.serialization.json.JsonNull)
                is String -> put(key, value)
                is Number -> put(key, JsonPrimitive(value))
                is Boolean -> put(key, value)
                is JsonObject -> put(key, value)
                else -> error("unsupported field $key")
            }
        }
    }

    private fun source(source: Source): String = when (source) {
        Source.WOLFX -> "wolfx"
        Source.P2PQUAKE -> "p2pquake"
    }

    fun start(now: Long, dataDirectory: String, locale: String) = input("start", now, mapOf("data_dir" to dataDirectory, "locale" to locale))
    fun opened(now: Long, source: Source) = input("opened", now, mapOf("source" to source(source)))
    fun received(now: Long, source: Source, text: String) = input("received", now, mapOf("source" to source(source), "text" to text))
    fun closed(now: Long, source: Source, reason: String) = input("closed", now, mapOf("source" to source(source), "reason" to reason))
    fun responded(now: Long, id: Long, status: Int, body: String) = input("responded", now, mapOf("id" to id, "status" to status, "body" to body))
    fun tick(now: Long) = input("tick", now)
    fun networkAvailable(now: Long) = input("network_available", now)
    fun configure(now: Long, settings: Settings) = input("configure", now, mapOf("settings" to CoreJson.json.encodeToJsonElement(Settings.serializer(), settings) as JsonObject))
    fun located(now: Long, coordinate: Coordinate) = input("located", now, mapOf("coordinate" to CoreJson.json.encodeToJsonElement(Coordinate.serializer(), coordinate) as JsonObject))
    fun locale(now: Long, locale: String) = input("locale", now, mapOf("locale" to locale))
    fun snapshot(now: Long) = input("snapshot", now)
    fun detail(now: Long, key: String) = input("detail", now, mapOf("key" to key))
    fun waves(now: Long) = input("waves", now)
    fun map(now: Long) = input("map", now)
    fun areas(now: Long, prefecture: Int) = input("areas", now, mapOf("prefecture" to prefecture))
    fun acknowledge(now: Long, key: String) = input("acknowledge", now, mapOf("key" to key))
    fun test(now: Long, level: AlertLevel) = input("test", now, mapOf("level" to level.name.lowercase()))
    fun stop(now: Long) = input("stop", now)
}
