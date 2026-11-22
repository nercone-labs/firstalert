# Using FirstAlert

## First start

1. Allow notifications. Every alert is a notification; without the permission none can be shown.
2. **Set where you are** (Settings → Where you are). Without a place, FirstAlert cannot tell
   whether a warning is for you, so every JMA warning alerts with sound.
3. Follow the checklist on the home screen. *Keep receiving in the background* leaves FirstAlert
   out of battery optimisation: without it, Android may cut the connections while the phone
   sleeps, and an alert may come late or not at all. *Allow the full-screen alert* (Android 14 and
   later) lets emergencies cover the lock screen.
4. Try an alert (Settings → How to alert → Try it). It says that it is a test.

The notification *Receiving* stays while FirstAlert keeps its connections open. Its words say
whether both sources are connected.

## Where you are

- **Choose a prefecture** — the centre of the prefecture stands for your place.
- **Choose on the map** — tap where you are; pinch to zoom.
- **Follow this device** — the approximate (coarse) location, from Android's own location
  service, at most every 15 minutes or 500 m. No Google Play services are used.

Your place never leaves the device: it is used only to compare the warnings with where you are.

**Area of the warnings.** The early warnings name areas (細分区域) such as 熊本県熊本. With *the
whole prefecture* (the default), a warning for any area of your prefecture is taken for yours: it
may alert more often, never less. Choosing your area avoids alerts for the other areas.

**Ground.** Softer ground shakes more. The choice adjusts FirstAlert's own estimate only (×1.0
firm, ×1.5 average, ×2.0 soft, on the peak ground velocity); the JMA forecast is used as JMA gives
it.

## When to alert

| Setting | Default | Meaning |
| --- | --- | --- |
| Emergency from | 震度4 | An early warning expects at least this where you are: alarm, voice, the whole screen. |
| With sound from | 震度3 | An early warning expects at least this where you are: sound and vibration. |
| Elsewhere, quietly from | 震度5弱 | An early warning expects at least this anywhere: a quiet notification. A JMA *warning* anywhere is always shown quietly. |
| Earthquake reports with sound, where you are | 震度3 | At least this was observed in your prefecture (or your area). |
| Earthquake reports quietly, anywhere | 震度3 | The strongest intensity was at least this. |
| Tsunami warnings | on | Your coast: emergency for 津波警報 and 大津波警報, sound for 津波注意報. Elsewhere: sound for warnings, quiet for advisories. |
| Nankai Trough and Hokkaido–Sanriku information | on | 南海トラフ地震臨時情報 and 北海道・三陸沖後発地震注意情報, with sound. |
| Training and test reports | off | When on, they are shown quietly and marked 【訓練】, never as real ones. |

"Where you are" is the stronger of two values: the forecast of the warning for your area (or the
strongest in your prefecture), and FirstAlert's own estimate from the hypocentre and the magnitude.

An update of an early warning alerts again only when it is more severe than what was already
told; otherwise the notification is updated quietly. A cancelled warning is told quietly and stops
the alarm. Information that arrives late (an early warning more than 3 minutes after it was
issued, a report more than 30 minutes) is kept in the history without alerting.

## How to alert

Each alert is told in several ways at once, so that it reaches everyone:

- **Sound** — each level has its own tones: an emergency is three rising tones, again and again;
  a warning, two falling tones twice; information, one soft tone. *Emergencies in silent mode*
  plays emergencies on the alarm stream, which sounds in silent mode and, by Android's default,
  through Do Not Disturb.
- **Voice** — the alert read aloud by the system's text-to-speech, in the language of the alerts.
- **Vibration** — a pattern of its own for each level.
- **Flashing light** (off by default) — the camera light flashes during emergencies, 2.5 times a
  second (below the 3 a second that can trigger seizures).
- **Whole screen** — emergencies cover the screen, also when it is locked, with the intensity,
  the countdown and what to do in large type, and two large buttons: *Stop sound* and *Close*.

An emergency repeats its tones and voice for a minute unless stopped.

## The countdown

The seconds to the strong shaking come from the arrival time JMA forecasts for your area, or else
from FirstAlert's estimate (straight rays at 4.5 km/s). Both are a guide: the shaking can come
earlier, especially close to the epicentre, where the warning may arrive after the shaking.

## Language

The alerts, the voice and the screens can be in Japanese, やさしい日本語 or English, or follow the
system. Names of places stay in Japanese, as JMA gives them. On Android 13 and later the screens
follow the choice too; before, they follow the system.

## Sources

Both sources are on by default: whichever is faster alerts, and when one stops, the other still
reaches you.

- **Wolfx** relays JMA early warnings (forecasts and warnings), the list of recent earthquakes,
  the tsunami information and the special information.
- **P2P Quake** relays JMA early warnings (warnings only), earthquake reports with every station,
  and tsunami forecasts.

Turning both off stops receiving altogether, and the *Receiving* notification goes away.
