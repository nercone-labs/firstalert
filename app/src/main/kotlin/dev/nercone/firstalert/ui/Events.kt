package dev.nercone.firstalert.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Campaign
import androidx.compose.material.icons.filled.Tsunami
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.nercone.firstalert.R
import dev.nercone.firstalert.core.AlertLevel
import dev.nercone.firstalert.core.EarthquakeEvent
import dev.nercone.firstalert.core.Snapshot
import dev.nercone.firstalert.core.TsunamiGrade
import dev.nercone.firstalert.core.TsunamiState
import kotlinx.coroutines.delay
import kotlin.math.ceil

/** The parts that show earthquakes and tsunamis, shared by the screens. */
object Events {
    /** The time of the servers, ticking every `interval` ms: the clock of countdowns. */
    @Composable
    fun rememberNow(offset: Long, interval: Long): Long {
        var now by remember { mutableLongStateOf(System.currentTimeMillis() + offset) }
        LaunchedEffect(offset, interval) {
            while (true) {
                now = System.currentTimeMillis() + offset
                delay(interval)
            }
        }
        return now
    }

    /** The badge of the strongest intensity of an event, or a dash when none is known. */
    @Composable
    fun Badge(event: EarthquakeEvent, snapshot: Snapshot?, size: androidx.compose.ui.unit.Dp = 52.dp) {
        val intensity = event.maxIntensity
        if (intensity != null) {
            Intensities.Badge(intensity, Labels.intensityShort(snapshot, intensity), Labels.intensity(snapshot, intensity), size, if (size > 60.dp) 30.sp else 20.sp)
        } else {
            Surface(Modifier.size(size), shape = MaterialTheme.shapes.medium, color = MaterialTheme.colorScheme.surfaceContainerHighest) {
                Column(verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) { Text("–", style = MaterialTheme.typography.titleLarge) }
            }
        }
    }

