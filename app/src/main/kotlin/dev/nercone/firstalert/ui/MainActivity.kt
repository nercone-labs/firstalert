package dev.nercone.firstalert.ui

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Home
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.launch
import dev.nercone.firstalert.R
import dev.nercone.firstalert.service.ReceiverService
import kotlinx.coroutines.flow.MutableStateFlow

class MainActivity : ComponentActivity() {
    val model: AppViewModel by viewModels()

    /** The earthquake a notification asked to open. */
    private val subject = MutableStateFlow<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        ReceiverService.start(this)
        // The language of the application may have changed while it was closed.
        model.host.localeChanged(resources.configuration.locales.get(0).toLanguageTag())
        subject.value = intent?.getStringExtra(EXTRA_SUBJECT)
        setContent { FirstAlertTheme.Content { FirstAlertApp.Content(model, subject) } }
        // While the application is open, an emergency opens the alert screen at once (from the
        // background, the full-screen intent of its notification does).
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.RESUMED) {
                model.host.emergency.collect { emergency ->
                    if (emergency?.fullScreen == true) startActivity(Intent(this@MainActivity, AlertActivity::class.java))
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        intent.getStringExtra(EXTRA_SUBJECT)?.let { subject.value = it }
    }

    override fun onConfigurationChanged(newConfig: android.content.res.Configuration) {
        super.onConfigurationChanged(newConfig)
        model.host.localeChanged(newConfig.locales.get(0).toLanguageTag())
    }

    companion object {
        const val EXTRA_SUBJECT = "subject"
    }
}

/** The application: its destinations and where the user is, in the frame of the family (`Frame`). */
object FirstAlertApp {
    enum class Destination(override val title: Int, override val selectedIcon: ImageVector, override val icon: ImageVector) : Frame.Destination {
        HOME(R.string.tab_home, Icons.Filled.Home, Icons.Outlined.Home),
        HISTORY(R.string.tab_history, Icons.Filled.History, Icons.Outlined.History),
        SETTINGS(R.string.tab_settings, Icons.Filled.Settings, Icons.Outlined.Settings),
    }

    /** Where the user is: a destination, an earthquake opened from it, or a page of the settings. */
    data class Route(val destination: Destination, val page: SettingsScreen.Page?, val event: String?)

    fun interface Navigator {
        fun open(destination: Destination, page: SettingsScreen.Page?, event: String?)
    }

    @Composable
    fun Content(model: AppViewModel, subject: MutableStateFlow<String?>) {
        val snapshot by model.snapshot.collectAsStateWithLifecycle()
        val map by model.map.collectAsStateWithLifecycle()
        val waves by model.waves.collectAsStateWithLifecycle()
        val opened by subject.collectAsStateWithLifecycle()
        var destination by rememberSaveable { mutableStateOf(Destination.HOME) }
        var page by rememberSaveable { mutableStateOf<SettingsScreen.Page?>(null) }
        var event by rememberSaveable { mutableStateOf<String?>(null) }
        val route = Route(destination, page.takeIf { destination == Destination.SETTINGS }, event.takeIf { destination != Destination.SETTINGS })
        val navigator = Navigator { target, targetPage, targetEvent ->
            page = targetPage.takeIf { target == Destination.SETTINGS }
            event = targetEvent
            destination = target
        }
        LaunchedEffect(opened) {
            opened?.let {
                navigator.open(Destination.HISTORY, null, it)
                subject.value = null
            }
        }
        BackHandler(enabled = route.page != null || route.event != null) {
            page = null
            event = null
        }
        BackHandler(enabled = route.page == null && route.event == null && destination != Destination.HOME) { navigator.open(Destination.HOME, null, null) }

        NotificationRequest()

        val data = ScreenData(snapshot, map, waves)
        Frame.Content(
            destinations = Destination.entries,
            destination = destination,
            route = route,
            title = title(route, data),
            operation = null,
            onNavigate = { navigator.open(it, null, null) },
            onBack = if (route.page != null || route.event != null) ({ page = null; event = null }) else null,
        ) { shown ->
            when {
                shown.event != null -> DetailScreen.Content(shown.event, data, model)
                shown.destination == Destination.HOME -> HomeScreen.Content(data, model, navigator)
                shown.destination == Destination.HISTORY -> HistoryScreen.Content(data, navigator)
                else -> SettingsScreen.Content(data, model, shown.page) { page = it }
            }
        }
    }

    @Composable
    fun title(route: Route, data: ScreenData): String = when {
        route.event != null -> data.snapshot?.earthquakes?.firstOrNull { it.key == route.event }?.text?.title ?: stringResource(R.string.detail_title)
        route.page != null -> stringResource(route.page.title)
        route.destination == Destination.HOME -> stringResource(R.string.app_name)
        else -> stringResource(route.destination.title)
    }

    /** Asks for the permission to notify at the first start: without it, no alert can be shown. */
    @Composable
    fun NotificationRequest() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        val context = LocalContext.current
        var asked by rememberSaveable { mutableStateOf(false) }
        val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {}
        LaunchedEffect(Unit) {
            if (!asked && context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                asked = true
                launcher.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
        }
    }
}

/** What every screen reads. */
data class ScreenData(val snapshot: dev.nercone.firstalert.core.Snapshot?, val map: List<dev.nercone.firstalert.core.MapShape>, val waves: List<dev.nercone.firstalert.core.Waves>)
