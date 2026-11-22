//! やさしい日本語: short sentences, common words, and the reading of the harder words in
//! parentheses the first time they appear.

use super::{Composed, EventText, Phrasebook, one_decimal, parts};
use crate::alert::AlertLevel;
use crate::event::{EarthquakeEvent, TsunamiOutlook};
use crate::settings::Language;
use crate::tsunami::{SpecialEntry, TsunamiState};
use japan::Prefecture;
use jma::{Intensity, IntensityRange, Time, TsunamiGrade};

pub struct EasyJapanese;

impl EasyJapanese {
    fn grade(grade: TsunamiGrade) -> &'static str {
        match grade {
            TsunamiGrade::MajorWarning => "大津波警報（おおつなみけいほう）",
            TsunamiGrade::Warning => "津波警報（つなみけいほう）",
            TsunamiGrade::Advisory => "津波注意報（つなみちゅういほう）",
            TsunamiGrade::Forecast => "津波予報（つなみよほう）",
            TsunamiGrade::Unknown => "津波（つなみ）の 情報",
        }
    }

    fn place_of(event: &EarthquakeEvent) -> String {
        event.hypocentre.name.clone().unwrap_or_else(|| "場所（ばしょ）を 調（しら）べています".to_owned())
    }
}

impl Phrasebook for EasyJapanese {
    fn language(&self) -> Language {
        Language::EasyJapanese
    }

    fn intensity(&self, intensity: Intensity) -> String {
        format!("震度（しんど）{}", self.intensity_short(intensity))
    }

    fn intensity_short(&self, intensity: Intensity) -> String {
        match intensity {
            Intensity::Zero => "0",
            Intensity::One => "1",
            Intensity::Two => "2",
            Intensity::Three => "3",
            Intensity::Four => "4",
            Intensity::FiveLower => "5弱（じゃく）",
            Intensity::FiveUpper => "5強（きょう）",
            Intensity::SixLower => "6弱（じゃく）",
            Intensity::SixUpper => "6強（きょう）",
            Intensity::Seven => "7",
        }
        .to_owned()
    }

    fn intensity_range(&self, range: IntensityRange) -> String {
        match range.upper {
            None => format!("{}より 強（つよ）い", self.intensity(range.lower)),
            Some(upper) if upper == range.lower => self.intensity(upper),
            Some(upper) => format!("{}から {}", self.intensity(range.lower), self.intensity_short(upper)),
        }
    }

