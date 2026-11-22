package dev.nercone.firstalert.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import dev.nercone.firstalert.R
import dev.nercone.firstalert.core.AlertLevel
import dev.nercone.firstalert.core.ConnectionState
import dev.nercone.firstalert.core.Ground
import dev.nercone.firstalert.core.Intensity
import dev.nercone.firstalert.core.Language
import dev.nercone.firstalert.core.PlaceSource
import dev.nercone.firstalert.core.Snapshot
import dev.nercone.firstalert.core.Source
import dev.nercone.firstalert.core.TsunamiGrade
import java.text.DateFormat
import java.util.Date
import java.util.TimeZone

/** The names of the values of the core in the language of the screens. */
object Labels {
    /** The name of an intensity as the core words it (`震度5弱`), or the code without a snapshot. */
    fun intensity(snapshot: Snapshot?, intensity: Intensity): String = snapshot?.intensities?.firstOrNull { it.intensity == intensity }?.name ?: intensity.code

    fun intensityShort(snapshot: Snapshot?, intensity: Intensity): String = snapshot?.intensities?.firstOrNull { it.intensity == intensity }?.short ?: intensity.code

    fun intensityFeel(snapshot: Snapshot?, intensity: Intensity): String? = snapshot?.intensities?.firstOrNull { it.intensity == intensity }?.description

    @Composable
    fun level(level: AlertLevel): String = stringResource(
        when (level) {
            AlertLevel.EMERGENCY -> R.string.level_emergency
            AlertLevel.WARNING -> R.string.level_warning
            AlertLevel.INFORMATION -> R.string.level_information
            AlertLevel.NONE -> R.string.level_none
        },
    )

    @Composable
    fun source(source: Source): String = stringResource(if (source == Source.WOLFX) R.string.source_wolfx else R.string.source_p2pquake)

    @Composable
    fun connection(state: ConnectionState): String = stringResource(
        when (state) {
            ConnectionState.OPEN -> R.string.connection_open
            ConnectionState.CONNECTING -> R.string.connection_connecting
            ConnectionState.WAITING -> R.string.connection_waiting
            ConnectionState.DISABLED -> R.string.connection_disabled
        },
    )

    @Composable
    fun ground(ground: Ground): String = stringResource(
        when (ground) {
            Ground.FIRM -> R.string.ground_firm
            Ground.AVERAGE -> R.string.ground_average
            Ground.SOFT -> R.string.ground_soft
        },
    )

    @Composable
    fun groundHelp(ground: Ground): String = stringResource(
        when (ground) {
            Ground.FIRM -> R.string.ground_firm_help
            Ground.AVERAGE -> R.string.ground_average_help
            Ground.SOFT -> R.string.ground_soft_help
        },
    )

    @Composable
    fun language(language: Language?): String = stringResource(
        when (language) {
            null -> R.string.language_system
            Language.JAPANESE -> R.string.language_japanese
            Language.EASY_JAPANESE -> R.string.language_easy_japanese
            Language.ENGLISH -> R.string.language_english
        },
    )

    @Composable
    fun placeSource(source: PlaceSource): String = stringResource(
        when (source) {
            PlaceSource.PREFECTURE -> R.string.place_by_prefecture
            PlaceSource.POINT -> R.string.place_by_point
            PlaceSource.DEVICE -> R.string.place_by_device
        },
    )

    @Composable
    fun tsunami(grade: TsunamiGrade): String = stringResource(
        when (grade) {
            TsunamiGrade.MAJOR_WARNING -> R.string.tsunami_major_warning
            TsunamiGrade.WARNING -> R.string.tsunami_warning
            TsunamiGrade.ADVISORY -> R.string.tsunami_advisory
            TsunamiGrade.FORECAST -> R.string.tsunami_forecast
            TsunamiGrade.UNKNOWN -> R.string.tsunami_unknown
        },
    )

    /** A time in Japan Standard Time, the time of every report, with the date when it is not today. */
    fun time(millis: Long): String {
        val format = DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT)
        format.timeZone = TimeZone.getTimeZone("Asia/Tokyo")
        return format.format(Date(millis))
    }
}
