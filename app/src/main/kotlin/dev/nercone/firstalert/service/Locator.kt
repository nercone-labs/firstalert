package dev.nercone.firstalert.service

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.Looper
import dev.nercone.firstalert.core.Coordinate

/**
 * Follows the location of the device, coarsely and rarely, with the platform's own location
 * service (no Google Play services): only while the user's place is set to follow the device,
 * and only with the permission the user granted.
 */
class Locator(private val context: Context, private val onLocation: (Coordinate) -> Unit) {
    private val manager = context.getSystemService(LocationManager::class.java)
    private val listener = LocationListener { location: Location -> onLocation(Coordinate(location.latitude, location.longitude)) }
    private var running = false

    fun permitted(): Boolean = context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED

    /** The providers to ask, best first; the fused provider of the platform exists from Android 12. */
    private fun providers(): List<String> {
        val candidates = buildList {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) add(LocationManager.FUSED_PROVIDER)
            add(LocationManager.NETWORK_PROVIDER)
            add(LocationManager.PASSIVE_PROVIDER)
        }
        return candidates.filter { runCatching { manager.isProviderEnabled(it) }.getOrDefault(false) || it == LocationManager.PASSIVE_PROVIDER }
    }

    /** Starts following, when permitted and not yet started; safe to call again. */
    fun start() {
        if (running || !permitted()) return
        val provider = providers().firstOrNull() ?: return
        try {
            manager.getLastKnownLocation(provider)?.let { onLocation(Coordinate(it.latitude, it.longitude)) }
            manager.requestLocationUpdates(provider, INTERVAL_MILLIS, DISTANCE_METRES, listener, Looper.getMainLooper())
            running = true
        } catch (error: SecurityException) {
            // The permission was withdrawn: the place stays where it was.
        } catch (error: IllegalArgumentException) {
            // The provider went away.
        }
    }

    fun stop() {
        if (!running) return
        manager.removeUpdates(listener)
        running = false
    }

    companion object {
        const val INTERVAL_MILLIS = 15 * 60_000L
        const val DISTANCE_METRES = 500f
    }
}
