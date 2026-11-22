package dev.nercone.firstalert.alert

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import dev.nercone.firstalert.core.Pattern
import kotlin.math.PI
import kotlin.math.min
import kotlin.math.sin

/**
 * The sounds of the alerts, synthesised: each level has a pattern of its own that can be told
 * apart without seeing the screen. They are FirstAlert's own, not an imitation of the chime of
 * any broadcaster or of the government.
 *
 * - Emergency: three rising tones, quickly, again and again — urgent, and unlike any other sound
 *   of the phone.
 * - Warning: two falling tones, twice.
 * - Information: one soft tone.
 */
object Tones {
    const val SAMPLE_RATE = 44_100

    /** One tone: its frequency (0 for silence) and length. */
    data class Tone(val hertz: Double, val millis: Int)

    fun of(pattern: Pattern): List<Tone> = when (pattern) {
        Pattern.EMERGENCY -> listOf(Tone(880.0, 140), Tone(0.0, 30), Tone(1175.0, 140), Tone(0.0, 30), Tone(1568.0, 220), Tone(0.0, 260))
        Pattern.WARNING -> listOf(Tone(1046.5, 260), Tone(784.0, 380), Tone(0.0, 220), Tone(1046.5, 260), Tone(784.0, 380))
        Pattern.INFORMATION -> listOf(Tone(659.3, 220))
    }

    /** How many times a pattern plays before the voice. */
    fun repeats(pattern: Pattern): Int = when (pattern) {
        Pattern.EMERGENCY -> 4
        Pattern.WARNING -> 1
        Pattern.INFORMATION -> 1
    }

    /**
     * The samples of `tones`, `repeats` times: 16-bit mono, each tone shaped by short fades so that
     * it starts and ends without a click, with a third harmonic for a brighter, more audible sound.
     */
    fun render(tones: List<Tone>, repeats: Int): ShortArray {
        val once = tones.sumOf { it.millis } * SAMPLE_RATE / 1000
        val samples = ShortArray(once * repeats)
        var index = 0
        repeat(repeats) {
            for (tone in tones) {
                val count = tone.millis * SAMPLE_RATE / 1000
                val fade = min(count / 4, SAMPLE_RATE / 200)
                for (i in 0 until count) {
                    val value = if (tone.hertz == 0.0) {
                        0.0
                    } else {
                        val t = i.toDouble() / SAMPLE_RATE
                        val envelope = when {
                            i < fade -> i.toDouble() / fade
                            i > count - fade -> (count - i).toDouble() / fade
                            else -> 1.0
                        }
                        envelope * (0.8 * sin(2 * PI * tone.hertz * t) + 0.2 * sin(2 * PI * tone.hertz * 3 * t))
                    }
                    samples[index++] = (value * Short.MAX_VALUE * 0.9).toInt().toShort()
                }
            }
        }
        return samples
    }

    /** A track that plays `samples` once, on the stream of `usage`. The caller releases it. */
    fun track(samples: ShortArray, usage: Int): AudioTrack {
        val attributes = AudioAttributes.Builder().setUsage(usage).setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build()
        val format = AudioFormat.Builder().setSampleRate(SAMPLE_RATE).setEncoding(AudioFormat.ENCODING_PCM_16BIT).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build()
        val track = AudioTrack.Builder()
            .setAudioAttributes(attributes)
            .setAudioFormat(format)
            .setBufferSizeInBytes(samples.size * 2)
            .setTransferMode(AudioTrack.MODE_STATIC)
            .build()
        track.write(samples, 0, samples.size)
        return track
    }
}
