package dev.nercone.firstalert.ui

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialExpressiveTheme
import androidx.compose.material3.MotionScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

/**
 * Material 3 Expressive with the colours of the wallpaper (Material You), and expressive motion.
 * Before Android 12, which has no colours of the wallpaper, a scheme of the icon's red. The
 * colours of danger (intensities, alert levels) are fixed (`Intensities`, `Levels`): they must
 * mean the same on every device.
 */
object FirstAlertTheme {
    private val lightFallback = lightColorScheme(primary = Color(0xFFB3261E), onPrimary = Color.White, primaryContainer = Color(0xFFFFDAD6), onPrimaryContainer = Color(0xFF410002))
    private val darkFallback = darkColorScheme(primary = Color(0xFFFFB4AB), onPrimary = Color(0xFF690005), primaryContainer = Color(0xFF93000A), onPrimaryContainer = Color(0xFFFFDAD6))

    @OptIn(ExperimentalMaterial3ExpressiveApi::class)
    @Composable
    fun Content(content: @Composable () -> Unit) {
        val context = LocalContext.current
        val dark = isSystemInDarkTheme()
        val colors = when {
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
            dark -> darkFallback
            else -> lightFallback
        }
        MaterialExpressiveTheme(colorScheme = colors, motionScheme = MotionScheme.expressive(), content = content)
    }
}
