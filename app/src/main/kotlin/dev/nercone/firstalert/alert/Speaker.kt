package dev.nercone.firstalert.alert

import android.content.Context
import android.media.AudioAttributes
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import dev.nercone.firstalert.core.Language
import java.util.Locale

/**
 * Reads alerts aloud with the text-to-speech engine of the system. It starts the engine when
 * first needed and keeps it; a text asked for before the engine is ready is read once it is.
 */
class Speaker(private val context: Context) {
    private var engine: TextToSpeech? = null
    private var ready = false
    private var pending: Triple<String, Language?, Boolean>? = null
    var onDone: (() -> Unit)? = null

    fun speak(text: String, language: Language?, alarm: Boolean) {
        if (!ready) {
            pending = Triple(text, language, alarm)
            if (engine == null) {
                engine = TextToSpeech(context.applicationContext) { status ->
                    ready = status == TextToSpeech.SUCCESS
                    if (ready) {
                        engine?.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                            override fun onStart(utteranceId: String?) {}
                            override fun onDone(utteranceId: String?) {
                                onDone?.invoke()
                            }

                            @Deprecated("Deprecated in Java")
                            override fun onError(utteranceId: String?) {
                                onDone?.invoke()
                            }
                        })
                        pending?.let { (t, l, a) -> speak(t, l, a) }
                        pending = null
                    }
                }
            }
            return
        }
        val tts = engine ?: return
        val locale = when (language) {
            Language.ENGLISH -> Locale.ENGLISH
            Language.JAPANESE, Language.EASY_JAPANESE -> Locale.JAPANESE
            null -> Locale.getDefault()
        }
        if (tts.isLanguageAvailable(locale) >= TextToSpeech.LANG_AVAILABLE) tts.language = locale
        val usage = if (alarm) AudioAttributes.USAGE_ALARM else AudioAttributes.USAGE_NOTIFICATION
        tts.setAudioAttributes(AudioAttributes.Builder().setUsage(usage).setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build())
        // A little slower than the default, to be followed easily.
        tts.setSpeechRate(0.95f)
        tts.speak(text, TextToSpeech.QUEUE_FLUSH, null, "alert")
    }

    fun stop() {
        pending = null
        engine?.stop()
    }
}
