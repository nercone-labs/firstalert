package dev.nercone.firstalert.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.sizeIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.nercone.firstalert.core.AlertLevel
import dev.nercone.firstalert.core.Intensity

/**
 * The colours of the intensity classes. They rise from cool to warm and grow darker towards the
 * strongest, so that the order reads also in grey and for users who see colours differently; the
 * class is always written on its colour too, never told by the colour alone. Each colour carries
 * the text colour of the most contrast (at least 4.5:1).
 */
object Intensities {
    data class Swatch(val background: Color, val content: Color)

    fun swatch(intensity: Intensity): Swatch = when (intensity) {
        Intensity.ZERO -> Swatch(Color(0xFFE6E9EE), Color(0xFF1B1F24))
        Intensity.ONE -> Swatch(Color(0xFFCFE3F4), Color(0xFF0B2740))
        Intensity.TWO -> Swatch(Color(0xFF6FB4E8), Color(0xFF051D30))
        Intensity.THREE -> Swatch(Color(0xFF3FAE7A), Color(0xFF02210F))
        Intensity.FOUR -> Swatch(Color(0xFFF5D547), Color(0xFF2B2300))
        Intensity.FIVE_LOWER -> Swatch(Color(0xFFF59E2E), Color(0xFF2A1600))
        Intensity.FIVE_UPPER -> Swatch(Color(0xFFE5641E), Color(0xFF1F0B00))
        Intensity.SIX_LOWER -> Swatch(Color(0xFFC62828), Color(0xFFFFFFFF))
        Intensity.SIX_UPPER -> Swatch(Color(0xFF8E1630), Color(0xFFFFFFFF))
        Intensity.SEVEN -> Swatch(Color(0xFF4A1458), Color(0xFFFFFFFF))
    }

    fun color(intensity: Intensity): Color = swatch(intensity).background

    fun argb(intensity: Intensity): Int = color(intensity).toArgb()

    /**
     * A badge with the class written on its colour. `label` is the class as the core writes it in
     * the language of the user (`5弱`, `5-`); `description` is read by screen readers.
     */
    @Composable
    fun Badge(intensity: Intensity, label: String, description: String, size: Dp = 48.dp, fontSize: TextUnit = 20.sp, modifier: Modifier = Modifier) {
        val swatch = swatch(intensity)
        Box(
            modifier
                .sizeIn(minWidth = size, minHeight = size)
                .clip(MaterialTheme.shapes.medium)
                .background(swatch.background)
                .border(1.dp, swatch.content.copy(alpha = 0.25f), MaterialTheme.shapes.medium)
                .semantics { contentDescription = description }
                .padding(horizontal = 6.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(label, color = swatch.content, fontSize = fontSize, fontWeight = FontWeight.Bold, maxLines = 1)
        }
    }
}

/** The colours of the alert levels, fixed like those of the intensities. */
object Levels {
    data class Swatch(val background: Color, val content: Color)

    fun swatch(level: AlertLevel): Swatch = when (level) {
        AlertLevel.EMERGENCY -> Swatch(Color(0xFFB00020), Color.White)
        AlertLevel.WARNING -> Swatch(Color(0xFFF5B400), Color(0xFF231A00))
        AlertLevel.INFORMATION -> Swatch(Color(0xFF2F5D8C), Color.White)
        AlertLevel.NONE -> Swatch(Color(0xFF5F6368), Color.White)
    }
}
