package dev.nercone.firstalert.service

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.PowerManager
import android.util.Log
import dev.nercone.firstalert.alert.Alerter
import dev.nercone.firstalert.core.AlertLevel
import dev.nercone.firstalert.core.Core
import dev.nercone.firstalert.core.Coordinate
import dev.nercone.firstalert.core.Effect
import dev.nercone.firstalert.core.EventDetail
import dev.nercone.firstalert.core.Inputs
import dev.nercone.firstalert.core.LogLevel
import dev.nercone.firstalert.core.MapShape
import dev.nercone.firstalert.core.Notification
import dev.nercone.firstalert.core.PrefectureAreas
import dev.nercone.firstalert.core.Response
import dev.nercone.firstalert.core.Settings
import dev.nercone.firstalert.core.Snapshot
import dev.nercone.firstalert.core.Source
import dev.nercone.firstalert.core.Waves
import java.io.IOException
import java.util.Locale
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject
import okhttp3.Call
import okhttp3.Callback
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.WebSocket
import okhttp3.WebSocketListener

/**
 * Carries out what the core asks (`Effect`) and tells it what happens (`Inputs`): the platform
 * half of the core, the same for the service and the screens of this process.
 *
 * Everything runs on one thread (`dispatcher`), the thread of the core too, so that the
 * connections, the timer and the core see the events in the order they happen. The callbacks of
 * OkHttp come from its own threads and are handed over to it.
 */
class Host(private val context: Context) {
    private val dispatcher = Dispatchers.IO.limitedParallelism(1)
    // An unexpected exception is logged and the host carries on: it must never end the process
    // that receives the alerts.
    private val scope = CoroutineScope(SupervisorJob() + dispatcher + CoroutineExceptionHandler { _, error -> Log.e(TAG, "host", error) })
    private val core = Core(dispatcher)
    private val alerter = Alerter(context) { key -> input(Inputs.acknowledge(now(), key)) }
    /** The WebSockets: no read timeout (a quiet relay is not a dead one; the pings tell). */
    private val client = OkHttpClient.Builder().connectTimeout(10, TimeUnit.SECONDS).readTimeout(0, TimeUnit.MILLISECONDS).build()

    /** The requests: a response that stalls is given up, so that the next opening asks again. */
    private val http = client.newBuilder().readTimeout(15, TimeUnit.SECONDS).callTimeout(20, TimeUnit.SECONDS).build()
    private val sockets = mutableMapOf<Source, WebSocket>()

    /** Connections lost in a row, by source, for keeping awake through the first retries. */
    private val lostInARow = mutableMapOf<Source, Int>()

    /** Each new socket of a source has a new generation; callbacks of a replaced one are ignored. */
    private val generations = mutableMapOf<Source, Int>()
    private var wake: Job? = null
    private var started = false
    private var locator: Locator? = null
    private var mapShapes: List<MapShape>? = null

    private val _snapshot = MutableStateFlow<Snapshot?>(null)
    val snapshot: StateFlow<Snapshot?> = _snapshot.asStateFlow()

    /** The emergency to show on the alert screen, until the user closes it. */
    private val _emergency = MutableStateFlow<Notification?>(null)
    val emergency: StateFlow<Notification?> = _emergency.asStateFlow()

    fun now(): Long = System.currentTimeMillis()

    /** Starts the core once; later calls do nothing. */
    fun start() = scope.launch {
        if (started) return@launch
        started = true
        val directory = context.filesDir.resolve("core").apply { mkdirs() }
        handle(Inputs.start(now(), directory.absolutePath, locale()))
        watchNetwork()
    }

    /** Hands an input to the core and carries out its answer. */
    fun input(input: JsonObject) = scope.launch { handle(input) }

    private suspend fun handle(input: JsonObject): Response {
        val response = core.handle(input)
        response.error?.let { Log.e(TAG, "core: $it") }
        // The effects first: an alert is not to wait for the screens.
        response.effects.forEach { effect ->
            try {
                execute(effect)
            } catch (error: RuntimeException) {
                Log.e(TAG, "effect $effect", error)
            }
        }
        response.snapshot?.let { applySnapshot(it) }
        return response
    }

