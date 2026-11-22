# FirstAlert — developer notes

These notes record what the core and the application do, why they are shaped the way they are,
and the options that were considered and set aside. Read them with the documentation of each
crate; every `lib.rs` states its contract.

## The goal

Receive the earthquake early warnings of the Japan Meteorological Agency (JMA) as fast as
possible, and tell them so that anyone understands them at once — whether they look at the
screen or not, hear or not, read Japanese quickly or not. Earthquake reports, tsunami warnings and
the special information on the Nankai Trough come the same way.

## Why this split

As in TypeFace Droplet and Replay Integrity, a core in Rust does nearly all the work and the GUI
is small. Unlike them, the core is not an executable run per command but a library loaded into
the application (`crates/firstalert-ffi`, `libfirstalert_ffi.so`): it holds state for as long as
the application runs (the connections' timers, the events being joined, what has been notified),
and a message must reach a decision in milliseconds, not after a process starts.

### The core performs no network access

The core is a state machine (`Core::handle(Input) -> Response`): the platform tells it what
happened (a connection opened, a text arrived, a response came, time passed) and carries out the
`Effect`s it returns (open a WebSocket, send a text, fetch a URL, notify, wake it up at a time).

- Every platform has a mature WebSocket and HTTP client with the system's TLS and proxy settings
  (OkHttp on Android, `URLSessionWebSocketTask` on Apple platforms, the .NET and JVM clients…).
  The host of a new platform is a few hundred lines.
- The core stays deterministic: the time is always an input, so the tests replay any sequence,
  and `firstalert-replay` runs the core on saved messages.
- *Set aside: TLS in the core* (BoringSSL through `boring`, or `rustls`). It would make the core
  self-sufficient on Linux and Windows, but adds a large native dependency to cross-compile for
  every ABI, duplicates what each system already does well, and bypasses the system's trust store
  and proxy. A future command-line host can still bring its own client.
- *Set aside: WebSocket framing in the core over a raw TLS stream from the platform.* It would
  move more into the core, but a home-made WebSocket client is a reliability risk exactly where
  reliability matters most.

The C interface carries JSON (`protocol.rs`). JSON costs a little CPU but keeps every binding
trivial and the protocol readable; the messages are small. The interface catches a panic of the
core and answers with an error, so a bug in reading one message never ends the process that
receives the alerts; for that reason the release profile does not use `panic = "abort"`, unlike
the other applications.

## The sources

| | Wolfx | P2P Quake |
| --- | --- | --- |
| Connection | `wss://ws-api.wolfx.jp/all_eew?exclude=…` (one socket for every source, the Chinese ones excluded) | `wss://api.p2pquake.net/v2/ws` |
| Early warnings | forecasts and warnings, every serial | warnings only |
| Earthquake reports | the list of the latest 50, without stations | each report, with every station |
| Tsunami | the national state (`active`, `cleared`, `unknown` …) | each forecast report |
| Special information | latest report of each product | — |
| Keep-alive | a heartbeat every minute; answers `ping` with `pong` | none at the application level |

Both are used at once. The faster one alerts; the other fills in what the first lacks (Wolfx's
forecast areas, P2P Quake's stations). Every time a connection opens — the first time too — the
core fetches what may have been issued while it was not open (`/v2/history` of P2P Quake; the
HTTP JSON of Wolfx's early warning, list, tsunami and special information). Catching up only on
reconnections was the first design; a device that started without a network then never saw a
tsunami warning issued before it got one, since neither relay sends it again.

Messages are read leniently (`crates/lenient`): a field that is missing or changes its type is
absent, and only a message without what identifies it (an early warning without an event ID or a
serial) is rejected. A strict schema would drop a whole warning because of one changed field.

P2P Quake may deliver a message more than once; its `id` is remembered (the last 1000, kept across
restarts) and a repeat is ignored.

## Joining the reports of one earthquake

An `EarthquakeEvent` gathers the early warnings and the reports of one earthquake.

- An **early warning** joins only the event with its JMA event ID (every warning has one), never
  an event by time alone.
- A **report** joins the event with its event ID when one has it. Otherwise (P2P Quake's reports
  have none) it joins the *closest* event — by origin time, within 90 s — that has no different
  event ID and that it can be shown to concern: with both epicentres known, within 150 km; a
  report without an epicentre (震度速報) only when it observed shaking in a prefecture the event
  concerns (the areas of its warning, its observations, or the prefecture within 100 km of its
  epicentre). A report that cannot be shown to be of an event becomes an event of its own.

The first design joined a report without an epicentre by time alone, and took the first match
rather than the closest. The independent review showed the consequence: a 震度速報 of a small
earthquake in Hokkaido, issued during an emergency warning for Noto, joined the Noto event, so
the report "ended" the warning — it silenced the alarm and withdrew the alert. A report is joined
to an active warning now only when it is shown to concern the same region. Two earthquakes this
close in time and place may still be joined; the event ID, when present, always decides.

- A warning of a higher serial replaces the earlier one; the same serial from the other relay
  adds its source and the areas the first lacked. A cancellation keeps the areas of the warning
  it cancels.
- A report replaces another of lower completeness (震度速報 < 震源 < 震源・震度 < 各地の震度) or
  a later one of the same; the Wolfx list, which has no time of issue, replaces only a less
  complete one, or one of the same kind whose values it changes (a correction).
- A report's hypocentre replaces the warning's.

The map colours each prefecture by its strongest observed intensity, or, before any observation,
by the strongest forecast of the warning.

## What an earthquake means where the user is

`LocalImpact` combines three things:

1. **The forecast of the warning** for the user's area (細分区域) when chosen, else the strongest
   of the areas of the user's prefecture. Matching the whole prefecture alerts more often, never
   less; choosing an area avoids that.
2. **FirstAlert's own estimate** (`jma::GroundMotion`): the peak ground velocity on the
   engineering bedrock by Si and Midorikawa (1999), amplified ×1.41 to a firm surface
   (Vs ≈ 400 m/s, the factor of NIED's hazard maps), times the ground the user chose (×1.0,
   ×1.5, ×2.0), converted to a measured intensity by Fujimoto and Midorikawa (2005). The JMA
   magnitude is taken for the moment magnitude, and the distance to the fault is the distance to
   the hypocentre less half the fault length of Matsuda (1975), at least 3 km: both err towards
   stronger shaking. Assumed hypocentres (仮定震源要素, the PLUM and level methods) have no
   real magnitude and depth and are not estimated.
3. **The observed intensity** in the user's area or prefecture, from the reports.

The intensity to alert with (`expected`) is the stronger of the forecast and the estimate.

The arrival of the strong shaking is the forecast's time for the area, or else straight rays at
7.8 km/s (P) and 4.5 km/s (S), the speeds of the upper mantle, which put the arrival earlier than
the true one for nearly every site: a countdown that ends early is the safe error. *Set aside: the
JMA2001 travel-time table*, which is more accurate but large; it can replace `Arrival` without
changing anything else.

The coefficients were checked against OpenQuake's implementation of Si and Midorikawa (1999)
(`openquake/hazardlib/gsim/si_midorikawa_1999.py`) and published quotations of the other
relations. An Si–Midorikawa estimate is typically within one class of the observation and can be
off by more; it is labelled as an estimate everywhere.

## How strongly to alert

`alert::Policy` gives each piece of information a level: none (history only), information
(quiet), warning (sound and vibration), emergency (alarm, voice, full screen, light).

| Information | Emergency | Warning | Information |
| --- | --- | --- | --- |
| Early warning | expected ≥ *Emergency from* (震度4) | expected ≥ *With sound from* (震度3); any JMA warning when no place is set | a JMA warning elsewhere; max ≥ *Elsewhere* (震度5弱) |
| Earthquake report | — | observed where you are ≥ 震度3 | max ≥ 震度3 |
| Tsunami | 津波警報 or 大津波警報 on your coast | 注意報 on your coast; a warning elsewhere | an advisory elsewhere; lifting |
| Special information | — | 南海トラフ地震臨時情報, 北海道・三陸沖後発地震注意情報 | — |

Why these defaults: JMA issues a warning where 震度4 or more is expected when 震度5弱 or more is
expected somewhere, so *Emergency from 震度4* matches the areas JMA itself warns. Without a place
the core cannot tell whether a warning is for the user, so it errs towards alerting.

Rules that apply to all:

- **Freshness.** Early warnings older than 3 minutes, reports older than 30 minutes, tsunami
  reports older than 6 hours and special information older than 12 hours are kept without
  alerting. This covers the catch-up requests at start and after a reconnection, and a relay that
  delivers late. The age is measured on the servers' clock (see *Time*).
- **No repeats.** What was notified under a key (`eew:<event>`, `quake:<event>`, `tsunami`,
  `special:<id>`) is remembered with its level and intensity, also across restarts. An update
  alerts again only when its level or intensity rises; otherwise the notification is updated
  quietly; an identical one is skipped. For an earthquake the remembered level is the highest
  reached, so that a warning that wavers between serials does not alert again and again. For
  tsunamis it is the level in force now (none after lifting): warnings fall and rise over hours
  and come again with the next earthquake, and the first design, which remembered the highest
  ever, left every tsunami warning after the first one silent.
- **Cancellation** is told quietly and stops the sound (`Effect::Silence`). When the report of an
  earthquake arrives, its early warning is withdrawn: the report says what happened.
- **A warning stays a warning**: a later serial that is only a forecast does not lower the
  level of an earthquake that was warned for.
- **Training** reports are dropped unless the user asks for them, and are then never more than
  information, marked 【訓練】.

## Words

The core words every alert (`words/`), so that every platform says the same: Japanese,
やさしい日本語 and English, each a `Phrasebook`. Each alert has a title, a body (most important
first: where, what is expected where you are, then the magnitude), the advice of what to do, and a
text for the voice. The advice follows the common public guidance in Japan (protect
your head; stay away from what can fall; tsunami: move to high ground and do not return until
lifted), in FirstAlert's own words. The descriptions of the intensity classes paraphrase JMA's
table of what each class feels like.

やさしい日本語 uses short sentences, common words, spaces between phrases, and the reading of the
harder words in parentheses. It is a language of its own, not a simplification of the screens.

The names of places (hypocentres, areas) come from JMA in Japanese and stay in Japanese in every
language; translating them reliably would need JMA's own tables in English.

## Time

Every input carries the platform's time. The servers' time is learnt from Wolfx's heartbeats and
pongs (`clock_offset_ms`), and, while Wolfx has given none for ten minutes (it may be off or
down), from the time P2P Quake received each live message, a fraction of a second early. All the
times of events and reports are kept in the servers' time, and freshness, the active warnings and
the countdowns use it: a device whose clock is minutes off still alerts and counts down correctly.
Only the wake-ups asked of the platform are converted back to its clock. Offsets beyond a day are
ignored as implausible. JMA's times are
Japan Standard Time (UTC+9, no daylight saving); the core keeps UTC milliseconds.

## Connections

Each relay has a `Connection`: back-off after failures (0.5, 1, 2, 5, 10, 20, then 30 s; reset
after a minute of stable connection), a 15 s limit to connect, WebSocket pings every 25 s (sent
by the platform), Wolfx's own `ping` every 30 s, and for Wolfx a silence of 100 s (it sends a
heartbeat every minute) means a dead connection, which is replaced. P2P Quake sends nothing while
nothing happens, so its liveness rests on the WebSocket pings. When the network changes, the
connections are replaced at once, since a socket on the old network may be dead without knowing.

## Files

The core keeps `settings.json` and `history.json` in the directory the platform gives
(`files/core` on Android), each written to a temporary file that then replaces the old one. A file
that cannot be read is kept aside as `.broken` and the core starts empty, so a damaged file is
neither lost nor fatal. The history keeps the latest 300 earthquakes.

## The Android application

- **A foreground service** (`ReceiverService`, type `specialUse`) keeps the process and its
  connections alive; its notification says whether both relays are connected. It starts with the
  application, after a boot and after an update, and stops itself when the user turns every relay
  off. `dataSync` was set aside: Android 15 limits it to six hours a day.
- **The host** (`service/Host.kt`) carries out the effects on one thread, the thread of the core,
  so that events are handled in the order they happen, and carries them out before it publishes
  the snapshot to the screens. Each effect and the snapshot are read separately
  (`Core.decode`), so that a field the application cannot read never costs an alert; an
  unexpected exception is logged and the host carries on. Callbacks of a replaced socket are
  recognised by a generation number and ignored.
- **Staying awake.** A message holds a partial wake lock while it is read and alerted (30 s at
  most); an emergency holds one while it sounds (a minute); the first three lost connections in a
  row hold one through the back-off (35 s), so that a connection comes back within seconds with
  the screen off. After that a device without a network is let sleep: holding the processor awake
  for as long as it is offline would empty the battery. HTTP requests have a 20 s limit, so that a
  stalled catch-up is retried at the next opening.
- **Presenting an alert** (`alert/Alerter.kt`): a weaker alert never cuts off a stronger one that
  is still sounding, and the repetition of an emergency is driven by the tones, not by the voice,
  so that it repeats without a text-to-speech engine (common on phones without Google services)
  or with the voice turned off. The alert screen follows the updates of its alert (a later
  serial, a cancellation) and closes when the alert is withdrawn.
- **OkHttp** is the WebSocket and HTTP client (Android has no WebSocket client of its own). It is
  the one dependency the other applications do not have, and is verified like every other
  (`gradle/verification-metadata.xml`).
- **Sounds** are synthesised (`alert/Tones.kt`) rather than set on the notification channels:
  each level needs its own pattern, and an emergency must be able to play on the alarm stream.
  They are FirstAlert's own; imitating the chime of a broadcaster was set aside both for its
  rights and so as not to be taken for the official broadcast.
- **The full-screen alert** (`AlertActivity`) opens from the notification's full-screen intent,
  or directly when the application is open. On Android 14 and later the permission is the user's
  to grant; the home screen offers it.
- **Location** uses the platform's `LocationManager` (the fused provider from Android 12), coarse,
  every 15 minutes or 500 m, and only while the place follows the device. Google Play services
  are not used.
- **The map** is drawn on a `Canvas` from the outlines the core holds; no map tiles are fetched,
  so the map works offline and tells no server where the user looks.

## Universal design

The aim is that the alerts work for nearly everyone, not as a separate mode for some.

- **Redundancy.** Every alert is told at once in words, colour and shape, sound, voice and
  vibration, and optionally light. Nothing is told by colour alone: intensity badges carry their
  class, the epicentre is a cross, the user's place a ringed dot, the waves are explained in the
  legend.
- **Colours.** The intensity colours rise from cool to warm and darken towards the strongest, so
  the order reads in grey too; every pair of colour and text has a contrast of at least 4.5:1
  (the lowest is 5.6:1). The colours of danger are fixed, not taken from the wallpaper.
- **Large and plain.** The alert screen uses the largest type; its two buttons are at least 64 dp
  high and do different things (stop the sound, close), so the warning can stay in sight in
  silence. Type follows the system's font scale.
- **Screen readers.** Badges and the map have descriptions; the countdown is a live region
  announced every five seconds rather than every tick.
- **Light.** The flashing light stays below three flashes a second (WCAG 2.3.1).
- **Language.** やさしい日本語 for readers of Japanese as a second language and for children; English.

## Known limitations

These are stated in the application (About, and the home screen) as well.

- **The relays are unofficial.** Wolfx and P2P Quake relay JMA's information without any
  guarantee; P2P Quake states that its early warnings are not redundant and not re-sent after a
  failure. Using both reduces, but does not remove, the risk of a missed or late alert.
- **Android may delay the network in deep sleep.** A foreground service keeps the process, but
  Doze can still suspend the network of applications not exempted from battery optimisation. The
  home screen asks the user to exempt FirstAlert; without it, alerts can be late.
- **Close to the epicentre the warning can come after the shaking.** This is inherent to early
  warnings; the countdown says so.
- **The estimate is a guide.** See above; the forecast of JMA, when there is one, counts at least
  as much.
- **Tsunami areas are matched to prefectures.** A user in a prefecture with several coasts is
  alerted for any of them.
- **Wolfx's list** is read for its ten newest earthquakes on each update.
- **The snapshot is whole.** Every change sends the screens the whole state (up to 300
  earthquakes). It is read after the alert has been carried out, so it does not delay alerts, but
  on a slow phone it costs memory and time; sending summaries, with the details on demand, is
  left for later.
- **Timers in deep sleep.** Apart from the wake locks above, the timers of the core run on the
  application's clock, which stops while the processor sleeps; an exact alarm
  (`AlarmManager`) would need a permission the user must grant, and was set aside for now.

## Review

The code was reviewed independently (a separate agent that had not written it) in three steps:
listing every candidate problem, then verifying each by tracing the code, by the replay tool and
by small programs against the core. Eighteen problems were found and verified; all are fixed, and
the ones in the core have tests of their own in `crates/firstalert/tests/engine.rs` (the second
tsunami episode, a raised advisory, Wolfx's `expired`, a report of another earthquake during a
warning, a device clock behind the servers, a device clock ahead with P2P Quake alone, a forecast
after a warning, and the catch-up on every opening).

## Continuous integration

The workflows are those of TypeFace Droplet and Replay Integrity (`test.yml` on every push and
pull request; `release.yml` on a tag `vMAJOR.MINOR.PATCH`, which writes the version, runs the
tests and publishes the signed APK). One addition: Gradle runs through `.github/scripts/gradle.sh`,
which runs it again (up to three times, a minute and then two apart) when it failed only because
a dependency repository could not be reached.

The first release failed this way. Its release job started with an empty Gradle home and could not
resolve the Kotlin Compose plugin, which the two test jobs of the same run, also with empty Gradle
homes, had resolved minutes before from the same commit; the same build passes locally with an
empty Gradle home too. Gradle reports an unreachable repository and a missing artifact with the
same message, so the script cannot tell them apart: a dependency that is really missing fails
after the last attempt, a few minutes later than it would otherwise. A build or test failure is
never retried. *Set aside:* raising Gradle's own network retries, which do not cover a failure
that ends as "not found", and retrying every failure, which would run failing tests three times.
