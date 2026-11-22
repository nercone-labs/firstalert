package dev.nercone.firstalert.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

// The types of `firstalert::protocol` as the core writes them in JSON. Times are milliseconds
// since the epoch. The core and the application are always built together, but unknown keys are
// ignored all the same, so that a field added to the core never breaks the screens.

object CoreJson {
    val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
        classDiscriminator = "type"
    }
}

@Serializable
enum class AlertLevel {
    @SerialName("none") NONE,
    @SerialName("information") INFORMATION,
    @SerialName("warning") WARNING,
    @SerialName("emergency") EMERGENCY,
}

@Serializable
enum class Source {
    @SerialName("wolfx") WOLFX,
    @SerialName("p2pquake") P2PQUAKE,
}

@Serializable
enum class Language {
    @SerialName("japanese") JAPANESE,
    @SerialName("easy_japanese") EASY_JAPANESE,
    @SerialName("english") ENGLISH,
}

@Serializable
enum class PlaceSource {
    @SerialName("prefecture") PREFECTURE,
    @SerialName("point") POINT,
    @SerialName("device") DEVICE,
}

@Serializable
enum class Ground {
    @SerialName("firm") FIRM,
    @SerialName("average") AVERAGE,
    @SerialName("soft") SOFT,
}

/** A class of the JMA seismic intensity scale, by the code the core writes (`5-`, `6+`, …). */
@Serializable
enum class Intensity(val code: String) {
    @SerialName("0") ZERO("0"),
    @SerialName("1") ONE("1"),
    @SerialName("2") TWO("2"),
    @SerialName("3") THREE("3"),
    @SerialName("4") FOUR("4"),
    @SerialName("5-") FIVE_LOWER("5-"),
    @SerialName("5+") FIVE_UPPER("5+"),
    @SerialName("6-") SIX_LOWER("6-"),
    @SerialName("6+") SIX_UPPER("6+"),
    @SerialName("7") SEVEN("7"),
}

@Serializable
enum class TsunamiGrade {
    @SerialName("unknown") UNKNOWN,
    @SerialName("forecast") FORECAST,
    @SerialName("advisory") ADVISORY,
    @SerialName("warning") WARNING,
    @SerialName("major_warning") MAJOR_WARNING,
}

@Serializable
data class Coordinate(val latitude: Double, val longitude: Double)

@Serializable
data class Place(val source: PlaceSource, val coordinate: Coordinate, val prefecture: Int? = null, val area: String? = null)

@Serializable
data class Sources(val wolfx: Boolean = true, val p2pquake: Boolean = true)

@Serializable
data class Thresholds(
    @SerialName("eew_emergency") val eewEmergency: Intensity = Intensity.FOUR,
    @SerialName("eew_local") val eewLocal: Intensity = Intensity.THREE,
    @SerialName("eew_anywhere") val eewAnywhere: Intensity = Intensity.FIVE_LOWER,
    @SerialName("report_anywhere") val reportAnywhere: Intensity = Intensity.THREE,
    @SerialName("report_local") val reportLocal: Intensity = Intensity.THREE,
)

@Serializable
data class Presentation(
    val sound: Boolean = true,
    @SerialName("sound_when_silenced") val soundWhenSilenced: Boolean = true,
    val voice: Boolean = true,
    val vibration: Boolean = true,
    val flash: Boolean = false,
    @SerialName("full_screen") val fullScreen: Boolean = true,
)

@Serializable
data class Settings(
    val version: Int = 1,
    val language: Language? = null,
    val place: Place? = null,
    val ground: Ground = Ground.AVERAGE,
    val sources: Sources = Sources(),
    val thresholds: Thresholds = Thresholds(),
    val tsunami: Boolean = true,
    val special: Boolean = true,
    val training: Boolean = false,
    val presentation: Presentation = Presentation(),
)

@Serializable
data class IntensityRange(val lower: Intensity, val upper: Intensity? = null) {
    /** The class to show: the upper bound, or the lower bound of an open range. */
    val strongest: Intensity get() = upper?.let { maxOf(it, lower) } ?: lower
}

@Serializable
data class Hypocentre(val name: String? = null, val epicentre: Coordinate? = null, @SerialName("depth_km") val depthKm: Double? = null, val magnitude: Double? = null)

@Serializable
data class EewArea(
    val name: String,
    val prefecture: Int? = null,
    val intensity: IntensityRange? = null,
    val arrival: Long? = null,
    val warning: Boolean = false,
    val arrived: Boolean = false,
)

