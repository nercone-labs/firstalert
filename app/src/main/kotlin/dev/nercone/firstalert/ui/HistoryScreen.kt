package dev.nercone.firstalert.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.foundation.layout.Row
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.nercone.firstalert.R
import dev.nercone.firstalert.core.Intensity

/** Every earthquake kept, newest first; the special information; the latest tsunami state. */
object HistoryScreen {
    enum class Filter(val label: Int) {
        ALL(R.string.history_all),
        FELT(R.string.history_strong),
        MINE(R.string.history_mine),
    }

    @Composable
    fun Content(data: ScreenData, navigator: FirstAlertApp.Navigator) {
        val snapshot = data.snapshot ?: return
        var filter by rememberSaveable { mutableStateOf(Filter.ALL) }
        val events = snapshot.earthquakes.filter { e ->
            when (filter) {
                Filter.ALL -> true
                Filter.FELT -> (e.maxIntensity ?: Intensity.ZERO) >= Intensity.THREE
                Filter.MINE -> (e.local?.observed ?: e.local?.expected ?: Intensity.ZERO) >= Intensity.ONE
            }
        }
        Widgets.Screen(spacing = 2.dp) {
            snapshot.tsunami?.takeIf { it.inForce }?.let { tsunami -> item(key = "tsunami") { Events.Tsunami(tsunami) } }
            item(key = "filters") {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Filter.entries.forEach { f -> FilterChip(selected = filter == f, onClick = { filter = f }, label = { Text(stringResource(f.label)) }) }
                }
            }
            if (events.isEmpty()) item(key = "empty") { Widgets.Help(stringResource(R.string.history_empty)) }
            events.forEachIndexed { index, event ->
                item(key = event.key) { Events.Row(event, snapshot, index, events.size) { navigator.open(FirstAlertApp.Destination.HISTORY, null, event.key) } }
            }
            val special = snapshot.special.filter { it.extraordinary }
            if (special.isNotEmpty()) {
                item(key = "special-header") { Widgets.Header(stringResource(R.string.history_special)) }
                special.forEachIndexed { index, entry ->
                    item(key = "special-${entry.id}") {
                        Widgets.Panel(index, special.size) {
                            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Text(listOfNotNull(entry.title, entry.label).joinToString(" "), style = MaterialTheme.typography.titleMedium)
                                (entry.time ?: entry.received).let { Text(Labels.time(it), style = MaterialTheme.typography.bodySmall, color = Widgets.secondary) }
                                entry.text?.let { Text(it.take(400), style = MaterialTheme.typography.bodyMedium) }
                            }
                        }
                    }
                }
            }
        }
    }
}
