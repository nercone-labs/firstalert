<img width="1920" src="https://raw.githubusercontent.com/nercone-dev/website/refs/heads/main/public/static/assets/images/projects/labs/firstalert/header.png" />

# FirstAlert

An app that receives Japan's earthquake early warnings (緊急地震速報), earthquake reports and tsunami warnings as fast as it can, and tells them in ways anyone can understand.

> [!WARNING]
> FirstAlert is **not** an official service. It receives the information of the Japan Meteorological Agency through unofficial relays (Wolfx and P2P Quake), which can be late, wrong, or stop without notice. Never rely on it alone: follow the official information, the emergency broadcasts (緊急速報メール, TV, radio) and the instructions of your municipality.

## What it does

- **Receives from two relays at once** — the [Wolfx Open API](https://wolfx.jp/docs/open-api) and the [P2P Quake JSON API v2](https://www.p2pquake.net/develop/json_api_v2/) over WebSocket — and joins what both say of one earthquake. Whichever is faster alerts; the other fills the gaps; the same report is never alerted twice.
- **Tells what it means where you are.** With your place (a prefecture, a point on the map, or the device's location), it matches the areas of the warning and estimates the shaking and its arrival from the hypocentre and the magnitude, and counts down to the strong shaking.
- **Alerts in proportion.** An early warning that expects strong shaking where you are is an *emergency*: an alarm (also in silent mode, if you allow it), the voice, a vibration of its own, a flashing light (optional), and the whole screen, also when locked. Warnings elsewhere and earthquake reports come quietly. Every threshold is a setting.
- **Tsunami warnings** for your coast, and the extraordinary information on the Nankai Trough and off Hokkaido and Sanriku.
- **Universal design.** Every alert is told in words, colour *and* shape, sound, voice and vibration at once, so that it reaches people who cannot see the screen, hear the sound, or read Japanese quickly. The words come in Japanese, *やさしい日本語* (easy Japanese) and English. Intensities are always written on their colour, the colours meet WCAG contrast, the light flashes slower than three times a second, and countdowns are announced to screen readers.

See [docs/usage.md](docs/usage.md) for the settings.

## How it is built

Like [TypeFace Droplet](https://github.com/nercone-labs/typeface-droplet) and [Replay Integrity](https://github.com/nercone-labs/replay-integrity), the app is split into a core (Rust) that does nearly all the work and a thin GUI (Kotlin, Compose, Material 3 Expressive):

- `crates/lenient` — lenient reading of JSON: a field that changes its type is lost alone, never the message.
- `crates/geodesy` — coordinates, distances, projections and polygons on the sphere.
- `crates/jma` — the JMA seismic intensity scale, tsunami grades, JST, and the estimate of the shaking at a site.
- `crates/japan` — the prefectures and their outlines, the areas of the warnings and of tsunami forecasts.
- `crates/wolfx`, `crates/p2pquake` — the messages of each relay.
- `crates/firstalert` — the core: joins the reports, estimates their meaning where you are, decides how strongly to alert, and words the alerts. It is a state machine without network input or output (see below), and `firstalert-replay` runs it on saved messages.
- `crates/firstalert-ffi` — the C interface of the core (for Swift, C#, C …) and its Java Native Interface (Android, desktop JVMs).

The core performs no network access: it tells the platform which WebSockets to open, what to send and which URLs to fetch, and the platform hands back what arrives. Every platform has a mature WebSocket and HTTP client with the system's TLS, so a new platform (iOS, iPadOS, macOS, Linux, Windows) only needs a small host for these effects, its notifications and its screens. See [.docs/design.md](.docs/design.md).

## Credits

The information is that of the [Japan Meteorological Agency](https://www.jma.go.jp/), relayed by the [Wolfx Project](https://wolfx.jp/) and [P2P地震情報](https://www.p2pquake.net/). The outlines of the prefectures are from [Natural Earth](https://www.naturalearthdata.com/) (public domain).
