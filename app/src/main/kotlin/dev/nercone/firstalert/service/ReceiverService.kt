package dev.nercone.firstalert.service

import android.app.Notification
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import dev.nercone.firstalert.FirstAlertApplication
import dev.nercone.firstalert.R
import dev.nercone.firstalert.alert.Alerter
import dev.nercone.firstalert.core.ConnectionState
import dev.nercone.firstalert.core.Snapshot
import dev.nercone.firstalert.ui.MainActivity
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

/**
 * Keeps the process receiving: a foreground service whose notification says whether the relays
 * are connected. The connections themselves belong to the `Host` of the application. When the
 * user turns every relay off, there is nothing to receive and the service stops.
 */
class ReceiverService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        foreground(notification(null))
        val host = (application as FirstAlertApplication).host
        host.start()
        scope.launch {
            host.snapshot.filterNotNull().map { Status.of(it) }.distinctUntilChanged().collect { status ->
                if (!status.enabled) {
                    stopSelf()
                    return@collect
                }
                NotificationManagerCompat.from(this@ReceiverService).let { manager ->
                    if (manager.areNotificationsEnabled()) manager.notify(SERVICE_NOTIFICATION, notification(status))
                }
            }
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int = START_STICKY

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    private fun foreground(notification: Notification) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(SERVICE_NOTIFICATION, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        } else {
            startForeground(SERVICE_NOTIFICATION, notification)
        }
    }

    /** What the notification of the service says. */
    data class Status(val enabled: Boolean, val open: Int, val enabledCount: Int) {
        companion object {
            fun of(snapshot: Snapshot): Status {
                val enabled = snapshot.connections.filter { it.enabled }
                return Status(enabled.isNotEmpty(), enabled.count { it.state == ConnectionState.OPEN }, enabled.size)
            }
        }
    }

    private fun notification(status: Status?): Notification {
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        val text = when {
            status == null -> getString(R.string.service_starting)
            status.open == status.enabledCount -> getString(R.string.service_receiving)
            status.open > 0 -> getString(R.string.service_partly)
            else -> getString(R.string.service_reconnecting)
        }
        return NotificationCompat.Builder(this, Alerter.CHANNEL_SERVICE)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(text)
            .setContentIntent(open)
            .setOngoing(true)
            .setShowWhen(false)
            .setSilent(true)
            .setPriority(NotificationCompat.PRIORITY_MIN)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .build()
    }

    companion object {
        const val SERVICE_NOTIFICATION = 1

        /** Starts receiving; allowed from the foreground, after a boot and after an update. */
        fun start(context: Context) {
            try {
                context.startForegroundService(Intent(context, ReceiverService::class.java))
            } catch (error: IllegalStateException) {
                // Not allowed from the background on this version: it starts with the next screen.
            } catch (error: SecurityException) {
                // The same, on versions that report it so.
            }
        }
    }
}
