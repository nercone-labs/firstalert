use crate::Prefecture;
use geodesy::{Bounds, Coordinate, Polygon};
use std::fmt;
use std::sync::OnceLock;

/// The outline of one prefecture: its islands, each a ring.
#[derive(Debug, Clone, PartialEq)]
pub struct PrefectureShape {
    pub prefecture: Prefecture,
    pub rings: Vec<Polygon>,
    pub bounds: Bounds,
}

impl PrefectureShape {
    pub fn contains(&self, point: &Coordinate) -> bool {
        self.bounds.contains(point) && self.rings.iter().any(|r| r.contains(point))
    }

    /// A point that stands for the prefecture: the centroid of its largest ring.
    pub fn representative_point(&self) -> Coordinate {
        self.rings
            .iter()
            .max_by(|a, b| a.signed_area().abs().total_cmp(&b.signed_area().abs()))
            .and_then(Polygon::centroid)
            .unwrap_or_else(|| self.bounds.center())
    }
}

/// The outlines of the prefectures, simplified for maps of the whole country or of a region.
///
/// The binary format (`data/prefectures.bin`, all integers little-endian): the magic `FAJP`, the
/// version `1` (one byte), the number of prefectures (one byte); then for each prefecture its
/// code (one byte) and the number of rings (u16), and for each ring the number of points (u16)
/// followed by the points, each a longitude and a latitude in units of 10⁻⁴ degree (i32 each).
/// A ring is closed: its last point joins its first.
#[derive(Debug, Clone, PartialEq)]
pub struct Boundaries {
    pub shapes: Vec<PrefectureShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryError {
    Magic,
    Version(u8),
    Truncated,
    Prefecture(u8),
}

impl fmt::Display for BoundaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoundaryError::Magic => f.write_str("not a FAJP file"),
            BoundaryError::Version(v) => write!(f, "unsupported FAJP version {v}"),
            BoundaryError::Truncated => f.write_str("the FAJP file ends early"),
            BoundaryError::Prefecture(code) => write!(f, "unknown prefecture code {code}"),
        }
    }
}

impl std::error::Error for BoundaryError {}

const BUILT_IN: &[u8] = include_bytes!("../data/prefectures.bin");

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], BoundaryError> {
        if self.bytes.len() < n {
            return Err(BoundaryError::Truncated);
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, BoundaryError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, BoundaryError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn i32(&mut self) -> Result<i32, BoundaryError> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

impl Boundaries {
    pub const VERSION: u8 = 1;

    /// The outlines built into the crate, read once.
    pub fn built_in() -> &'static Boundaries {
        static BUILT: OnceLock<Boundaries> = OnceLock::new();
        BUILT.get_or_init(|| Boundaries::parse(BUILT_IN).expect("data/prefectures.bin is valid"))
    }

    pub fn parse(bytes: &[u8]) -> Result<Boundaries, BoundaryError> {
        let mut reader = Reader { bytes };
        if reader.take(4)? != b"FAJP" {
            return Err(BoundaryError::Magic);
        }
        let version = reader.u8()?;
        if version != Self::VERSION {
            return Err(BoundaryError::Version(version));
        }
        let count = reader.u8()?;
        let mut shapes = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let code = reader.u8()?;
            let prefecture = Prefecture::from_code(code).ok_or(BoundaryError::Prefecture(code))?;
            let ring_count = reader.u16()?;
            let mut rings = Vec::with_capacity(ring_count as usize);
            for _ in 0..ring_count {
                let point_count = reader.u16()?;
                let mut points = Vec::with_capacity(point_count as usize);
                for _ in 0..point_count {
                    let longitude = reader.i32()? as f64 / 1e4;
                    let latitude = reader.i32()? as f64 / 1e4;
                    points.push(Coordinate::new(latitude, longitude));
                }
                rings.push(Polygon::new(points));
            }
            let bounds = rings.iter().filter_map(Polygon::bounds).reduce(|a, b| a.union(&b)).ok_or(BoundaryError::Truncated)?;
            shapes.push(PrefectureShape { prefecture, rings, bounds });
        }
        Ok(Boundaries { shapes })
    }

    pub fn shape(&self, prefecture: Prefecture) -> Option<&PrefectureShape> {
        self.shapes.iter().find(|s| s.prefecture == prefecture)
    }

    /// The prefecture that holds `point`; `None` at sea or abroad.
    pub fn locate(&self, point: &Coordinate) -> Option<Prefecture> {
        self.shapes.iter().find(|s| s.contains(point)).map(|s| s.prefecture)
    }

    /// The prefecture that holds `point`, or else the one whose outline is nearest to it within
    /// `max_km` (for points on a coast that the simplified outline leaves just outside).
    pub fn locate_near(&self, point: &Coordinate, max_km: f64) -> Option<Prefecture> {
        self.locate(point).or_else(|| {
            self.shapes
                .iter()
                .flat_map(|s| s.rings.iter().flat_map(|r| r.points()).map(move |p| (s.prefecture, point.distance_km(p))))
                .filter(|(_, d)| *d <= max_km)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(p, _)| p)
        })
    }

    pub fn bounds(&self) -> Option<Bounds> {
        self.shapes.iter().map(|s| s.bounds).reduce(|a, b| a.union(&b))
    }
}
