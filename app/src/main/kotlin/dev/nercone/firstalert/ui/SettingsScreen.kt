package dev.nercone.firstalert.ui

import android.Manifest
import android.app.LocaleManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.LocaleList
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Campaign
import androidx.compose.material.icons.filled.Code
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.NotificationsActive
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.filled.Sensors
import androidx.compose.material.icons.filled.Terrain
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.nercone.firstalert.R
import dev.nercone.firstalert.core.AlertLevel
import dev.nercone.firstalert.core.Coordinate
import dev.nercone.firstalert.core.Ground
import dev.nercone.firstalert.core.Intensity
import dev.nercone.firstalert.core.Language
import dev.nercone.firstalert.core.Place
import dev.nercone.firstalert.core.PlaceSource
import dev.nercone.firstalert.core.PrefectureAreas
import dev.nercone.firstalert.core.Settings
import dev.nercone.firstalert.core.Snapshot
import dev.nercone.firstalert.core.Source

/** The settings: a list of pages, each of one concern. */
object SettingsScreen {
    const val SOURCE = "https://github.com/nercone-labs/firstalert"

    enum class Page(val title: Int) {
        PLACE(R.string.settings_place),
        ALERTS(R.string.settings_alerts),
        PRESENTATION(R.string.settings_presentation),
        LANGUAGE(R.string.settings_language),
        SOURCES(R.string.settings_sources),
        ABOUT(R.string.settings_about),
    }

    @Composable
    fun Content(data: ScreenData, model: AppViewModel, page: Page?, onOpen: (Page) -> Unit) {
        val snapshot = data.snapshot ?: return
        Widgets.Screen {
            when (page) {
                null -> item(key = "pages") { Pages(snapshot, onOpen) }
                Page.PLACE -> item(key = "place") { PlacePage(snapshot, data, model) }
                Page.ALERTS -> item(key = "alerts") { Alerts(snapshot, model) }
                Page.PRESENTATION -> item(key = "presentation") { Presentation(snapshot, model) }
                Page.LANGUAGE -> item(key = "language") { LanguagePage(snapshot, model) }
                Page.SOURCES -> item(key = "sources") { Sources(snapshot, model) }
                Page.ABOUT -> item(key = "about") { About() }
            }
        }
    }

    @Composable
    fun Pages(snapshot: Snapshot, onOpen: (Page) -> Unit) {
        val s = snapshot.settings
        val off = stringResource(R.string.off)
        val pages = listOf(
            Triple(Page.PLACE, Icons.Default.Place, snapshot.place?.label ?: stringResource(R.string.place_none)),
            Triple(Page.ALERTS, Icons.Default.Campaign, stringResource(R.string.settings_alerts_summary, Labels.intensity(snapshot, s.thresholds.eewEmergency))),
            Triple(Page.PRESENTATION, Icons.Default.NotificationsActive, listOfNotNull(
                stringResource(R.string.presentation_sound).takeIf { s.presentation.sound },
                stringResource(R.string.presentation_voice).takeIf { s.presentation.voice },
                stringResource(R.string.presentation_vibration).takeIf { s.presentation.vibration },
                stringResource(R.string.presentation_flash).takeIf { s.presentation.flash },
            ).joinToString(" · ").ifEmpty { off }),
            Triple(Page.LANGUAGE, Icons.Default.Language, Labels.language(s.language)),
            Triple(Page.SOURCES, Icons.Default.Sensors, listOfNotNull("Wolfx".takeIf { s.sources.wolfx }, stringResource(R.string.source_p2pquake).takeIf { s.sources.p2pquake }).joinToString(" · ").ifEmpty { off }),
            Triple(Page.ABOUT, Icons.Default.Info, stringResource(R.string.settings_about_summary)),
        )
        Widgets.Group {
            pages.forEachIndexed { index, (page, icon, summary) -> Widgets.Item(index, pages.size, stringResource(page.title), summary, icon, onClick = { onOpen(page) }) }
        }
    }

