package dev.nercone.firstalert.ui

import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.VolumeOff
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.nercone.firstalert.FirstAlertApplication
import dev.nercone.firstalert.R
import dev.nercone.firstalert.core.AlertLevel
import dev.nercone.firstalert.core.Notification

/**
 * The emergency over the whole screen, also on the lock screen: the level, what to expect, the
 * countdown, and what to do, in the largest type the screen allows, on the colour of danger. The
 * buttons are large; stopping the sound and closing are separate, so the warning can stay in sight
 * in silence.
 */
class AlertActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val host = (application as FirstAlertApplication).host
        setContent {
            FirstAlertTheme.Content {
                val notification by host.emergency.collectAsStateWithLifecycle()
                val snapshot by host.snapshot.collectAsStateWithLifecycle()
                val shown = notification
                if (shown == null) {
                    finish()
                } else {
                    Screen(shown, snapshot?.clockOffsetMs ?: 0, onSilence = { host.input(dev.nercone.firstalert.core.Inputs.acknowledge(host.now(), shown.key)) }) {
                        host.dismissEmergency(shown.key)
                        finish()
                    }
                }
            }
        }
    }

    @Composable
    fun Screen(n: Notification, offset: Long, onSilence: () -> Unit, onClose: () -> Unit) {
        // A cancellation or a lower update of the alert shows in its own colours, without a countdown.
        val swatch = Levels.swatch(n.level.coerceAtLeast(AlertLevel.INFORMATION))
        val emergency = n.level == AlertLevel.EMERGENCY
        val now = Events.rememberNow(offset, 200)
        Surface(Modifier.fillMaxSize(), color = swatch.background, contentColor = swatch.content) {
            Column(
                Modifier.fillMaxSize().safeDrawingPadding().verticalScroll(rememberScrollState()).padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(20.dp),
            ) {
                Text(n.title, fontSize = 30.sp, lineHeight = 36.sp, fontWeight = FontWeight.Black, modifier = Modifier.semantics { heading() })
                n.intensity?.let { intensity ->
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(20.dp)) {
                        Intensities.Badge(intensity, n.intensityLabel ?: intensity.code, n.intensityLabel ?: intensity.code, 120.dp, 52.sp)
                        Text(stringResource(if (n.local) R.string.alert_expected_here else R.string.alert_expected_max), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
                    }
                }
                if (emergency) Events.Countdown(n.arrival, false, now)
                Text(n.body, style = MaterialTheme.typography.titleLarge)
                n.actions?.let { Text(it, fontSize = 24.sp, lineHeight = 32.sp, fontWeight = FontWeight.Bold) }
                Button(
                    onClick = onSilence,
                    modifier = Modifier.fillMaxWidth().heightIn(min = 72.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = swatch.content, contentColor = swatch.background),
                ) {
                    Icon(Icons.AutoMirrored.Filled.VolumeOff, null)
                    Text("  " + stringResource(R.string.alert_silence), fontSize = 22.sp)
                }
                OutlinedButton(onClick = onClose, modifier = Modifier.fillMaxWidth().heightIn(min = 64.dp), colors = ButtonDefaults.outlinedButtonColors(contentColor = swatch.content)) {
                    Icon(Icons.Default.Close, null)
                    Text("  " + stringResource(R.string.alert_close), fontSize = 20.sp)
                }
            }
        }
    }
}
