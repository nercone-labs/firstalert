package dev.nercone.firstalert

import android.app.Application
import dev.nercone.firstalert.alert.Alerter
import dev.nercone.firstalert.service.Host

class FirstAlertApplication : Application() {
    lateinit var host: Host
        private set

    override fun onCreate() {
        super.onCreate()
        Alerter.createChannels(this)
        host = Host(this)
    }
}
