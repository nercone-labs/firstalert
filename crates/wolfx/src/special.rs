use jma::Time;
use lenient::Lenient;
use serde_json::Value;

/// The latest report of each kind of special earthquake information (`jma_special_eq`).
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialReports {
    pub reports: Vec<SpecialReport>,
    pub md5: Option<String>,
}

/// The product of a special report, by its JMA product code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpecialProduct {
    /// VYSE50 南海トラフ地震臨時情報: issued when an unusual event is seen along the Nankai
    /// Trough (調査中, 巨大地震警戒, 巨大地震注意, 調査終了).
    NankaiTroughExtra,
    /// VYSE51 南海トラフ地震臨時情報に関する解説情報.
    NankaiTroughExtraCommentary,
    /// VYSE52 南海トラフ地震関連解説情報, including the regular monthly commentary.
    NankaiTroughRelated,
    /// VYSE60 北海道・三陸沖後発地震注意情報: issued after a large earthquake off Hokkaido or
    /// Sanriku, when a following one is more likely than usual.
    HokkaidoSanrikuSubsequent,
    Other(String),
}

impl SpecialProduct {
    pub fn from_code(code: &str) -> SpecialProduct {
        match code {
            "VYSE50" => SpecialProduct::NankaiTroughExtra,
            "VYSE51" => SpecialProduct::NankaiTroughExtraCommentary,
            "VYSE52" => SpecialProduct::NankaiTroughRelated,
            "VYSE60" => SpecialProduct::HokkaidoSanrikuSubsequent,
            other => SpecialProduct::Other(other.to_owned()),
        }
    }

    /// Whether the product is issued only for an unusual situation, which people should hear of.
    pub fn is_extraordinary(&self) -> bool {
        matches!(self, SpecialProduct::NankaiTroughExtra | SpecialProduct::HokkaidoSanrikuSubsequent)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecialReport {
    pub id: String,
    pub product: SpecialProduct,
    pub time: Option<Time>,
    /// The report was withdrawn (`status` is `cancelled`).
    pub cancelled: bool,
    pub event_ids: Vec<String>,
    pub kind: Option<String>,
    /// The name of the information number, such as `巨大地震注意`.
    pub label: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
}

impl SpecialReports {
    pub fn from_value(value: &Value) -> SpecialReports {
        let reports = value
            .elements(&["reports"])
            .iter()
            .filter_map(|report| {
                Some(SpecialReport {
                    id: report.text(&["id"])?,
                    product: SpecialProduct::from_code(&report.text(&["product"]).unwrap_or_default()),
                    time: report.text(&["time"]).and_then(|t| Time::parse_rfc3339(&t)),
                    cancelled: report.text(&["status"]).is_some_and(|s| s == "cancelled"),
                    event_ids: report.elements(&["eventIds"]).iter().filter_map(|e| e.as_str().map(str::to_owned)).collect(),
                    kind: report.text(&["kind"]),
                    label: report.text(&["label"]),
                    title: report.text(&["title"]),
                    text: report.text(&["text"]),
                })
            })
            .collect();
        SpecialReports { reports, md5: value.text(&["md5"]) }
    }
}
