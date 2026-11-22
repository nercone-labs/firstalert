package dev.nercone.firstalert.alert

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioTrack
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.os.VibrationAttributes
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import dev.nercone.firstalert.FirstAlertApplication
import dev.nercone.firstalert.R
import dev.nercone.firstalert.core.AlertLevel
import dev.nercone.firstalert.core.Language
import dev.nercone.firstalert.core.Notification
import dev.nercone.firstalert.core.Pattern
import dev.nercone.firstalert.ui.AlertActivity
import dev.nercone.firstalert.ui.Intensities
import dev.nercone.firstalert.ui.MainActivity

/**
 * Presents the notifications of the core as it asks: the notification itself, and its sound,
 * voice, vibration, light and screen. The channels have no sound or vibration of their own: the
 * patterns are played here, so that each level sounds different, and an emergency can sound as an
 * alarm when the user allowed it.
 *
 * For an alerting notification: the tones play, then the voice reads it; an emergency repeats
 * both until the user stops it or `EMERGENCY_MILLIS` pass.
 */
class Alerter(private val context: Context, private val onAcknowledge: (String) -> Unit) {
    private val manager = NotificationManagerCompat.from(context)
    private val main = Handler(Looper.getMainLooper())
    private val speaker = Speaker(context)
    private val flasher = Flasher(context)
    private var track: AudioTrack? = null
    private var playing: String? = null
    private var playingLevel = AlertLevel.NONE
    private var until = 0L

    /** Each presentation has a generation; callbacks of an earlier one do nothing. */
    private var generation = 0
    private var awake: PowerManager.WakeLock? = null