    private fun applySnapshot(snapshot: Snapshot) {
        _snapshot.value = snapshot
        val device = snapshot.settings.place?.source == dev.nercone.firstalert.core.PlaceSource.DEVICE
        if (device) {
            // Started again on every snapshot: a permission granted later starts it then.
            (locator ?: Locator(context) { coordinate -> input(Inputs.located(now(), coordinate)) }.also { locator = it }).start()
        } else if (locator != null) {
            locator?.stop()
            locator = null
        }
    }

    private fun execute(effect: Effect) {
        when (effect) {
            is Effect.Connect -> connect(effect)
            is Effect.Send -> sockets[effect.source]?.send(effect.text)
            is Effect.Disconnect -> disconnect(effect.source)
            is Effect.Request -> request(effect)
            is Effect.Notify -> {
                val n = effect.notification
                alerter.show(n, _snapshot.value?.language)
                when {
                    n.level == AlertLevel.EMERGENCY && n.alerting -> _emergency.value = n
                    // An update of the alert on the screen (a later serial, a cancellation) replaces it there.
                    _emergency.value?.key == n.key -> _emergency.value = n
                }
            }
            is Effect.Withdraw -> {
                alerter.withdraw(effect.key)
                if (_emergency.value?.key == effect.key) _emergency.value = null
            }
            is Effect.Silence -> alerter.silence(effect.key)
            is Effect.Wake -> schedule(effect.at)
            is Effect.Log -> when (effect.level) {
                LogLevel.INFO -> Log.i(TAG, effect.message)
                LogLevel.WARNING -> Log.w(TAG, effect.message)
                LogLevel.ERROR -> Log.e(TAG, effect.message)
            }
        }
    }

    private fun connect(effect: Effect.Connect) {
        disconnect(effect.source)
        val generation = (generations[effect.source] ?: 0) + 1
        generations[effect.source] = generation
        val socketClient = effect.pingIntervalMs?.let { client.newBuilder().pingInterval(it, TimeUnit.MILLISECONDS).build() } ?: client
        val request = Request.Builder().url(effect.url).header("User-Agent", USER_AGENT).build()
        sockets[effect.source] = socketClient.newWebSocket(request, Listener(effect.source, generation))
    }

    private fun disconnect(source: Source) {
        generations[source] = (generations[source] ?: 0) + 1
        sockets.remove(source)?.let { socket ->
            if (!socket.close(1000, null)) socket.cancel()
        }
    }

