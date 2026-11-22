package dev.nercone.firstalert.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import dev.nercone.firstalert.FirstAlertApplication
import dev.nercone.firstalert.core.AlertLevel
import dev.nercone.firstalert.core.EventDetail
import dev.nercone.firstalert.core.MapShape
import dev.nercone.firstalert.core.PrefectureAreas
import dev.nercone.firstalert.core.Settings
import dev.nercone.firstalert.core.Snapshot
import dev.nercone.firstalert.core.Waves
import dev.nercone.firstalert.service.ReceiverService
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** The state of the screens: the snapshot of the core, the map, and the waves of active warnings. */
class AppViewModel(application: Application) : AndroidViewModel(application) {
    val host = (application as FirstAlertApplication).host
    val snapshot: StateFlow<Snapshot?> = host.snapshot

    private val _map = MutableStateFlow<List<MapShape>>(emptyList())
    val map: StateFlow<List<MapShape>> = _map.asStateFlow()

    private val _waves = MutableStateFlow<List<Waves>>(emptyList())
    val waves: StateFlow<List<Waves>> = _waves.asStateFlow()

    private val _prefectures = MutableStateFlow<List<PrefectureAreas>>(emptyList())
    val prefectures: StateFlow<List<PrefectureAreas>> = _prefectures.asStateFlow()

    init {
        host.start()
        viewModelScope.launch { _map.value = host.map() }
        viewModelScope.launch { _prefectures.value = (1..47).mapNotNull { host.areas(it) } }
        // The waves move while an early warning is active: asked for ten times a second then.
        viewModelScope.launch {
            while (isActive) {
                val active = snapshot.value?.active?.any { it != "tsunami" } == true
                _waves.value = if (active) host.waves() else emptyList()
                delay(if (active) WAVES_MILLIS else IDLE_MILLIS)
            }
        }
    }

    fun configure(change: (Settings) -> Settings) {
        host.configure(change)
        // Turning a relay back on needs the service again (it stops when nothing is received).
        ReceiverService.start(getApplication())
    }

    fun test(level: AlertLevel) = host.test(level)

    suspend fun detail(key: String): EventDetail? = host.detail(key)

    companion object {
        const val WAVES_MILLIS = 100L
        const val IDLE_MILLIS = 1_000L
    }
}