    @Composable
    fun PlacePage(snapshot: Snapshot, data: ScreenData, model: AppViewModel) {
        val context = LocalContext.current
        val prefectures by model.prefectures.collectAsStateWithLifecycle()
        val place = snapshot.settings.place
        val location = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) model.configure { s -> s.copy(place = Place(PlaceSource.DEVICE, s.place?.coordinate ?: TOKYO, null, null)) }
        }
        var areas by remember { mutableStateOf<PrefectureAreas?>(null) }
        LaunchedEffect(place?.prefecture) { areas = place?.prefecture?.let { model.host.areas(it) } }
        Column(verticalArrangement = Arrangement.spacedBy(Widgets.SPACING)) {
            Widgets.Section(null, stringResource(R.string.place_help)) {
                val sources = PlaceSource.entries
                sources.forEachIndexed { index, source ->
                    Widgets.RadioItem(index, sources.size, Labels.placeSource(source), stringResource(placeSourceHelp(source)), selected = place?.source == source) {
                        when (source) {
                            PlaceSource.DEVICE -> if (permitted(context)) model.configure { s -> s.copy(place = Place(PlaceSource.DEVICE, s.place?.coordinate ?: TOKYO, null, null)) } else location.launch(Manifest.permission.ACCESS_COARSE_LOCATION)
                            PlaceSource.PREFECTURE -> {
                                val prefecture = place?.prefecture ?: 13
                                prefectures.firstOrNull { it.prefecture == prefecture }?.representative?.let { c -> model.configure { s -> s.copy(place = Place(PlaceSource.PREFECTURE, c, prefecture, null)) } }
                            }
                            PlaceSource.POINT -> model.configure { s -> s.copy(place = Place(PlaceSource.POINT, s.place?.coordinate ?: TOKYO, null, s.place?.area)) }
                        }
                    }
                }
            }
            if (place != null) {
                Widgets.Section(stringResource(R.string.place_where)) {
                    val names = prefectures.associate { it.prefecture to it.name }
                    if (place.source == PlaceSource.PREFECTURE && prefectures.isNotEmpty()) {
                        Widgets.ChoiceItem(0, 2, stringResource(R.string.place_prefecture), prefectures.map { it.prefecture }, place.prefecture ?: 13, describe = { names[it].orEmpty() }, onSelect = { code ->
                            prefectures.firstOrNull { it.prefecture == code }?.representative?.let { c -> model.configure { s -> s.copy(place = Place(PlaceSource.PREFECTURE, c, code, null)) } }
                        })
                    } else {
                        Widgets.Item(0, 2, stringResource(R.string.place_prefecture), place.prefecture?.let { names[it] } ?: stringResource(R.string.place_outside))
                    }
                    val choices = listOf<String?>(null) + areas?.areas.orEmpty()
                    Widgets.ChoiceItem(
                        1, 2, stringResource(R.string.place_area), choices, place.area,
                        describe = { it ?: stringResource(R.string.place_area_whole) },
                        onSelect = { area -> model.configure { s -> s.copy(place = s.place?.copy(area = area)) } },
                        description = stringResource(R.string.place_area_help),
                        enabled = areas != null,
                    )
                }
                if (data.map.isNotEmpty()) {
                    Widgets.Section(stringResource(R.string.place_map), stringResource(if (place.source == PlaceSource.POINT) R.string.place_map_help else R.string.place_map_help_other)) {
                        Widgets.Panel(0, 1) {
                            val area = remember(place.coordinate) { JapanMap.Area.around(place.coordinate, 250.0) }
                            JapanMap.Content(
                                data.map, emptyMap(), null, place.coordinate, emptyList(), area, stringResource(R.string.place_map_description, snapshot.place?.label ?: ""),
                                onTap = if (place.source == PlaceSource.POINT) ({ c -> model.configure { s -> s.copy(place = Place(PlaceSource.POINT, c, null, null)) } }) else null,
                            )
                        }
                    }
                }
            }
            Widgets.Section(stringResource(R.string.ground_title), stringResource(R.string.ground_help)) {
                Widgets.ChoiceItem(0, 1, stringResource(R.string.ground), Ground.entries, snapshot.settings.ground, describe = { Labels.ground(it) }, help = { Labels.groundHelp(it) }, onSelect = { g -> model.configure { it.copy(ground = g) } }, icon = Icons.Default.Terrain)
            }
        }
    }

    private val TOKYO = Coordinate(35.6812, 139.7671)

    private fun permitted(context: Context) = context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED

    private fun placeSourceHelp(source: PlaceSource) = when (source) {
        PlaceSource.PREFECTURE -> R.string.place_by_prefecture_help
        PlaceSource.POINT -> R.string.place_by_point_help
        PlaceSource.DEVICE -> R.string.place_by_device_help
    }

    @Composable
    fun Alerts(snapshot: Snapshot, model: AppViewModel) {
        val s = snapshot.settings
        val t = s.thresholds
        val classes = Intensity.entries.filter { it >= Intensity.ONE }
        val describe: @Composable (Intensity) -> String = { Labels.intensity(snapshot, it) }
        val help: @Composable (Intensity) -> String? = { Labels.intensityFeel(snapshot, it) }
        Column(verticalArrangement = Arrangement.spacedBy(Widgets.SPACING)) {
            Widgets.Section(stringResource(R.string.alerts_eew), stringResource(R.string.alerts_eew_help)) {
                Widgets.ChoiceItem(0, 3, stringResource(R.string.alerts_eew_emergency), classes, t.eewEmergency, describe, { v -> model.configure { it.copy(thresholds = it.thresholds.copy(eewEmergency = v)) } }, help = help, description = stringResource(R.string.alerts_eew_emergency_help))
                Widgets.ChoiceItem(1, 3, stringResource(R.string.alerts_eew_local), classes, t.eewLocal, describe, { v -> model.configure { it.copy(thresholds = it.thresholds.copy(eewLocal = v)) } }, help = help, description = stringResource(R.string.alerts_eew_local_help))
                Widgets.ChoiceItem(2, 3, stringResource(R.string.alerts_eew_anywhere), classes, t.eewAnywhere, describe, { v -> model.configure { it.copy(thresholds = it.thresholds.copy(eewAnywhere = v)) } }, help = help, description = stringResource(R.string.alerts_eew_anywhere_help))
            }
            Widgets.Section(stringResource(R.string.alerts_report)) {
                Widgets.ChoiceItem(0, 2, stringResource(R.string.alerts_report_local), classes, t.reportLocal, describe, { v -> model.configure { it.copy(thresholds = it.thresholds.copy(reportLocal = v)) } }, help = help)
                Widgets.ChoiceItem(1, 2, stringResource(R.string.alerts_report_anywhere), classes, t.reportAnywhere, describe, { v -> model.configure { it.copy(thresholds = it.thresholds.copy(reportAnywhere = v)) } }, help = help)
            }
            Widgets.Section(stringResource(R.string.alerts_other), stringResource(R.string.alerts_training_help)) {
                Widgets.SwitchItem(0, 3, stringResource(R.string.alerts_tsunami), stringResource(R.string.alerts_tsunami_help), s.tsunami) { v -> model.configure { it.copy(tsunami = v) } }
                Widgets.SwitchItem(1, 3, stringResource(R.string.alerts_special), stringResource(R.string.alerts_special_help), s.special) { v -> model.configure { it.copy(special = v) } }
                Widgets.SwitchItem(2, 3, stringResource(R.string.alerts_training), null, s.training) { v -> model.configure { it.copy(training = v) } }
            }
        }
    }

    @Composable
    fun Presentation(snapshot: Snapshot, model: AppViewModel) {
        val p = snapshot.settings.presentation
        fun set(change: (dev.nercone.firstalert.core.Presentation) -> dev.nercone.firstalert.core.Presentation) = model.configure { it.copy(presentation = change(it.presentation)) }
        Column(verticalArrangement = Arrangement.spacedBy(Widgets.SPACING)) {
            Widgets.Section(null, stringResource(R.string.presentation_help)) {
                Widgets.SwitchItem(0, 6, stringResource(R.string.presentation_sound), stringResource(R.string.presentation_sound_help), p.sound) { v -> set { it.copy(sound = v) } }
                Widgets.SwitchItem(1, 6, stringResource(R.string.presentation_silenced), stringResource(R.string.presentation_silenced_help), p.soundWhenSilenced, enabled = p.sound) { v -> set { it.copy(soundWhenSilenced = v) } }
                Widgets.SwitchItem(2, 6, stringResource(R.string.presentation_voice), stringResource(R.string.presentation_voice_help), p.voice) { v -> set { it.copy(voice = v) } }
                Widgets.SwitchItem(3, 6, stringResource(R.string.presentation_vibration), stringResource(R.string.presentation_vibration_help), p.vibration) { v -> set { it.copy(vibration = v) } }
                Widgets.SwitchItem(4, 6, stringResource(R.string.presentation_flash), stringResource(R.string.presentation_flash_help), p.flash) { v -> set { it.copy(flash = v) } }
                Widgets.SwitchItem(5, 6, stringResource(R.string.presentation_full_screen), stringResource(R.string.presentation_full_screen_help), p.fullScreen) { v -> set { it.copy(fullScreen = v) } }
            }
            Widgets.Section(stringResource(R.string.test_title), stringResource(R.string.test_help)) {
                Widgets.Panel(0, 1) {
                    Widgets.Actions(
                        listOf(
                            Widgets.Action(stringResource(R.string.test_emergency), Icons.Default.Campaign, true) { model.test(AlertLevel.EMERGENCY) },
                            Widgets.Action(stringResource(R.string.test_warning), Icons.Default.NotificationsActive, true) { model.test(AlertLevel.WARNING) },
                        ),
                    )
                }
            }
        }
    }

    @Composable
    fun LanguagePage(snapshot: Snapshot, model: AppViewModel) {
        val context = LocalContext.current
        val options = listOf(null, Language.JAPANESE, Language.EASY_JAPANESE, Language.ENGLISH)
        Widgets.Section(null, stringResource(R.string.language_help)) {
            options.forEachIndexed { index, language ->
                Widgets.RadioItem(index, options.size, Labels.language(language), if (language == Language.EASY_JAPANESE) stringResource(R.string.language_easy_japanese_help) else null, snapshot.settings.language == language) {
                    model.configure { it.copy(language = language) }
                    applyLocale(context, language)
                }
            }
        }
    }

    /** Follows the language of the words with the language of the screens, where Android allows it per application. */
    private fun applyLocale(context: Context, language: Language?) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        val tags = when (language) {
            null -> LocaleList.getEmptyLocaleList()
            Language.JAPANESE, Language.EASY_JAPANESE -> LocaleList.forLanguageTags("ja")
            Language.ENGLISH -> LocaleList.forLanguageTags("en")
        }
        context.getSystemService(LocaleManager::class.java).applicationLocales = tags
    }

    @Composable
    fun Sources(snapshot: Snapshot, model: AppViewModel) {
        val s = snapshot.settings.sources
        Widgets.Section(null, stringResource(R.string.sources_help)) {
            Source.entries.forEachIndexed { index, source ->
                val status = snapshot.connections.firstOrNull { it.source == source }
                val supporting = listOfNotNull(
                    stringResource(if (source == Source.WOLFX) R.string.source_wolfx_help else R.string.source_p2pquake_help),
                    status?.let { Labels.connection(it.state) + (it.lastMessage?.let { t -> " · " + stringResource(R.string.connection_last, Labels.time(t)) } ?: "") },
                ).joinToString("\n")
                val checked = if (source == Source.WOLFX) s.wolfx else s.p2pquake
                Widgets.SwitchItem(index, 2, Labels.source(source), supporting, checked) { v ->
                    model.configure { it.copy(sources = if (source == Source.WOLFX) it.sources.copy(wolfx = v) else it.sources.copy(p2pquake = v)) }
                }
            }
        }
    }

    @Composable
    fun About() {
        val uri = LocalUriHandler.current
        val context = LocalContext.current
        val version = remember { context.packageManager.getPackageInfo(context.packageName, 0).versionName.orEmpty() }
        Column(verticalArrangement = Arrangement.spacedBy(Widgets.SPACING)) {
            Widgets.Identity(stringResource(R.string.about_version, version))
            Widgets.Notice(Widgets.Tone.WARNING, stringResource(R.string.about_disclaimer_title), stringResource(R.string.about_disclaimer))
            Widgets.Section(stringResource(R.string.about_credits)) {
                Widgets.Panel(0, 1) {
                    Text(stringResource(R.string.about_credits_text), style = MaterialTheme.typography.bodyMedium)
                }
            }
            Widgets.Item(0, 1, stringResource(R.string.about_source), SOURCE, Icons.Default.Code) { Widgets.open(uri, SOURCE) }
        }
    }
}
