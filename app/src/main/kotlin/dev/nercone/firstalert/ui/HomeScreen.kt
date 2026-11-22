package dev.nercone.firstalert.ui

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings as AndroidSettings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.filled.BatteryAlert
import androidx.compose.material.icons.filled.Fullscreen
import androidx.compose.material.icons.filled.NotificationsOff
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.filled.Science
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.LifecycleResumeEffect
import dev.nercone.firstalert.R
import dev.nercone.firstalert.core.ConnectionState
import dev.nercone.firstalert.core.EarthquakeEvent
import dev.nercone.firstalert.core.Snapshot

/**
 * The home screen: what is happening now (an early warning in progress, tsunami warnings), whether
 * the alerts can reach the user, the map of the latest earthquake, and the latest earthquakes.
 */
object HomeScreen {
    /** How many earthquakes the home screen lists. */
    const val RECENT = 5

    @Composable
    fun Content(data: ScreenData, model: AppViewModel, navigator: FirstAlertApp.Navigator) {
        val snapshot = data.snapshot
        Widgets.Screen(columnsWidth = 1100.dp) { width ->
            if (snapshot == null) {
                item(key = "starting") { Widgets.Hero(Widgets.Mood.NEUTRAL, busy = true, title = stringResource(R.string.hero_starting), text = stringResource(R.string.hero_starting_text)) {} }
                return@Screen
            }
            for (key in snapshot.active) {
                if (key == "tsunami") {
                    snapshot.tsunami?.let { tsunami -> item(key = "active-tsunami") { Events.Tsunami(tsunami) } }
                } else {
                    snapshot.earthquakes.firstOrNull { it.key == key }?.let { event -> item(key = "active-$key") { ActiveEvent(event, snapshot, model) } }
                }
            }
            item(key = "status") { Status(snapshot) }
            item(key = "setup") { Setup(snapshot, navigator) }
            val latest = snapshot.earthquakes.firstOrNull { e -> e.key in snapshot.active } ?: snapshot.earthquakes.firstOrNull()
            if (latest != null && data.map.isNotEmpty()) {
                item(key = "map") { LatestMap(latest, data, onOpen = { navigator.open(FirstAlertApp.Destination.HISTORY, null, latest.key) }) }
            }
            item(key = "recent-header") { Widgets.Header(stringResource(R.string.home_recent)) }
            val recent = snapshot.earthquakes.take(RECENT)
            if (recent.isEmpty()) {
                item(key = "recent-empty") { Widgets.Help(stringResource(R.string.home_recent_empty)) }
            } else {
                item(key = "recent") {
                    Widgets.Group {
                        recent.forEachIndexed { index, event -> Events.Row(event, snapshot, index, recent.size) { navigator.open(FirstAlertApp.Destination.HISTORY, null, event.key) } }
                    }
                }
                item(key = "recent-all") {
                    Widgets.Item(0, 1, stringResource(R.string.home_all), null, trailing = { Icon(Icons.AutoMirrored.Filled.ArrowForward, null) }) {
                        navigator.open(FirstAlertApp.Destination.HISTORY, null, null)
                    }
                }
            }
            item(key = "disclaimer") { Widgets.Help(stringResource(R.string.disclaimer)) }
        }
    }

    @Composable
    fun ActiveEvent(event: EarthquakeEvent, snapshot: Snapshot, model: AppViewModel) {
        var actions by remember { mutableStateOf<String?>(null) }
        LaunchedEffect(event.key, event.updated) { actions = model.detail(event.key)?.actions }
        Events.Active(event, snapshot, actions)
    }

    /** Whether the alerts can reach the user: the state of the connections to the relays. */
    @Composable
    fun Status(snapshot: Snapshot) {
        val enabled = snapshot.connections.filter { it.enabled }
        val open = enabled.filter { it.state == ConnectionState.OPEN }
        val (mood, title, text) = when {
            enabled.isEmpty() -> Triple(Widgets.Mood.BAD, R.string.hero_off, R.string.hero_off_text)
            open.size == enabled.size -> Triple(Widgets.Mood.GOOD, R.string.hero_receiving, R.string.hero_receiving_text)
            open.isNotEmpty() -> Triple(Widgets.Mood.NEUTRAL, R.string.hero_partly, R.string.hero_partly_text)
            enabled.all { it.failures >= 3 } -> Triple(Widgets.Mood.BAD, R.string.hero_offline, R.string.hero_offline_text)
            else -> Triple(Widgets.Mood.NEUTRAL, R.string.hero_connecting, R.string.hero_connecting_text)
        }
        val detail = snapshot.connections.map { c ->
            val last = c.lastMessage?.let { stringResource(R.string.connection_last, Labels.time(it)) }.orEmpty()
            "${Labels.source(c.source)}: ${Labels.connection(c.state)} $last".trim()
        }.joinToString("\n")
        val place = snapshot.place?.label?.let { stringResource(R.string.hero_place, it) }
        Widgets.Hero(mood, busy = false, title = stringResource(title), text = listOfNotNull(stringResource(text), place).joinToString("\n"), detail = detail) {}
    }

    /** Is the device ready to alert? What is missing, each with the way to set it. */
    data class Readiness(val notifications: Boolean, val fullScreen: Boolean, val battery: Boolean)