    /** One earthquake in a list: the intensity, where, when, and what the latest information is. */
    @Composable
    fun Row(event: EarthquakeEvent, snapshot: Snapshot?, index: Int, count: Int, onClick: () -> Unit) {
        Widgets.Panel(index, count, onClick = onClick) {
            Row(horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.CenterVertically) {
                Badge(event, snapshot)
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(event.text.title, style = MaterialTheme.typography.titleMedium)
                    if (event.text.subtitle.isNotEmpty()) Text(event.text.subtitle, style = MaterialTheme.typography.bodyMedium, color = Widgets.secondary)
                    Text(listOfNotNull(event.text.status, event.text.intensity).joinToString(" · "), style = MaterialTheme.typography.bodySmall, color = Widgets.secondary)
                    event.text.local?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary) }
                }
            }
        }
    }

    /**
     * An early warning in progress: the level as a band of its colour, what to expect at the
     * user's place in large type, and the countdown to the strong shaking. It is announced to
     * screen readers as it changes.
     */
    @Composable
    fun Active(event: EarthquakeEvent, snapshot: Snapshot, actions: String?) {
        val swatch = Levels.swatch(event.level.coerceAtLeast(AlertLevel.INFORMATION))
        val now = rememberNow(snapshot.clockOffsetMs, 200)
        Surface(Modifier.fillMaxWidth(), shape = MaterialTheme.shapes.extraLarge, color = swatch.background, contentColor = swatch.content) {
            Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Icon(Icons.Default.Campaign, null, Modifier.size(32.dp))
                    Text(event.text.status, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold, modifier = Modifier.semantics { heading() })
                }
                Text(event.text.title, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
                val local = event.local
                val expected = local?.expected
                if (expected != null && snapshot.place != null) {
                    Surface(shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surface, contentColor = MaterialTheme.colorScheme.onSurface) {
                        Row(Modifier.fillMaxWidth().padding(16.dp), horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.CenterVertically) {
                            Intensities.Badge(expected, Labels.intensityShort(snapshot, expected), Labels.intensity(snapshot, expected), 84.dp, 36.sp)
                            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Text(stringResource(R.string.active_your_place, snapshot.place.label), style = MaterialTheme.typography.titleMedium)
                                Text(stringResource(R.string.active_expected, Labels.intensity(snapshot, expected)), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
                                Labels.intensityFeel(snapshot, expected)?.let { Text(it, style = MaterialTheme.typography.bodyMedium) }
                            }
                        }
                    }
                }
                Countdown(local?.arrival, local?.forecast?.arrived == true, now)
                event.text.intensity?.let { Text(it, style = MaterialTheme.typography.bodyLarge) }
                if (event.text.subtitle.isNotEmpty()) Text(event.text.subtitle, style = MaterialTheme.typography.bodyMedium)
                actions?.let {
                    Text(it, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                }
            }
        }
    }

    /** The seconds to the strong shaking, large; or that it has arrived. A guide only. */
    @Composable
    fun Countdown(arrival: Long?, arrived: Boolean, now: Long) {
        if (arrival == null && !arrived) return
        val seconds = arrival?.let { ceil((it - now) / 1000.0).toInt() }
        val text = when {
            arrived || seconds == null || seconds <= 0 -> stringResource(R.string.countdown_arrived)
            else -> stringResource(R.string.countdown_seconds, seconds)
        }
        // Announced every few seconds, not every tick, so that a screen reader can keep up.
        var announced by remember { mutableStateOf(text) }
        LaunchedEffect(seconds) { if (seconds == null || seconds <= 0 || seconds % 5 == 0) announced = text }
        Column(Modifier.clearAndSetSemantics { contentDescription = announced; liveRegion = LiveRegionMode.Assertive }) {
            Text(text, fontSize = 40.sp, fontWeight = FontWeight.Black, lineHeight = 44.sp)
            Text(stringResource(R.string.countdown_note), style = MaterialTheme.typography.bodySmall)
        }
    }

    /** The tsunami warnings in force: the grades, worst first, and the areas. */
    @Composable
    fun Tsunami(state: TsunamiState) {
        val grade = state.local?.takeIf { it >= TsunamiGrade.ADVISORY } ?: state.highest ?: TsunamiGrade.UNKNOWN
        val level = when {
            state.local == TsunamiGrade.MAJOR_WARNING || state.local == TsunamiGrade.WARNING -> AlertLevel.EMERGENCY
            grade >= TsunamiGrade.ADVISORY -> AlertLevel.WARNING
            else -> AlertLevel.INFORMATION
        }
        val swatch = Levels.swatch(level)
        Surface(Modifier.fillMaxWidth(), shape = MaterialTheme.shapes.extraLarge, color = swatch.background, contentColor = swatch.content) {
            Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Icon(Icons.Default.Tsunami, null, Modifier.size(32.dp))
                    Text(Labels.tsunami(grade), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold, modifier = Modifier.semantics { heading() })
                }
                if (state.local != null && state.local >= TsunamiGrade.ADVISORY) {
                    Text(stringResource(R.string.tsunami_your_coast), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                }
                for (g in listOf(TsunamiGrade.MAJOR_WARNING, TsunamiGrade.WARNING, TsunamiGrade.ADVISORY)) {
                    val areas = state.areas.filter { it.grade == g }
                    if (areas.isEmpty()) continue
                    Text(Labels.tsunami(g), style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Bold)
                    Text(areas.joinToString("、") { a -> listOfNotNull(a.name, a.maxHeight).joinToString(" ") }, style = MaterialTheme.typography.bodyMedium)
                }
                if (grade >= TsunamiGrade.WARNING) Text(stringResource(R.string.tsunami_action_warning), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                else if (grade == TsunamiGrade.ADVISORY) Text(stringResource(R.string.tsunami_action_advisory), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                state.issued?.let { Text(stringResource(R.string.issued_at, Labels.time(it)), style = MaterialTheme.typography.bodySmall) }
            }
        }
    }
}
