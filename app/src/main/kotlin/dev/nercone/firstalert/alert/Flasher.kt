package dev.nercone.firstalert.alert

import android.content.Context
import android.hardware.camera2.CameraAccessException
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraManager
import android.os.Handler
import android.os.Looper

/**
 * Flashes the camera light during an emergency, for users who may not hear the alarm. Turning a
 * torch on or off needs no permission and no camera session.
 */
class Flasher(context: Context) {
    private val manager = context.getSystemService(CameraManager::class.java)
    private val handler = Handler(Looper.getMainLooper())
    private var running = false
    private var on = false
    private var until = 0L

    private val camera: String? by lazy {
        try {
            manager.cameraIdList.firstOrNull { manager.getCameraCharacteristics(it).get(CameraCharacteristics.FLASH_INFO_AVAILABLE) == true }
        } catch (error: CameraAccessException) {
            null
        }
    }

    private val step = object : Runnable {
        override fun run() {
            if (!running || System.currentTimeMillis() > until) {
                stop()
                return
            }
            set(!on)
            handler.postDelayed(this, if (on) ON_MILLIS else OFF_MILLIS)
        }
    }

    fun start(durationMillis: Long) {
        if (camera == null) return
        until = System.currentTimeMillis() + durationMillis
        if (running) return
        running = true
        handler.post(step)
    }

    fun stop() {
        running = false
        handler.removeCallbacks(step)
        if (on) set(false)
    }

    private fun set(value: Boolean) {
        val id = camera ?: return
        try {
            manager.setTorchMode(id, value)
            on = value
        } catch (error: CameraAccessException) {
            // The camera is in use by another application.
            running = false
        } catch (error: IllegalArgumentException) {
            running = false
        }
    }

    companion object {
        // About 2.5 flashes a second: well below the 3 a second that may trigger seizures.
        const val ON_MILLIS = 200L
        const val OFF_MILLIS = 200L
    }
}