@Serializable
data class EewState(
    val serial: Int,
    val announced: Long? = null,
    val warning: Boolean = false,
    @SerialName("ever_warning") val everWarning: Boolean = false,
    @SerialName("final_report") val finalReport: Boolean = false,
    val cancelled: Boolean = false,
    val training: Boolean = false,
    val assumption: Boolean = false,
    @SerialName("max_intensity") val maxIntensity: IntensityRange? = null,
    val areas: List<EewArea> = emptyList(),
    val sources: List<Source> = emptyList(),
    val received: Long,
)

@Serializable
data class ReportState(
    val kind: String,
    val issued: Long? = null,
    @SerialName("max_intensity") val maxIntensity: Intensity? = null,
    val tsunami: String,
    val comment: String? = null,
    val sources: List<Source> = emptyList(),
    val received: Long,
)

@Serializable
data class PrefectureIntensity(val prefecture: Int, val intensity: Intensity, val forecast: Boolean)

@Serializable
data class EstimatedShaking(
    val measured: Double,
    val intensity: Intensity,
    @SerialName("epicentral_distance_km") val epicentralDistanceKm: Double,
    @SerialName("hypocentral_distance_km") val hypocentralDistanceKm: Double,
    @SerialName("p_arrival") val pArrival: Long,
    @SerialName("s_arrival") val sArrival: Long,
)

@Serializable
data class AreaForecast(val area: String, val intensity: IntensityRange? = null, val arrival: Long? = null, val warning: Boolean = false, val arrived: Boolean = false)

@Serializable
data class LocalImpact(
    val estimate: EstimatedShaking? = null,
    val forecast: AreaForecast? = null,
    val observed: Intensity? = null,
    val expected: Intensity? = null,
    val arrival: Long? = null,
)

@Serializable
data class EventText(val title: String = "", val subtitle: String = "", val intensity: String? = null, val status: String = "", val local: String? = null)

@Serializable
data class EarthquakeEvent(
    val key: String,
    @SerialName("event_id") val eventId: String? = null,
    val origin: Long? = null,
    val hypocentre: Hypocentre = Hypocentre(),
    val eew: EewState? = null,
    val report: ReportState? = null,
    val prefectures: List<PrefectureIntensity> = emptyList(),
    val local: LocalImpact? = null,
    val level: AlertLevel = AlertLevel.NONE,
    @SerialName("first_seen") val firstSeen: Long,
    val updated: Long,
    val text: EventText = EventText(),
) {
    /** The strongest intensity known: observed, or else forecast. */
    val maxIntensity: Intensity? get() = report?.maxIntensity ?: eew?.maxIntensity?.strongest
}

@Serializable
data class Observation(val prefecture: Int? = null, val name: String, val intensity: Intensity, @SerialName("is_area") val isArea: Boolean = false, val estimated: Boolean = false)

@Serializable
data class EventDetail(val event: EarthquakeEvent, val observations: List<Observation> = emptyList(), val title: String = "", val body: String = "", val actions: String? = null)

@Serializable
data class TsunamiArea(
    val name: String,
    val grade: TsunamiGrade,
    val prefectures: List<Int> = emptyList(),
    val immediate: Boolean = false,
    @SerialName("first_arrival") val firstArrival: Long? = null,
    @SerialName("first_condition") val firstCondition: String? = null,
    @SerialName("max_height") val maxHeight: String? = null,
    @SerialName("max_height_m") val maxHeightM: Double? = null,
)

@Serializable
data class TsunamiState(
    val issued: Long? = null,
    val received: Long,
    val lifted: Boolean = false,
    val areas: List<TsunamiArea> = emptyList(),
    val highest: TsunamiGrade? = null,
    val local: TsunamiGrade? = null,
    val comment: String? = null,
    val sources: List<Source> = emptyList(),
) {
    val inForce: Boolean get() = !lifted && (highest == TsunamiGrade.ADVISORY || highest == TsunamiGrade.WARNING || highest == TsunamiGrade.MAJOR_WARNING)
}

@Serializable
data class SpecialEntry(
    val id: String,
    val product: String,
    val extraordinary: Boolean = false,
    val time: Long? = null,
    val received: Long,
    val cancelled: Boolean = false,
    val title: String? = null,
    val label: String? = null,
    val text: String? = null,
)

