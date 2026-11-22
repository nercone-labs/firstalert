use super::{Composed, EventText, Phrasebook, one_decimal, parts};
use crate::alert::AlertLevel;
use crate::event::{EarthquakeEvent, ReportKind, TsunamiOutlook};
use crate::settings::Language;
use crate::tsunami::{SpecialEntry, TsunamiState};
use japan::Prefecture;
use jma::{Intensity, IntensityRange, Time, TsunamiGrade};

pub struct Japanese;

impl Japanese {
    fn grade(grade: TsunamiGrade) -> &'static str {
        match grade {
            TsunamiGrade::MajorWarning => "大津波警報",
            TsunamiGrade::Warning => "津波警報",
            TsunamiGrade::Advisory => "津波注意報",
            TsunamiGrade::Forecast => "津波予報（若干の海面変動）",
            TsunamiGrade::Unknown => "津波の情報",
        }
    }

    fn place_of(event: &EarthquakeEvent) -> String {
        event.hypocentre.name.clone().unwrap_or_else(|| "震源は調査中".to_owned())
    }

    fn outlook(outlook: TsunamiOutlook) -> Option<&'static str> {
        match outlook {
            TsunamiOutlook::None => Some("この地震による津波の心配はありません。"),
            TsunamiOutlook::Slight => Some("若干の海面変動があるかもしれませんが、被害の心配はありません。"),
            TsunamiOutlook::Checking => Some("津波があるかどうかを調べています。海の近くでは今後の情報に注意してください。"),
            TsunamiOutlook::Advisory => Some("津波注意報が出ています。海からすぐ離れてください。"),
            TsunamiOutlook::Warning => Some("津波警報が出ています。すぐに高い所へ避難してください。"),
            TsunamiOutlook::Unknown => None,
        }
    }
}

impl Phrasebook for Japanese {
    fn language(&self) -> Language {
        Language::Japanese
    }

    fn intensity(&self, intensity: Intensity) -> String {
        format!("震度{}", self.intensity_short(intensity))
    }

    fn intensity_short(&self, intensity: Intensity) -> String {
        match intensity {
            Intensity::Zero => "0",
            Intensity::One => "1",
            Intensity::Two => "2",
            Intensity::Three => "3",
            Intensity::Four => "4",
            Intensity::FiveLower => "5弱",
            Intensity::FiveUpper => "5強",
            Intensity::SixLower => "6弱",
            Intensity::SixUpper => "6強",
            Intensity::Seven => "7",
        }
        .to_owned()
    }

    fn intensity_range(&self, range: IntensityRange) -> String {
        match range.upper {
            None => format!("{}以上", self.intensity(range.lower)),
            Some(upper) if upper == range.lower => self.intensity(upper),
            Some(upper) => format!("{}から{}", self.intensity(range.lower), self.intensity_short(upper)),
        }
    }

