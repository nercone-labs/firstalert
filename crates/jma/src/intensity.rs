use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A class of the JMA seismic intensity scale (気象庁震度階級, ten classes since 1996).
///
/// The order of the variants is the order of the classes, so classes compare as intensities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Intensity {
    Zero,
    One,
    Two,
    Three,
    Four,
    /// 5弱
    FiveLower,
    /// 5強
    FiveUpper,
    /// 6弱
    SixLower,
    /// 6強
    SixUpper,
    Seven,
}

impl Intensity {
    pub const ALL: [Intensity; 10] = [
        Intensity::Zero,
        Intensity::One,
        Intensity::Two,
        Intensity::Three,
        Intensity::Four,
        Intensity::FiveLower,
        Intensity::FiveUpper,
        Intensity::SixLower,
        Intensity::SixUpper,
        Intensity::Seven,
    ];

    /// The class of a measured seismic intensity (計測震度). JMA rounds the measured value to one
    /// decimal place first, and the classes begin at 0.5, 1.5, 2.5, 3.5, 4.5, 5.0, 5.5, 6.0 and
    /// 6.5. Not a number is `None`.
    pub fn from_measured(measured: f64) -> Option<Intensity> {
        if measured.is_nan() {
            return None;
        }
        // Rounded to one decimal place, in tenths, so that the thresholds compare exactly.
        let tenths = (measured * 10.0).round();
        let class = match tenths {
            t if t < 5.0 => Intensity::Zero,
            t if t < 15.0 => Intensity::One,
            t if t < 25.0 => Intensity::Two,
            t if t < 35.0 => Intensity::Three,
            t if t < 45.0 => Intensity::Four,
            t if t < 50.0 => Intensity::FiveLower,
            t if t < 55.0 => Intensity::FiveUpper,
            t if t < 60.0 => Intensity::SixLower,
            t if t < 65.0 => Intensity::SixUpper,
            _ => Intensity::Seven,
        };
        Some(class)
    }

    /// The smallest measured intensity of the class.
    pub fn lower_bound(&self) -> f64 {
        match self {
            Intensity::Zero => f64::NEG_INFINITY,
            Intensity::One => 0.5,
            Intensity::Two => 1.5,
            Intensity::Three => 2.5,
            Intensity::Four => 3.5,
            Intensity::FiveLower => 4.5,
            Intensity::FiveUpper => 5.0,
            Intensity::SixLower => 5.5,
            Intensity::SixUpper => 6.0,
            Intensity::Seven => 6.5,
        }
    }

    /// The short code of the class: `0`–`4`, `5-`, `5+`, `6-`, `6+`, `7`. It is the form JMA uses
    /// in its own JSON and XML, and the form this type is serialised as.
    pub fn code(&self) -> &'static str {
        match self {
            Intensity::Zero => "0",
            Intensity::One => "1",
            Intensity::Two => "2",
            Intensity::Three => "3",
            Intensity::Four => "4",
            Intensity::FiveLower => "5-",
            Intensity::FiveUpper => "5+",
            Intensity::SixLower => "6-",
            Intensity::SixUpper => "6+",
            Intensity::Seven => "7",
        }
    }

    /// Reads a class written in any of the usual forms: the code (`5-`), the Japanese name with
    /// ASCII or full-width digits (`5弱`, `５強`), optionally after `震度`. Text that names no
    /// class, such as `不明`, is `None`.
    pub fn parse(text: &str) -> Option<Intensity> {
        let text = text.trim();
        let text = text.strip_prefix("震度").unwrap_or(text).trim();
        let normalised: String = text
            .chars()
            .map(|c| match c {
                '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap_or(c),
                '弱' | '－' | '−' => '-',
                '強' | '＋' => '+',
                c => c,
            })
            .collect();
        let class = match normalised.as_str() {
            "0" => Intensity::Zero,
            "1" => Intensity::One,
            "2" => Intensity::Two,
            "3" => Intensity::Three,
            "4" => Intensity::Four,
            "5-" => Intensity::FiveLower,
            "5+" => Intensity::FiveUpper,
            "6-" => Intensity::SixLower,
            "6+" => Intensity::SixUpper,
            "7" => Intensity::Seven,
            _ => return None,
        };
        Some(class)
    }

    /// The numeric scale of the P2P Quake API (`10` for 1, `45` for 5弱, …, `70` for 7, and `0`
    /// for 0). `46` (5弱 or more, estimated, not received) is read as 5弱, its lower bound; other
    /// values are `None`.
    pub fn from_scale(scale: i64) -> Option<Intensity> {
        let class = match scale {
            0 => Intensity::Zero,
            10 => Intensity::One,
            20 => Intensity::Two,
            30 => Intensity::Three,
            40 => Intensity::Four,
            45 | 46 => Intensity::FiveLower,
            50 => Intensity::FiveUpper,
            55 => Intensity::SixLower,
            60 => Intensity::SixUpper,
            70 => Intensity::Seven,
            _ => return None,
        };
        Some(class)
    }
}

impl fmt::Display for Intensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl Serialize for Intensity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Intensity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Intensity::parse(&text).ok_or_else(|| serde::de::Error::custom(format!("not a seismic intensity: {text}")))
    }
}

/// A range of intensity classes, as the forecasts of an earthquake early warning give them:
/// from `lower`, up to `upper`, or with no upper bound (`〜程度以上`) when `upper` is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntensityRange {
    pub lower: Intensity,
    pub upper: Option<Intensity>,
}

impl IntensityRange {
    pub fn exactly(intensity: Intensity) -> Self {
        Self { lower: intensity, upper: Some(intensity) }
    }

    pub fn at_least(intensity: Intensity) -> Self {
        Self { lower: intensity, upper: None }
    }

    /// The highest class the range may reach; `None` when it has no upper bound.
    pub fn highest(&self) -> Option<Intensity> {
        self.upper.map(|u| u.max(self.lower))
    }

    /// The class to warn with: the upper bound, or the lower bound of an open range. A warning
    /// takes the strongest shaking the forecast admits.
    pub fn strongest_known(&self) -> Intensity {
        self.highest().unwrap_or(self.lower)
    }
}