    private val vibrator: Vibrator by lazy {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) context.getSystemService(VibratorManager::class.java).defaultVibrator else @Suppress("DEPRECATION") context.getSystemService(Vibrator::class.java)
    }

    fun show(notification: Notification, language: Language?) {
        post(notification)
        val presents = notification.sound != null || notification.voice || notification.vibration != null || notification.flash
        if (notification.alerting && presents) main.post { present(notification, language) }
    }

    private fun post(n: Notification) {
        if (context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) return
        val channel = when (n.level) {
            AlertLevel.EMERGENCY -> CHANNEL_EMERGENCY
            AlertLevel.WARNING -> CHANNEL_WARNING
            else -> CHANNEL_INFORMATION
        }
        val open = PendingIntent.getActivity(
            context,
            id(n.key),
            Intent(context, MainActivity::class.java).putExtra(MainActivity.EXTRA_SUBJECT, n.subject).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val text = listOfNotNull(n.body, n.actions).joinToString("\n\n")
        val builder = NotificationCompat.Builder(context, channel)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(n.title)
            .setContentText(n.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(text))
            .setContentIntent(open)
            .setWhen(n.time)
            .setShowWhen(true)
            .setOnlyAlertOnce(!n.alerting)
            .setOngoing(n.ongoing)
            .setAutoCancel(!n.ongoing)
            .setCategory(if (n.level == AlertLevel.EMERGENCY) NotificationCompat.CATEGORY_ALARM else NotificationCompat.CATEGORY_EVENT)
            .setPriority(
                when (n.level) {
                    AlertLevel.EMERGENCY -> NotificationCompat.PRIORITY_MAX
                    AlertLevel.WARNING -> NotificationCompat.PRIORITY_HIGH
                    else -> NotificationCompat.PRIORITY_DEFAULT
                },
            )
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
        n.intensity?.let { builder.setColor(Intensities.argb(it)).setColorized(n.level == AlertLevel.EMERGENCY) }
        if (n.level >= AlertLevel.WARNING) {
            val stop = PendingIntent.getBroadcast(
                context,
                id(n.key),
                Intent(context, AlertActionReceiver::class.java).setAction(AlertActionReceiver.ACTION_SILENCE).putExtra(AlertActionReceiver.EXTRA_KEY, n.key),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
            builder.addAction(0, context.getString(R.string.alert_silence), stop)
        }
        if (n.fullScreen) {
            val screen = PendingIntent.getActivity(
                context,
                id(n.key),
                Intent(context, AlertActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
            builder.setFullScreenIntent(screen, true)
        }
        if (n.arrival != null && n.level == AlertLevel.EMERGENCY) builder.setTimeoutAfter(EMERGENCY_TIMEOUT_MILLIS)
        try {
            manager.notify(id(n.key), builder.build())
        } catch (error: SecurityException) {
            // The permission was withdrawn.
        }
    }

    /**
     * Plays the tones, then the voice, of `n`; an emergency again and again until stopped or
     * `EMERGENCY_MILLIS` pass. A stronger alert that is still sounding is not cut off by a weaker
     * one. The repetition does not depend on the voice: without a text-to-speech engine, or with
     * the voice off, the tones still repeat.
     */
    private fun present(n: Notification, language: Language?) {
        val now = System.currentTimeMillis()
        if (playing != null && playing != n.key && playingLevel > n.level && now < until) return
        stopPlaying()
        val run = ++generation
        playing = n.key
        playingLevel = n.level
        val emergency = n.level == AlertLevel.EMERGENCY
        until = now + if (emergency) EMERGENCY_MILLIS else ONCE_MILLIS
        // The processor stays awake for the whole emergency, so that its repetition is not delayed.
        if (emergency) awake = context.getSystemService(PowerManager::class.java).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "FirstAlert:alert").apply {
            setReferenceCounted(false)
            acquire(EMERGENCY_MILLIS)
        }
        n.vibration?.let { vibrate(it) }
        if (n.flash) flasher.start(EMERGENCY_MILLIS)
        cycle(n, language, run)
    }

    /** One round: the tones, then the voice, then the next round or the end. */
    private fun cycle(n: Notification, language: Language?, run: Int) {
        if (generation != run) return
        val speech = n.speech?.takeIf { n.voice }
        val afterTones = {
            if (generation == run) {
                if (speech == null) {
                    next(n, language, run)
                } else {
                    var advanced = false
                    val advance = {
                        if (!advanced && generation == run) {
                            advanced = true
                            next(n, language, run)
                        }
                    }
                    speaker.onDone = { main.post(advance) }
                    speaker.speak(speech, language, n.soundWhenSilenced)
                    // Without an engine, or a slow one, the round goes on all the same.
                    main.postDelayed(advance, SPEECH_LIMIT_MILLIS)
                }
            }
        }
        val sound = n.sound
        if (sound != null) play(sound, if (n.soundWhenSilenced) AudioAttributes.USAGE_ALARM else AudioAttributes.USAGE_NOTIFICATION_EVENT, afterTones) else afterTones()
    }

    private fun next(n: Notification, language: Language?, run: Int) {
        if (generation != run) return
        if (n.level == AlertLevel.EMERGENCY && System.currentTimeMillis() < until) {
            main.postDelayed({ cycle(n, language, run) }, REPEAT_PAUSE_MILLIS)
        } else if (n.level != AlertLevel.EMERGENCY) {
            playing = null
            playingLevel = AlertLevel.NONE
        }
    }

    private fun play(pattern: Pattern, usage: Int, then: () -> Unit) {
        val samples = Tones.render(Tones.of(pattern), Tones.repeats(pattern))
        val track = try {
            Tones.track(samples, usage)
        } catch (error: UnsupportedOperationException) {
            then()
            return
        } catch (error: IllegalArgumentException) {
            then()
            return
        }
        if (track.state != AudioTrack.STATE_INITIALIZED) {
            track.release()
            then()
            return
        }
        this.track = track
        track.notificationMarkerPosition = samples.size - 1
        track.setPlaybackPositionUpdateListener(object : AudioTrack.OnPlaybackPositionUpdateListener {
            override fun onMarkerReached(t: AudioTrack) {
                main.post {
                    release(t)
                    then()
                }
            }

            override fun onPeriodicNotification(t: AudioTrack) {}
        }, main)
        try {
            track.play()
        } catch (error: IllegalStateException) {
            release(track)
            then()
        }
    }

    private fun release(t: AudioTrack) {
        runCatching { t.stop() }
        t.release()
        if (track === t) track = null
    }

    private fun vibrate(pattern: Pattern) {
        if (!vibrator.hasVibrator()) return
        val (timings, repeat) = when (pattern) {
            Pattern.EMERGENCY -> longArrayOf(0, 700, 200, 700, 200, 700, 800) to 0
            Pattern.WARNING -> longArrayOf(0, 300, 150, 300) to -1
            Pattern.INFORMATION -> longArrayOf(0, 150) to -1
        }
        val effect = VibrationEffect.createWaveform(timings, repeat)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val usage = if (pattern == Pattern.EMERGENCY) VibrationAttributes.USAGE_ALARM else VibrationAttributes.USAGE_NOTIFICATION
            vibrator.vibrate(effect, VibrationAttributes.createForUsage(usage))
        } else {
            @Suppress("DEPRECATION")
            vibrator.vibrate(effect, AudioAttributes.Builder().setUsage(if (pattern == Pattern.EMERGENCY) AudioAttributes.USAGE_ALARM else AudioAttributes.USAGE_NOTIFICATION).build())
        }
        if (repeat >= 0) main.postDelayed({ if (playing == null || System.currentTimeMillis() >= until) vibrator.cancel() }, EMERGENCY_MILLIS)
    }

    private fun stopPlaying() {
        generation++
        playing = null
        playingLevel = AlertLevel.NONE
        awake?.let { if (it.isHeld) it.release() }
        awake = null
        track?.let { release(it) }
        speaker.stop()
        flasher.stop()
        vibrator.cancel()
    }

    /** Stops the sound, voice, vibration and light of `key`, if they are of it. */
    fun silence(key: String) = main.post { if (playing == key || key == ALL) stopPlaying() }

    fun withdraw(key: String) {
        silence(key)
        manager.cancel(id(key))
    }

    fun acknowledge(key: String) = onAcknowledge(key)

    companion object {
        const val CHANNEL_EMERGENCY = "emergency"
        const val CHANNEL_WARNING = "warning"
        const val CHANNEL_INFORMATION = "information"
        const val CHANNEL_SERVICE = "service"
        const val ALL = "*"

        /** How long an emergency keeps sounding without the user. */
        const val EMERGENCY_MILLIS = 60_000L
        const val REPEAT_PAUSE_MILLIS = 800L

        /** The longest a voice is waited for before the alert goes on without it. */
        const val SPEECH_LIMIT_MILLIS = 20_000L

        /** How long an alert that plays once counts as playing, for not being cut off. */
        const val ONCE_MILLIS = 15_000L

        /** An early warning's notification is out of date after this. */
        const val EMERGENCY_TIMEOUT_MILLIS = 15 * 60_000L

        /** The id of the notification of a key: stable, and never that of the service (1). */
        fun id(key: String): Int = key.hashCode().let { if (it == 1) 2 else it }

        fun createChannels(context: Context) {
            val manager = context.getSystemService(NotificationManager::class.java)
            fun channel(id: String, name: Int, description: Int, importance: Int) = NotificationChannel(id, context.getString(name), importance).apply {
                this.description = context.getString(description)
                // The sounds and vibrations are played by the application, by level.
                setSound(null, null)
                enableVibration(false)
                lockscreenVisibility = android.app.Notification.VISIBILITY_PUBLIC
            }
            manager.createNotificationChannels(
                listOf(
                    channel(CHANNEL_EMERGENCY, R.string.channel_emergency, R.string.channel_emergency_text, NotificationManager.IMPORTANCE_HIGH),
                    channel(CHANNEL_WARNING, R.string.channel_warning, R.string.channel_warning_text, NotificationManager.IMPORTANCE_HIGH),
                    channel(CHANNEL_INFORMATION, R.string.channel_information, R.string.channel_information_text, NotificationManager.IMPORTANCE_DEFAULT),
                    channel(CHANNEL_SERVICE, R.string.channel_service, R.string.channel_service_text, NotificationManager.IMPORTANCE_MIN),
                ),
            )
        }
    }
}

/** The "stop the sound" button of a notification. */
class AlertActionReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != ACTION_SILENCE) return
        val key = intent.getStringExtra(EXTRA_KEY) ?: return
        (context.applicationContext as FirstAlertApplication).host.dismissEmergency(key)
    }

    companion object {
        const val ACTION_SILENCE = "dev.nercone.firstalert.SILENCE"
        const val EXTRA_KEY = "key"
    }
}
