package dev.nercone.firstalert.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.nercone.firstalert.R
import dev.nercone.firstalert.core.EventDetail
import dev.nercone.firstalert.core.Intensity
import java.util.Locale

/** One earthquake: what is known, what it means at the user's place, the map, every observation. */
object DetailScreen {
    @Composable
    fun Content(key: String, data: ScreenData, model: AppViewModel) {
        val snapshot = data.snapshot ?: return
        val event = snapshot.earthquakes.firstOrNull { it.key == key }
        var detail by remember { mutableStateOf<EventDetail?>(null) }
        LaunchedEffect(key, event?.updated) { detail = model.detail(key) }
        if (event == null) {
            Widgets.Screen { item { Widgets.Help(stringResource(R.string.detail_missing)) } }
            return
        }
        val place = snapshot.place?.place?.coordinate
        val fills = event.prefectures.associate { it.prefecture to it.intensity }
        val area = remember(key, data.map.size, place) { JapanMap.areaFor(event.hypocentre.epicentre, place, fills.keys, data.map) }
        Widgets.Screen {
            if (key in snapshot.active) item(key = "active") { Events.Active(event, snapshot, detail?.actions) }
            item(key = "summary") {
                Widgets.Panel(0, 1) {
                    Row(horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.CenterVertically) {
                        Events.Badge(event, snapshot, 72.dp)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(event.text.title, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
                            Text(event.text.subtitle, style = MaterialTheme.typography.bodyMedium, color = Widgets.secondary)
                            Text(event.text.status, style = MaterialTheme.typography.bodySmall, color = Widgets.secondary)
                        }
                    }
                    detail?.let { d ->
                        Text(d.body, style = MaterialTheme.typography.bodyLarge)
                        d.actions?.let { Text(it, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold) }
                    }
                }
            }
            event.local?.let { local ->
                item(key = "local") {
                    Widgets.Section(stringResource(R.string.detail_local, snapshot.place?.label ?: "")) {
                        val rows = buildList {
                            local.observed?.let { add(stringResource(R.string.detail_observed) to Labels.intensity(snapshot, it)) }
                            local.forecast?.intensity?.let { add(stringResource(R.string.detail_forecast, local.forecast.area) to Labels.intensity(snapshot, it.strongest)) }
                            local.estimate?.let { e ->
                                add(stringResource(R.string.detail_estimate) to "${Labels.intensity(snapshot, e.intensity)} (${String.format(Locale.ROOT, "%.1f", e.measured)})")
                                add(stringResource(R.string.detail_distance) to stringResource(R.string.kilometres, e.epicentralDistanceKm.toInt()))
                            }
                        }
                        Widgets.Panel(0, 1) {
                            rows.forEach { (label, value) -> Widgets.Field(label, value) }
                            Text(stringResource(R.string.detail_estimate_note), style = MaterialTheme.typography.bodySmall, color = Widgets.secondary)
                        }
                    }
                }
            }
            if (data.map.isNotEmpty()) {
                item(key = "map") {
                    Widgets.Panel(0, 1) {
                        JapanMap.Content(data.map, fills, event.hypocentre.epicentre, place, data.waves.filter { it.key == key }, area, HomeScreen.mapDescription(event, snapshot))
                        HomeScreen.Legend(fills.values.toSet(), snapshot, false)
                    }
                }
            }
            val observations = detail?.observations.orEmpty()
            if (observations.isNotEmpty()) {
                item(key = "observations-header") { Widgets.Header(stringResource(R.string.detail_observations)) }
                observations.groupBy { it.intensity }.toSortedMap(compareByDescending<Intensity> { it }).forEach { (intensity, points) ->
                    item(key = "observed-$intensity") {
                        Widgets.Panel(0, 1) {
                            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                                Intensities.Badge(intensity, Labels.intensityShort(snapshot, intensity), Labels.intensity(snapshot, intensity), 40.dp)
                                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                    points.groupBy { it.prefecture }.forEach { (prefecture, list) ->
                                        val name = data.map.firstOrNull { it.prefecture == prefecture }?.name.orEmpty()
                                        Text(name, style = MaterialTheme.typography.titleSmall)
                                        Text(list.joinToString("、") { it.name + if (it.estimated) "*" else "" }, style = MaterialTheme.typography.bodyMedium)
                                    }
                                }
                            }
                        }
                    }
                }
                if (observations.any { it.estimated }) item(key = "estimated") { Widgets.Help(stringResource(R.string.detail_estimated_note)) }
            } else {
                val areas = event.eew?.areas.orEmpty().filter { it.intensity != null }.sortedByDescending { it.intensity?.strongest }
                if (areas.isNotEmpty()) {
                    item(key = "areas-header") { Widgets.Header(stringResource(R.string.detail_areas)) }
                    item(key = "areas") {
                        Widgets.Panel(0, 1) {
                            areas.forEach { a ->
                                val range = a.intensity!!
                                Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                                    Intensities.Badge(range.strongest, Labels.intensityShort(snapshot, range.strongest) + if (range.upper == null) "+" else "", Labels.intensity(snapshot, range.strongest), 36.dp)
                                    Text(a.name + if (a.warning) " · " + stringResource(R.string.detail_warning_area) else "", style = MaterialTheme.typography.bodyMedium)
                                }
                            }
                        }
                    }
                }
            }
            item(key = "sources") {
                val sources = (event.eew?.sources.orEmpty() + event.report?.sources.orEmpty()).distinct()
                if (sources.isNotEmpty()) Widgets.Help(stringResource(R.string.detail_sources, sources.map { Labels.source(it) }.joinToString(", ")))
            }
        }
    }
}