    private inner class Listener(val source: Source, val generation: Int) : WebSocketListener() {
        private fun current() = generations[source] == generation

        override fun onOpen(webSocket: WebSocket, response: okhttp3.Response) {
            scope.launch {
                if (current()) {
                    lostInARow[source] = 0
                    handle(Inputs.opened(now(), source))
                }
            }
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            // Keep the processor awake while the message is read and an alert is raised.
            val lock = wakeLock(WAKE_LOCK_MILLIS, "FirstAlert:message")
            scope.launch {
                try {
                    if (current()) handle(Inputs.received(now(), source, text))
                } finally {
                    if (lock.isHeld) lock.release()
                }
            }
        }

        override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
            webSocket.close(1000, null)
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) = lost("closed $code $reason")

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: okhttp3.Response?) = lost("failed: ${t.javaClass.simpleName}: ${t.message}")

        /**
         * The connection is gone. For the first few failures in a row the processor is kept
         * awake through the back-off, so that the connection comes back within seconds even
         * with the screen off; after that, a device without a network is let sleep.
         */
        private fun lost(reason: String) {
            scope.launch {
                if (!current()) return@launch
                val failures = (lostInARow[source] ?: 0) + 1
                lostInARow[source] = failures
                if (failures <= AWAKE_RETRIES) wakeLock(RETRY_WAKE_LOCK_MILLIS, "FirstAlert:retry")
                handle(Inputs.closed(now(), source, reason))
            }
        }
    }

    /** A partial wake lock that ends by itself after `millis`. */
    private fun wakeLock(millis: Long, tag: String): PowerManager.WakeLock =
        context.getSystemService(PowerManager::class.java).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, tag).apply {
            setReferenceCounted(false)
            acquire(millis)
        }

    private fun request(effect: Effect.Request) {
        val request = Request.Builder().url(effect.url).header("User-Agent", USER_AGENT).build()
        http.newCall(request).enqueue(object : Callback {
            override fun onFailure(call: Call, e: IOException) {
                input(Inputs.responded(now(), effect.id, 0, e.message ?: e.javaClass.simpleName))
            }

            override fun onResponse(call: Call, response: okhttp3.Response) {
                val body = try {
                    response.use { it.body.string() }
                } catch (error: IOException) {
                    input(Inputs.responded(now(), effect.id, 0, error.message ?: "read failed"))
                    return
                }
                input(Inputs.responded(now(), effect.id, response.code, body))
            }
        })
    }

    private fun schedule(at: Long) {
        wake?.cancel()
        wake = scope.launch {
            delay((at - now()).coerceAtLeast(0))
            handle(Inputs.tick(now()))
        }
    }

    private fun watchNetwork() {
        val connectivity = context.getSystemService(ConnectivityManager::class.java)
        connectivity.registerDefaultNetworkCallback(object : ConnectivityManager.NetworkCallback() {
            private var current: Network? = null
            private var seen = false

            override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
                if (!capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED)) return
                if (network != current) {
                    current = network
                    // The very first network is the one the connections were opened on; every
                    // later one (a change, or the network coming back after a loss) reconnects.
                    if (seen) input(Inputs.networkAvailable(now())) else seen = true
                }
            }

            override fun onLost(network: Network) {
                if (network == current) current = null
            }
        })
    }

    fun configure(change: (Settings) -> Settings) = scope.launch {
        val settings = _snapshot.value?.settings ?: return@launch
        handle(Inputs.configure(now(), change(settings)))
    }

    fun dismissEmergency(key: String) {
        input(Inputs.acknowledge(now(), key))
        if (_emergency.value?.key == key) _emergency.value = null
        // A test has nothing more to tell once closed.
        if (key == TEST) alerter.withdraw(key)
    }

    fun test(level: AlertLevel) = input(Inputs.test(now(), level))

    /** Asks the core something, on its thread, carrying out the effects of the answer too. */
    private suspend fun ask(input: JsonObject): Response = withContext(dispatcher) { handle(input) }

    suspend fun detail(key: String): EventDetail? = ask(Inputs.detail(now(), key)).detail

    suspend fun waves(): List<Waves> = ask(Inputs.waves(now())).waves.orEmpty()

    suspend fun areas(prefecture: Int): PrefectureAreas? = ask(Inputs.areas(now(), prefecture)).areas

    /** The outlines of the prefectures, asked of the core once. */
    suspend fun map(): List<MapShape> = mapShapes ?: ask(Inputs.map(now())).map.orEmpty().also { mapShapes = it }

    fun located(coordinate: Coordinate) = input(Inputs.located(now(), coordinate))

    /** The language of the application: its own (per-application language) or the system's. */
    private fun locale(): String = context.resources.configuration.locales.get(0)?.toLanguageTag() ?: Locale.getDefault().toLanguageTag()

    /** The language may have changed: the core words the alerts in the language of the application. */
    fun localeChanged(tag: String) = input(Inputs.locale(now(), tag))

    companion object {
        const val TAG = "FirstAlert"
        const val TEST = "test"
        const val USER_AGENT = "FirstAlert"
        const val WAKE_LOCK_MILLIS = 30_000L

        /** How many lost connections in a row keep the processor awake for the retry. */
        const val AWAKE_RETRIES = 3

        /** Longer than the longest back-off of the core (30 s). */
        const val RETRY_WAKE_LOCK_MILLIS = 35_000L
    }
}
