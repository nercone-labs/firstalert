package dev.nercone.firstalert.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/** Starts receiving after a boot and after the application was updated. */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            Intent.ACTION_BOOT_COMPLETED, Intent.ACTION_MY_PACKAGE_REPLACED -> ReceiverService.start(context)
        }
    }
}