    fn intensity_feel(&self, intensity: Intensity) -> &'static str {
        match intensity {
            Intensity::Zero => "人は揺れを感じません。",
            Intensity::One => "屋内で静かにしている人の中には、揺れをわずかに感じる人がいます。",
            Intensity::Two => "屋内で静かにしている人の多くが揺れを感じます。電灯などのつり下げ物がわずかに揺れます。",
            Intensity::Three => "屋内にいるほとんどの人が揺れを感じます。棚の食器が音を立てることがあります。",
            Intensity::Four => "ほとんどの人が驚きます。つり下げ物が大きく揺れ、座りの悪い置物が倒れることがあります。",
            Intensity::FiveLower => "多くの人が恐怖を覚え、物につかまりたくなります。棚の食器や本が落ちることがあります。",
            Intensity::FiveUpper => "物につかまらないと歩くことが難しくなります。固定していない家具が倒れることがあります。",
            Intensity::SixLower => "立っていることが難しくなります。固定していない家具の多くが動き、倒れるものもあります。",
            Intensity::SixUpper => "はわないと動くことができません。固定していない家具のほとんどが動き、倒れます。",
            Intensity::Seven => "揺れにほんろうされ、自分の意思で動けません。耐震性の低い建物は倒れるものが多くなります。",
        }
    }

    fn time(&self, time: Time) -> String {
        let (month, day, hour, minute) = parts(time);
        format!("{month}月{day}日 {hour}時{minute:02}分")
    }

    fn magnitude(&self, magnitude: f64) -> String {
        format!("M{}", one_decimal(magnitude))
    }

    fn depth(&self, depth_km: f64) -> String {
        if depth_km <= 0.0 { "ごく浅い".to_owned() } else { format!("深さ{}km", depth_km.round() as i64) }
    }

    fn prefecture(&self, prefecture: Prefecture) -> String {
        prefecture.name().to_owned()
    }

    fn event_text(&self, event: &EarthquakeEvent) -> EventText {
        let mut subtitle = Vec::new();
        if let Some(origin) = event.origin {
            subtitle.push(format!("{}ごろ", self.time(origin)));
        }
        if let Some(m) = event.hypocentre.magnitude {
            subtitle.push(self.magnitude(m));
        }
        if let Some(d) = event.hypocentre.depth_km {
            subtitle.push(self.depth(d));
        }
        let status = match (&event.report, &event.eew) {
            (Some(report), _) => match report.kind {
                ReportKind::Intensities => "震度速報",
                ReportKind::Hypocentre => "震源に関する情報",
                ReportKind::HypocentreAndIntensities => "震源・震度に関する情報",
                ReportKind::Detail => "各地の震度に関する情報",
                ReportKind::Foreign => "遠地地震に関する情報",
                ReportKind::Other => "地震情報",
            }
            .to_owned(),
            (None, Some(eew)) => {
                let kind = if eew.cancelled {
                    "緊急地震速報（取消）".to_owned()
                } else if eew.ever_warning {
                    "緊急地震速報（警報）".to_owned()
                } else {
                    "緊急地震速報（予報）".to_owned()
                };
                let serial = if eew.final_report { format!("第{}報・最終", eew.serial) } else { format!("第{}報", eew.serial) };
                let training = if eew.training { "【訓練】" } else { "" };
                format!("{training}{kind} {serial}")
            }
            (None, None) => "地震".to_owned(),
        };
        let intensity = match (&event.report, &event.eew) {
            (Some(r), _) => r.max_intensity.map(|i| format!("最大{}", self.intensity(i))),
            (None, Some(e)) => e.max_intensity.map(|r| format!("予想 最大{}", self.intensity_range(r))),
            _ => None,
        };
        let local = event.local.as_ref().and_then(|l| match (l.observed, l.expected) {
            (Some(observed), _) => Some(format!("あなたの地域: {}", self.intensity(observed))),
            (None, Some(expected)) if event.report.is_none() => Some(format!("あなたの地域: {}程度の予想", self.intensity(expected))),
            _ => None,
        });
        EventText { title: Self::place_of(event), subtitle: subtitle.join(" · "), intensity, status, local }
    }

    fn eew(&self, event: &EarthquakeEvent, level: AlertLevel, place: Option<&str>) -> Composed {
        let eew = event.eew.as_ref();
        let where_ = Self::place_of(event);
        let training = eew.is_some_and(|e| e.training);
        let prefix = if training { "【訓練】" } else { "" };
        if eew.is_some_and(|e| e.cancelled) {
            return Composed {
                title: format!("{prefix}緊急地震速報は取り消されました"),
                body: "先ほどの緊急地震速報は取り消されました。この地震による強い揺れの心配はありません。".to_owned(),
                actions: None,
                speech: Some("先ほどの緊急地震速報は、取り消されました。".to_owned()),
            };
        }
        // A forecast of no shaking says nothing of the user's place.
        let expected = event.local.as_ref().and_then(|l| l.expected).filter(|e| *e >= Intensity::One);
        let warning = eew.is_some_and(|e| e.warning || e.ever_warning);
        let title = match level {
            AlertLevel::Emergency => format!("{prefix}緊急地震速報 強い揺れに警戒"),
            _ if warning => format!("{prefix}緊急地震速報（警報）"),
            _ => format!("{prefix}緊急地震速報（予報）"),
        };
        let mut body = vec![format!("{where_}で地震が発生しました。")];
        match (place, expected) {
            (Some(place), Some(expected)) => body.push(format!("{place}は{}程度の揺れが予想されます。", self.intensity(expected))),
            _ => {
                if let Some(max) = eew.and_then(|e| e.max_intensity) {
                    body.push(format!("最も強い所で{}程度の揺れが予想されます。", self.intensity_range(max)));
                }
            }
        }
        if let Some(m) = event.hypocentre.magnitude {
            body.push(format!("地震の規模は{}（推定）。", self.magnitude(m)));
        }
        let actions = (level >= AlertLevel::Warning).then(|| {
            "頭を守ってください。机の下など、物が落ちてこない、倒れてこない、移動してこない所で、揺れがおさまるまで待ってください。車を運転中は、ハザードランプをつけて、ゆっくり止まってください。".to_owned()
        });
        let speech = match (level, place, expected) {
            (AlertLevel::Emergency, Some(place), Some(expected)) => Some(format!(
                "緊急地震速報。強い揺れに警戒してください。{where_}で地震。{place}は、{}程度の予想です。頭を守ってください。",
                self.intensity(expected)
            )),
            (AlertLevel::Emergency, _, _) => Some(format!("緊急地震速報。強い揺れに警戒してください。{where_}で地震。頭を守ってください。")),
            (AlertLevel::Warning, _, _) => Some(format!("緊急地震速報。{where_}で地震。揺れに注意してください。")),
            _ => None,
        };
        Composed { title, body: body.join(""), actions, speech }
    }

    fn report(&self, event: &EarthquakeEvent, place: Option<&str>) -> Composed {
        let Some(report) = &event.report else { return Composed::default() };
        let max = report.max_intensity.map(|i| self.intensity(i));
        let title = match (report.kind, &max) {
            (ReportKind::Intensities, Some(max)) => format!("震度速報 最大{max}"),
            (ReportKind::Foreign, _) => "遠地地震に関する情報".to_owned(),
            (_, Some(max)) => format!("地震情報 最大{max} {}", Self::place_of(event)),
            (_, None) => format!("地震情報 {}", Self::place_of(event)),
        };
        let mut body = Vec::new();
        let when = event.origin.map(|t| format!("{}ごろ、", self.time(t))).unwrap_or_default();
        match &event.hypocentre.name {
            Some(name) => body.push(format!("{when}{name}で地震がありました。")),
            None => body.push(format!("{when}地震がありました。震源は調べています。")),
        }
        if let Some(max) = &max {
            body.push(format!("最も強い揺れは{max}でした。"));
        }
        let mut detail = Vec::new();
        if let Some(m) = event.hypocentre.magnitude {
            detail.push(format!("規模は{}", self.magnitude(m)));
        }
        if let Some(d) = event.hypocentre.depth_km {
            detail.push(if d <= 0.0 { "震源はごく浅い".to_owned() } else { format!("震源の{}", self.depth(d)) });
        }
        if !detail.is_empty() {
            body.push(format!("{}。", detail.join("、")));
        }
        if let (Some(place), Some(observed)) = (place, event.local.as_ref().and_then(|l| l.observed)) {
            body.push(format!("{place}では{}でした。", self.intensity(observed)));
        }
        if let Some(outlook) = Self::outlook(report.tsunami) {
            body.push(outlook.to_owned());
        }
        let strong = report.max_intensity.is_some_and(|i| i >= Intensity::FiveLower)
            || event.local.as_ref().and_then(|l| l.observed).is_some_and(|i| i >= Intensity::Four);
        let actions = strong.then(|| "けがをしていないか、火が出ていないかを確かめてください。しばらくは同じくらいの地震に注意してください。".to_owned());
        let speech = Some(format!("地震情報。{}", body.join("")));
        Composed { title, body: body.join(""), actions, speech }
    }

    fn tsunami(&self, state: &TsunamiState, place: Option<Prefecture>) -> Composed {
        if state.lifted || state.highest.is_none() {
            return Composed {
                title: "津波の警報・注意報は解除されました".to_owned(),
                body: "津波警報・注意報はすべて解除されました。海の近くでは、しばらく海面の変動に注意してください。".to_owned(),
                actions: None,
                speech: Some("津波の警報と注意報は、すべて解除されました。".to_owned()),
            };
        }
        let highest = state.highest.unwrap_or(TsunamiGrade::Unknown);
        let local = state.local;
        let title = match local {
            Some(grade) if grade.calls_for_action() => format!("{}（あなたの地域）", Self::grade(grade)),
            _ => Self::grade(highest).to_owned(),
        };
        let mut body = Vec::new();
        if let (Some(p), Some(grade)) = (place, local) {
            let areas: Vec<&str> = state.areas.iter().filter(|a| a.prefectures.contains(&p) && a.grade == grade).map(|a| a.name.as_str()).collect();
            body.push(format!("{}の沿岸（{}）に{}が出ています。", self.prefecture(p), areas.join("、"), Self::grade(grade)));
        }
        for grade in [TsunamiGrade::MajorWarning, TsunamiGrade::Warning, TsunamiGrade::Advisory] {
            let areas: Vec<&str> = state.areas.iter().filter(|a| a.grade == grade).map(|a| a.name.as_str()).collect();
            if !areas.is_empty() {
                body.push(format!("{}: {}。", Self::grade(grade), areas.join("、")));
            }
        }
        let actions = match local.or(Some(highest)) {
            Some(TsunamiGrade::MajorWarning | TsunamiGrade::Warning) => Some(
                "海や川の近くにいる人は、すぐにできるだけ高い所へ避難してください。津波は何度も来ます。警報が解除されるまで戻らないでください。".to_owned(),
            ),
            Some(TsunamiGrade::Advisory) => Some("海の中や海岸にいる人は、すぐに海から上がって、海岸から離れてください。".to_owned()),
            _ => None,
        };
        let speech = match local {
            Some(grade) if grade.calls_for_action() => Some(format!(
                "{}。{}の沿岸です。{}",
                Self::grade(grade),
                place.map(|p| self.prefecture(p)).unwrap_or_default(),
                actions.clone().unwrap_or_default()
            )),
            _ => Some(format!("{}が出ています。", Self::grade(highest))),
        };
        Composed { title, body: body.join(""), actions, speech }
    }

    fn special(&self, entry: &SpecialEntry) -> Composed {
        let title = entry.title.clone().unwrap_or_else(|| "地震に関する特別な情報".to_owned());
        let first = entry.text.as_deref().map(|t| t.split('。').next().unwrap_or("").trim().to_owned() + "。").filter(|t| t.len() > 3);
        let body = [entry.label.clone(), first].into_iter().flatten().collect::<Vec<_>>().join(" ");
        Composed {
            title: title.clone(),
            body,
            actions: Some("気象庁や自治体の呼びかけを確かめてください。家具の固定、避難の場所と経路、水と食べ物の備えをもう一度確かめてください。".to_owned()),
            speech: Some(format!("{title}が発表されました。")),
        }
    }

    fn test(&self, level: AlertLevel) -> Composed {
        let (title, speech) = match level {
            AlertLevel::Emergency => ("テスト: 緊急地震速報 強い揺れに警戒", "これはテストです。緊急地震速報の時は、このようにお知らせします。"),
            AlertLevel::Warning => ("テスト: 揺れに注意", "これはテストです。揺れに注意が必要な時は、このようにお知らせします。"),
            _ => ("テスト: お知らせ", "これはテストです。"),
        };
        Composed {
            title: title.to_owned(), body: "これはテストです。実際の地震ではありません。".to_owned(), actions: None, speech: Some(speech.to_owned())
        }
    }
}