    fn intensity_feel(&self, intensity: Intensity) -> &'static str {
        match intensity {
            Intensity::Zero => "人（ひと）は ゆれが わかりません。",
            Intensity::One => "家（いえ）の 中（なか）で 静（しず）かに していると、少（すこ）し ゆれが わかります。",
            Intensity::Two => "家の 中で 静かに している 人は、ゆれが わかります。",
            Intensity::Three => "家の 中の 人は、ほとんど ゆれが わかります。",
            Intensity::Four => "とても びっくりします。電気（でんき）が 大（おお）きく ゆれます。",
            Intensity::FiveLower => "こわいです。何（なに）かに つかまりたく なります。物（もの）が 落（お）ちます。",
            Intensity::FiveUpper => "つかまらないと 歩（ある）けません。家具（かぐ）が たおれることが あります。",
            Intensity::SixLower => "立（た）って いられません。家具が 動（うご）いて、たおれます。",
            Intensity::SixUpper => "はわないと 動けません。家具の ほとんどが たおれます。",
            Intensity::Seven => "自分（じぶん）で 動けません。弱（よわ）い 建物（たてもの）は たおれます。",
        }
    }

    fn time(&self, time: Time) -> String {
        let (month, day, hour, minute) = parts(time);
        format!("{month}月（がつ）{day}日（にち） {hour}時（じ）{minute}分（ふん）")
    }

    fn magnitude(&self, magnitude: f64) -> String {
        format!("マグニチュード {}", one_decimal(magnitude))
    }

    fn depth(&self, depth_km: f64) -> String {
        if depth_km <= 0.0 { "とても 浅（あさ）い".to_owned() } else { format!("深（ふか）さ {}km", depth_km.round() as i64) }
    }

    fn prefecture(&self, prefecture: Prefecture) -> String {
        prefecture.name().to_owned()
    }

    fn event_text(&self, event: &EarthquakeEvent) -> EventText {
        let mut subtitle = Vec::new();
        if let Some(origin) = event.origin {
            subtitle.push(self.time(origin));
        }
        if let Some(m) = event.hypocentre.magnitude {
            subtitle.push(self.magnitude(m));
        }
        let status = match (&event.report, &event.eew) {
            (Some(_), _) => "地震（じしん）の 情報（じょうほう）".to_owned(),
            (None, Some(eew)) if eew.cancelled => "緊急地震速報（きんきゅうじしんそくほう）は 取（と）り消（け）し".to_owned(),
            (None, Some(_)) => "緊急地震速報（きんきゅうじしんそくほう）".to_owned(),
            (None, None) => "地震（じしん）".to_owned(),
        };
        let intensity = match (&event.report, &event.eew) {
            (Some(r), _) => r.max_intensity.map(|i| format!("一番（いちばん） 強い ゆれ {}", self.intensity(i))),
            (None, Some(e)) => e.max_intensity.map(|r| format!("{}の ゆれが 来（く）るかもしれません", self.intensity_range(r))),
            _ => None,
        };
        let local = event.local.as_ref().and_then(|l| match (l.observed, l.expected) {
            (Some(observed), _) => Some(format!("あなたの いる 所（ところ）: {}", self.intensity(observed))),
            (None, Some(expected)) if event.report.is_none() => Some(format!("あなたの いる 所: {}ぐらい", self.intensity(expected))),
            _ => None,
        });
        EventText { title: Self::place_of(event), subtitle: subtitle.join(" · "), intensity, status, local }
    }

    fn eew(&self, event: &EarthquakeEvent, level: AlertLevel, place: Option<&str>) -> Composed {
        let eew = event.eew.as_ref();
        let prefix = if eew.is_some_and(|e| e.training) { "【練習（れんしゅう）】" } else { "" };
        if eew.is_some_and(|e| e.cancelled) {
            return Composed {
                title: format!("{prefix}緊急地震速報（きんきゅうじしんそくほう）は 取（と）り消（け）しです"),
                body: "さっきの お知（し）らせは まちがいでした。強（つよ）い ゆれは 来（き）ません。".to_owned(),
                actions: None,
                speech: Some("さっきの 地震の お知らせは、まちがいでした。".to_owned()),
            };
        }
        // A forecast of no shaking says nothing of the user's place.
        let expected = event.local.as_ref().and_then(|l| l.expected).filter(|e| *e >= Intensity::One);
        let title = match level {
            AlertLevel::Emergency => format!("{prefix}地震（じしん）です。強（つよ）く ゆれます"),
            _ => format!("{prefix}地震（じしん）の お知（し）らせ"),
        };
        let mut body = vec![format!("{}で 地震（じしん）が ありました。", Self::place_of(event))];
        if let (Some(place), Some(expected)) = (place, expected) {
            body.push(format!("{place}は {}ぐらい ゆれます。", self.intensity(expected)));
        }
        let actions = (level >= AlertLevel::Warning).then(|| "頭（あたま）を 守（まも）って ください。机（つくえ）の 下（した）に 入（はい）って ください。ゆれが 止（と）まるまで 動（うご）かないで ください。".to_owned());
        let speech = (level >= AlertLevel::Warning).then(|| "地震です。強く ゆれます。頭を 守って ください。".to_owned());
        Composed { title, body: body.join(""), actions, speech }
    }

    fn report(&self, event: &EarthquakeEvent, place: Option<&str>) -> Composed {
        let Some(report) = &event.report else { return Composed::default() };
        let title = match report.max_intensity {
            Some(max) => format!("地震（じしん）が ありました {}", self.intensity(max)),
            None => "地震（じしん）が ありました".to_owned(),
        };
        let mut body = vec![format!("{}で 地震が ありました。", Self::place_of(event))];
        if let Some(max) = report.max_intensity {
            body.push(format!("一番（いちばん） 強（つよ）い ゆれは {}でした。", self.intensity(max)));
        }
        if let (Some(place), Some(observed)) = (place, event.local.as_ref().and_then(|l| l.observed)) {
            body.push(format!("{place}は {}でした。", self.intensity(observed)));
        }
        match report.tsunami {
            TsunamiOutlook::None | TsunamiOutlook::Slight => body.push("津波（つなみ）は 心配（しんぱい） ありません。".to_owned()),
            TsunamiOutlook::Advisory | TsunamiOutlook::Warning => {
                body.push("津波（つなみ）が 来（き）ます。海（うみ）から 離（はな）れて ください。".to_owned())
            }
            TsunamiOutlook::Checking => body.push("津波（つなみ）が 来るか 調（しら）べています。".to_owned()),
            TsunamiOutlook::Unknown => {}
        }
        let speech = Some(body.join(""));
        Composed { title, body: body.join(""), actions: None, speech }
    }

    fn tsunami(&self, state: &TsunamiState, place: Option<Prefecture>) -> Composed {
        if state.lifted || state.highest.is_none() {
            return Composed {
                title: "津波（つなみ）の お知（し）らせは 終（お）わりました".to_owned(),
                body: "津波の 警報（けいほう）と 注意報（ちゅういほう）は 全部（ぜんぶ） 終わりました。".to_owned(),
                actions: None,
                speech: Some("津波の お知らせは 終わりました。".to_owned()),
            };
        }
        let grade = state.local.filter(|g| g.calls_for_action()).or(state.highest).unwrap_or(TsunamiGrade::Unknown);
        let mut body = Vec::new();
        if let (Some(p), Some(_)) = (place, state.local) {
            body.push(format!("{}の 海（うみ）に {}が 出（で）ました。", p.name(), Self::grade(grade)));
        } else {
            let areas: Vec<&str> = state.areas.iter().filter(|a| a.grade.calls_for_action()).map(|a| a.name.as_str()).collect();
            body.push(format!("{}: {}。", Self::grade(grade), areas.join("、")));
        }
        let actions = grade
            .calls_for_action()
            .then(|| "海（うみ）や 川（かわ）から 離（はな）れて ください。高（たか）い 所（ところ）へ 逃（に）げて ください。".to_owned());
        Composed {
            title: Self::grade(grade).to_owned(),
            body: body.join(""),
            actions: actions.clone(),
            speech: Some(format!("津波です。{}", actions.unwrap_or_default())),
        }
    }

    fn special(&self, entry: &SpecialEntry) -> Composed {
        Composed {
            title: entry.title.clone().unwrap_or_else(|| "大（おお）きな 地震（じしん）の 情報（じょうほう）".to_owned()),
            body: "大きな 地震が 起（お）きやすく なっているかもしれません。".to_owned(),
            actions: Some(
                "逃（に）げる 場所（ばしょ）と 道（みち）を 確（たし）かめて ください。水（みず）と 食（た）べ物（もの）を 用意（ようい）して ください。"
                    .to_owned(),
            ),
            speech: Some("大きな 地震の 情報が 出ました。".to_owned()),
        }
    }

    fn test(&self, level: AlertLevel) -> Composed {
        let title = match level {
            AlertLevel::Emergency => "テスト: 地震（じしん）です。強（つよ）く ゆれます",
            _ => "テスト: お知（し）らせ",
        };
        Composed {
            title: title.to_owned(),
            body: "これは テストです。本当（ほんとう）の 地震では ありません。".to_owned(),
            actions: None,
            speech: Some("これは テストです。".to_owned()),
        }
    }
}