    fun readiness(context: Context): Readiness {
        val notifications = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU || context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
        val fullScreen = Build.VERSION.SDK_INT < Build.VERSION_CODES.UPSIDE_DOWN_CAKE || context.getSystemService(NotificationManager::class.java).canUseFullScreenIntent()
        val battery = context.getSystemService(PowerManager::class.java).isIgnoringBatteryOptimizations(context.packageName)
        return Readiness(notifications, fullScreen, battery)
    }

    @Composable
    fun Setup(snapshot: Snapshot, navigator: FirstAlertApp.Navigator) {
        val context = LocalContext.current
        var resumes by remember { mutableIntStateOf(0) }
        LifecycleResumeEffect(Unit) {
            resumes++
            onPauseOrDispose {}
        }
        val readiness = remember(resumes) { readiness(context) }
        val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { resumes++ }
        val items = buildList<Triple<Int, androidx.compose.ui.graphics.vector.ImageVector, () -> Unit>> {
            if (snapshot.place == null) add(Triple(R.string.setup_place, Icons.Default.Place) { navigator.open(FirstAlertApp.Destination.SETTINGS, SettingsScreen.Page.PLACE, null) })
            if (!readiness.notifications && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) add(Triple(R.string.setup_notifications, Icons.Default.NotificationsOff) { permission.launch(Manifest.permission.POST_NOTIFICATIONS) })
            if (!readiness.fullScreen && snapshot.settings.presentation.fullScreen && Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                add(Triple(R.string.setup_full_screen, Icons.Default.Fullscreen) { open(context, Intent(AndroidSettings.ACTION_MANAGE_APP_USE_FULL_SCREEN_INTENT, Uri.parse("package:${context.packageName}"))) })
            }
            if (!readiness.battery) add(Triple(R.string.setup_battery, Icons.Default.BatteryAlert) { open(context, Intent(AndroidSettings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:${context.packageName}"))) })
        }
        if (items.isEmpty()) {
            Widgets.Item(0, 1, stringResource(R.string.setup_try), stringResource(R.string.setup_try_text), Icons.Default.Science) {
                navigator.open(FirstAlertApp.Destination.SETTINGS, SettingsScreen.Page.PRESENTATION, null)
            }
            return
        }
        Widgets.Section(stringResource(R.string.setup_title), stringResource(R.string.setup_help)) {
            items.forEachIndexed { index, (text, icon, action) ->
                Widgets.Item(index, items.size, stringResource(text), stringResource(setupHelp(text)), icon, trailing = { Icon(Icons.AutoMirrored.Filled.ArrowForward, null) }, onClick = action)
            }
        }
    }

    private fun setupHelp(text: Int): Int = when (text) {
        R.string.setup_place -> R.string.setup_place_text
        R.string.setup_notifications -> R.string.setup_notifications_text
        R.string.setup_full_screen -> R.string.setup_full_screen_text
        else -> R.string.setup_battery_text
    }

    private fun open(context: Context, intent: Intent) {
        try {
            context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        } catch (error: android.content.ActivityNotFoundException) {
            context.startActivity(Intent(AndroidSettings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:${context.packageName}")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        }
    }

    /** The map of the latest (or the active) earthquake, with a legend of its intensities. */
    @Composable
    fun LatestMap(event: EarthquakeEvent, data: ScreenData, onOpen: () -> Unit) {
        val snapshot = data.snapshot
        val fills = event.prefectures.associate { it.prefecture to it.intensity }
        val place = snapshot?.place?.place?.coordinate
        val area = remember(event.key, data.map.size, place) { JapanMap.areaFor(event.hypocentre.epicentre, place, fills.keys, data.map) }
        val waves = data.waves.filter { it.key == event.key }
        Widgets.Panel(0, 1, onClick = onOpen) {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Events.Badge(event, snapshot, 40.dp)
                    Column(Modifier.weight(1f)) {
                        Text(event.text.title, style = MaterialTheme.typography.titleMedium)
                        Text(event.text.subtitle, style = MaterialTheme.typography.bodySmall, color = Widgets.secondary)
                    }
                }
                Box(Modifier.padding(top = 4.dp)) {
                    JapanMap.Content(data.map, fills, event.hypocentre.epicentre, place, waves, area, mapDescription(event, snapshot))
                }
                Legend(fills.values.toSet(), snapshot, waves.isNotEmpty())
            }
        }
    }

    @Composable
    fun mapDescription(event: EarthquakeEvent, snapshot: Snapshot?): String = listOfNotNull(
        stringResource(R.string.map_description, event.text.title),
        event.text.intensity,
        event.text.local,
    ).joinToString(". ")

    /** The intensities on the map, strongest first, each with its colour and name. */
    @Composable
    fun Legend(intensities: Set<dev.nercone.firstalert.core.Intensity>, snapshot: Snapshot?, waves: Boolean) {
        if (intensities.isEmpty() && !waves) return
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            intensities.sortedDescending().take(6).forEach { Intensities.Badge(it, Labels.intensityShort(snapshot, it), Labels.intensity(snapshot, it), 32.dp, 14.sp) }
        }
        Text(stringResource(if (waves) R.string.map_legend_waves else R.string.map_legend), style = MaterialTheme.typography.bodySmall, color = Widgets.secondary)
    }
}