@Serializable
enum class ConnectionState {
    @SerialName("disabled") DISABLED,
    @SerialName("waiting") WAITING,
    @SerialName("connecting") CONNECTING,
    @SerialName("open") OPEN,
}

@Serializable
data class ConnectionStatus(
    val source: Source,
    val enabled: Boolean,
    val state: ConnectionState,
    val since: Long? = null,
    @SerialName("last_message") val lastMessage: Long? = null,
    val failures: Int = 0,
)

@Serializable
data class PlaceSummary(val place: Place, val label: String)

@Serializable
data class IntensityLabel(val intensity: Intensity, val name: String, val short: String, val description: String)

@Serializable
data class Snapshot(
    val now: Long,
    @SerialName("clock_offset_ms") val clockOffsetMs: Long = 0,
    val language: Language,
    val settings: Settings,
    val connections: List<ConnectionStatus> = emptyList(),
    val earthquakes: List<EarthquakeEvent> = emptyList(),
    val tsunami: TsunamiState? = null,
    val special: List<SpecialEntry> = emptyList(),
    val active: List<String> = emptyList(),
    val level: AlertLevel = AlertLevel.NONE,
    val place: PlaceSummary? = null,
    val intensities: List<IntensityLabel> = emptyList(),
)

@Serializable
enum class Category {
    @SerialName("eew") EEW,
    @SerialName("earthquake") EARTHQUAKE,
    @SerialName("tsunami") TSUNAMI,
    @SerialName("special") SPECIAL,
    @SerialName("test") TEST,
}

@Serializable
enum class Pattern {
    @SerialName("emergency") EMERGENCY,
    @SerialName("warning") WARNING,
    @SerialName("information") INFORMATION,
}

@Serializable
data class Notification(
    val key: String,
    val category: Category,
    val level: AlertLevel,
    val alerting: Boolean,
    val title: String,
    val body: String,
    val actions: String? = null,
    val speech: String? = null,
    val intensity: Intensity? = null,
    @SerialName("intensity_label") val intensityLabel: String? = null,
    val local: Boolean = false,
    val tsunami: TsunamiGrade? = null,
    val arrival: Long? = null,
    val time: Long,
    val sound: Pattern? = null,
    @SerialName("sound_when_silenced") val soundWhenSilenced: Boolean = false,
    val voice: Boolean = false,
    val vibration: Pattern? = null,
    val flash: Boolean = false,
    @SerialName("full_screen") val fullScreen: Boolean = false,
    val ongoing: Boolean = false,
    val subject: String? = null,
)

@Serializable
enum class LogLevel {
    @SerialName("info") INFO,
    @SerialName("warning") WARNING,
    @SerialName("error") ERROR,
}

/** Something the core asks the platform to do (`firstalert::protocol::Effect`). */
@Serializable
sealed interface Effect {
    @Serializable @SerialName("connect")
    data class Connect(val source: Source, val url: String, @SerialName("ping_interval_ms") val pingIntervalMs: Long? = null) : Effect

    @Serializable @SerialName("send")
    data class Send(val source: Source, val text: String) : Effect

    @Serializable @SerialName("disconnect")
    data class Disconnect(val source: Source) : Effect

    @Serializable @SerialName("request")
    data class Request(val id: Long, val url: String) : Effect

    @Serializable @SerialName("notify")
    data class Notify(val notification: Notification) : Effect

    @Serializable @SerialName("withdraw")
    data class Withdraw(val key: String) : Effect

    @Serializable @SerialName("silence")
    data class Silence(val key: String) : Effect

    @Serializable @SerialName("wake")
    data class Wake(val at: Long) : Effect

    @Serializable @SerialName("log")
    data class Log(val level: LogLevel, val message: String) : Effect
}

@Serializable
data class Waves(val key: String, val epicentre: Coordinate, @SerialName("p_radius_km") val pRadiusKm: Double? = null, @SerialName("s_radius_km") val sRadiusKm: Double? = null)

@Serializable
data class MapShape(val prefecture: Int, val name: String, val rings: List<List<List<Double>>>)

@Serializable
data class Response(
    val effects: List<Effect> = emptyList(),
    val snapshot: Snapshot? = null,
    val detail: EventDetail? = null,
    val waves: List<Waves>? = null,
    val map: List<MapShape>? = null,
    val areas: PrefectureAreas? = null,
    val error: String? = null,
)

@Serializable
data class PrefectureAreas(val prefecture: Int, val name: String, val representative: Coordinate? = null, val areas: List<String> = emptyList())
