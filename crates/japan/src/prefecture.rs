use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A prefecture of Japan, by its JIS X 0401 code (1 Hokkaido … 47 Okinawa).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Prefecture(u8);

/// The names of the prefectures in the order of their codes: Japanese, and English.
const NAMES: [(&str, &str); 47] = [
    ("北海道", "Hokkaido"),
    ("青森県", "Aomori"),
    ("岩手県", "Iwate"),
    ("宮城県", "Miyagi"),
    ("秋田県", "Akita"),
    ("山形県", "Yamagata"),
    ("福島県", "Fukushima"),
    ("茨城県", "Ibaraki"),
    ("栃木県", "Tochigi"),
    ("群馬県", "Gunma"),
    ("埼玉県", "Saitama"),
    ("千葉県", "Chiba"),
    ("東京都", "Tokyo"),
    ("神奈川県", "Kanagawa"),
    ("新潟県", "Niigata"),
    ("富山県", "Toyama"),
    ("石川県", "Ishikawa"),
    ("福井県", "Fukui"),
    ("山梨県", "Yamanashi"),
    ("長野県", "Nagano"),
    ("岐阜県", "Gifu"),
    ("静岡県", "Shizuoka"),
    ("愛知県", "Aichi"),
    ("三重県", "Mie"),
    ("滋賀県", "Shiga"),
    ("京都府", "Kyoto"),
    ("大阪府", "Osaka"),
    ("兵庫県", "Hyogo"),
    ("奈良県", "Nara"),
    ("和歌山県", "Wakayama"),
    ("鳥取県", "Tottori"),
    ("島根県", "Shimane"),
    ("岡山県", "Okayama"),
    ("広島県", "Hiroshima"),
    ("山口県", "Yamaguchi"),
    ("徳島県", "Tokushima"),
    ("香川県", "Kagawa"),
    ("愛媛県", "Ehime"),
    ("高知県", "Kochi"),
    ("福岡県", "Fukuoka"),
    ("佐賀県", "Saga"),
    ("長崎県", "Nagasaki"),
    ("熊本県", "Kumamoto"),
    ("大分県", "Oita"),
    ("宮崎県", "Miyazaki"),
    ("鹿児島県", "Kagoshima"),
    ("沖縄県", "Okinawa"),
];

impl Prefecture {
    pub const COUNT: u8 = 47;

    pub fn from_code(code: u8) -> Option<Prefecture> {
        (1..=Self::COUNT).contains(&code).then_some(Prefecture(code))
    }

    pub fn all() -> impl Iterator<Item = Prefecture> {
        (1..=Self::COUNT).map(Prefecture)
    }

    pub fn code(&self) -> u8 {
        self.0
    }

    /// The Japanese name, with its suffix: `北海道`, `東京都`, `大阪府`, `熊本県`.
    pub fn name(&self) -> &'static str {
        NAMES[self.0 as usize - 1].0
    }

    /// The name in English, without a suffix: `Tokyo`, `Kumamoto`.
    pub fn english_name(&self) -> &'static str {
        NAMES[self.0 as usize - 1].1
    }

    /// The Japanese name without its suffix (`東京`, `熊本`; `北海道` keeps its name).
    pub fn short_name(&self) -> &'static str {
        let name = self.name();
        if self.0 == 1 {
            return name;
        }
        name.strip_suffix(['都', '府', '県']).unwrap_or(name)
    }

    /// The prefecture of its Japanese name, with or without the suffix.
    pub fn from_name(name: &str) -> Option<Prefecture> {
        let name = name.trim();
        Self::all().find(|p| p.name() == name || p.short_name() == name)
    }

    /// The prefecture whose name `text` begins with (`熊本県熊本` → 熊本県, `北海道道北` →
    /// 北海道, `東京` → 東京都). The longest name wins, so that no prefecture is taken for one
    /// whose name is a prefix of its own.
    pub fn prefix_of(text: &str) -> Option<Prefecture> {
        let text = text.trim();
        Self::all()
            .flat_map(|p| [(p, p.name()), (p, p.short_name())])
            .filter(|(_, name)| text.starts_with(name))
            .max_by_key(|(_, name)| name.len())
            .map(|(p, _)| p)
    }
}

impl Serialize for Prefecture {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for Prefecture {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let code = u8::deserialize(deserializer)?;
        Prefecture::from_code(code).ok_or_else(|| serde::de::Error::custom(format!("not a prefecture code: {code}")))
    }
}
