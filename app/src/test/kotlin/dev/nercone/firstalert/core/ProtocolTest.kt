package dev.nercone.firstalert.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The application reads what the core writes (`firstalert::protocol`). These responses are
 * written as the core writes them: every effect, a snapshot, and fields the application does not
 * know, which must be ignored.
 */
class ProtocolTest {
    @Test
    fun everyEffectIsRead() {
        val response = Core.decode(
            """
            {"effects":[
              {"type":"connect","source":"wolfx","url":"wss://ws-api.wolfx.jp/all_eew","ping_interval_ms":25000},
              {"type":"send","source":"wolfx","text":"ping"},
              {"type":"disconnect","source":"p2pquake"},
              {"type":"request","id":3,"url":"https://api.p2pquake.net/v2/history?codes=551"},
              {"type":"notify","notification":{"key":"eew:1","category":"eew","level":"emergency","alerting":true,"title":"t","body":"b",
                "actions":"a","speech":"s","intensity":"5-","intensity_label":"5弱","local":true,"tsunami":null,"arrival":1000,"time":900,
                "sound":"emergency","sound_when_silenced":true,"voice":true,"vibration":"emergency","flash":false,"full_screen":true,"ongoing":true,"subject":"1"}},
              {"type":"withdraw","key":"eew:1"},
              {"type":"silence","key":"eew:1"},
              {"type":"wake","at":5000},
              {"type":"log","level":"warning","message":"m"}
            ]}
            """.trimIndent(),
        )
        assertNull(response.error)
        assertEquals(9, response.effects.size)
        assertEquals(Effect.Connect(Source.WOLFX, "wss://ws-api.wolfx.jp/all_eew", 25_000), response.effects[0])
        val notify = response.effects[4] as Effect.Notify
        assertEquals(AlertLevel.EMERGENCY, notify.notification.level)
        assertEquals(Intensity.FIVE_LOWER, notify.notification.intensity)
        assertEquals(Pattern.EMERGENCY, notify.notification.sound)
        assertEquals(Effect.Wake(5000), response.effects[7])
    }

    @Test
    fun snapshotWithUnknownFields() {
        val response = Core.decode(
            """
            {"effects":[],"snapshot":{"now":1,"clock_offset_ms":-20,"language":"easy_japanese","future_field":{"x":1},
              "settings":{"version":1,"language":null,"place":{"source":"point","coordinate":{"latitude":35.0,"longitude":139.0},"prefecture":13,"area":null},
                "ground":"soft","sources":{"wolfx":true,"p2pquake":false},
                "thresholds":{"eew_emergency":"4","eew_local":"3","eew_anywhere":"5-","report_anywhere":"3","report_local":"3"},
                "tsunami":true,"special":true,"training":false,
                "presentation":{"sound":true,"sound_when_silenced":true,"voice":true,"vibration":true,"flash":true,"full_screen":true}},
              "connections":[{"source":"p2pquake","enabled":false,"state":"disabled","since":null,"last_message":null,"failures":0}],
              "earthquakes":[{"key":"k","event_id":"20261201120000","origin":10,"hypocentre":{"name":"熊本県熊本地方","epicentre":{"latitude":32.7,"longitude":130.8},"depth_km":10.0,"magnitude":6.4},
                "eew":null,"report":{"kind":"detail","issued":20,"max_intensity":"6-","tsunami":"none","comment":null,"sources":["p2pquake"],"received":30},
                "prefectures":[{"prefecture":43,"intensity":"6-","forecast":false}],"local":{"estimate":null,"forecast":null,"observed":"6-","expected":null,"arrival":null},
                "level":"warning","first_seen":5,"updated":30,"text":{"title":"熊本県熊本地方","subtitle":"s","intensity":"最大震度6弱","status":"各地の震度に関する情報","local":null}}],
              "tsunami":{"issued":40,"received":41,"lifted":false,"areas":[{"name":"有明・八代海","grade":"warning","prefectures":[40,41,42,43],"immediate":true,
                "first_arrival":null,"first_condition":null,"max_height":"３ｍ","max_height_m":3.0}],"highest":"warning","local":"warning","comment":null,"sources":["p2pquake"]},
              "special":[],"active":["tsunami"],"level":"emergency","place":null,"intensities":[]}}
            """.trimIndent(),
        )
        val snapshot = response.snapshot
        assertNotNull(snapshot)
        snapshot!!
        assertEquals(Language.EASY_JAPANESE, snapshot.language)
        assertEquals(Ground.SOFT, snapshot.settings.ground)
        assertEquals(PlaceSource.POINT, snapshot.settings.place?.source)
        assertEquals(Intensity.SIX_LOWER, snapshot.earthquakes[0].maxIntensity)
        assertTrue(snapshot.tsunami!!.inForce)
        assertEquals(AlertLevel.EMERGENCY, snapshot.level)
    }

    @Test
    fun settingsRoundTripThroughTheInputs() {
        val settings = Settings(language = Language.ENGLISH, place = Place(PlaceSource.DEVICE, Coordinate(43.0, 141.3), 1, "石狩地方中部"))
        val input = Inputs.configure(42, settings)
        assertEquals("configure", input["type"].toString().trim('"'))
        val back = CoreJson.json.decodeFromJsonElement(Settings.serializer(), input["settings"]!!)
        assertEquals(settings, back)
    }

    @Test
    fun unreadableResponsesBecomeErrors() {
        assertNotNull(Core.decode("not json").error)
        assertNotNull(Core.decode("""{"effects":[{"type":"unknown_effect"}]}""").error)
    }

    @Test
    fun intensityRangesShowTheirStrongestClass() {
        assertEquals(Intensity.SIX_UPPER, IntensityRange(Intensity.FIVE_UPPER, Intensity.SIX_UPPER).strongest)
        assertEquals(Intensity.FIVE_LOWER, IntensityRange(Intensity.FIVE_LOWER, null).strongest)
    }
}
