use crate::Prefecture;
use std::sync::OnceLock;

/// An area of earthquake early warnings and seismic intensity reports (細分区域, such as
/// `熊本県熊本` or `石狩地方中部`), with the forecast region of the warnings it belongs to
/// (府県予報区, such as `熊本` or `北海道道央`) and its prefecture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeismicArea {
    pub name: &'static str,
    pub region: &'static str,
    pub prefecture: Prefecture,
}

/// A coastal area of tsunami forecasts (津波予報区, such as `福島県` or `有明・八代海`), with the
/// prefectures whose coast it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TsunamiArea {
    pub name: &'static str,
    pub prefectures: Vec<Prefecture>,
}

const SEISMIC: &str = include_str!("../data/areas.tsv");
const TSUNAMI: &str = include_str!("../data/tsunami_areas.tsv");

impl SeismicArea {
    /// Every area, in the order JMA lists them (by prefecture, north to south).
    pub fn all() -> &'static [SeismicArea] {
        static AREAS: OnceLock<Vec<SeismicArea>> = OnceLock::new();
        AREAS.get_or_init(|| {
            SEISMIC
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    let mut fields = line.split('\t');
                    let (prefecture, region, name) = (fields.next(), fields.next(), fields.next());
                    let prefecture = prefecture.and_then(Prefecture::from_name).expect("data/areas.tsv: unknown prefecture");
                    SeismicArea { name: name.expect("data/areas.tsv: no name"), region: region.expect("data/areas.tsv: no region"), prefecture }
                })
                .collect()
        })
    }

    pub fn find(name: &str) -> Option<&'static SeismicArea> {
        let name = name.trim();
        Self::all().iter().find(|a| a.name == name)
    }

    pub fn of(prefecture: Prefecture) -> impl Iterator<Item = &'static SeismicArea> {
        Self::all().iter().filter(move |a| a.prefecture == prefecture)
    }

    /// The prefecture of a forecast region of the warnings (`北海道道北`, `伊豆諸島`, `熊本`).
    pub fn prefecture_of_region(region: &str) -> Option<Prefecture> {
        let region = region.trim();
        Self::all().iter().find(|a| a.region == region).map(|a| a.prefecture).or_else(|| Prefecture::prefix_of(region))
    }

    /// The prefecture of an area or of a forecast region, by its name.
    pub fn prefecture_of(name: &str) -> Option<Prefecture> {
        Self::find(name).map(|a| a.prefecture).or_else(|| Self::prefecture_of_region(name))
    }
}

impl TsunamiArea {
    pub fn all() -> &'static [TsunamiArea] {
        static AREAS: OnceLock<Vec<TsunamiArea>> = OnceLock::new();
        AREAS.get_or_init(|| {
            TSUNAMI
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    let (name, prefectures) = line.split_once('\t').expect("data/tsunami_areas.tsv: no prefectures");
                    let prefectures = prefectures.split(',').map(|p| Prefecture::from_name(p).expect("data/tsunami_areas.tsv: unknown prefecture")).collect();
                    TsunamiArea { name, prefectures }
                })
                .collect()
        })
    }

    pub fn find(name: &str) -> Option<&'static TsunamiArea> {
        let name = name.trim();
        Self::all().iter().find(|a| a.name == name)
    }

    /// The prefectures whose coast an area covers, by its name. A name that is not in the list
    /// (a new or renamed area) is matched to the prefecture it begins with, if any.
    pub fn prefectures_of(name: &str) -> Vec<Prefecture> {
        match Self::find(name) {
            Some(area) => area.prefectures.clone(),
            None => Prefecture::prefix_of(name).into_iter().collect(),
        }
    }
}
