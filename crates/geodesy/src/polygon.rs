use crate::Coordinate;

/// The smallest rectangle in latitude and longitude that holds a set of points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}

impl Bounds {
    /// The bounds of `points`; `None` for no points.
    pub fn of<'a>(points: impl IntoIterator<Item = &'a Coordinate>) -> Option<Bounds> {
        let mut bounds: Option<Bounds> = None;
        for point in points {
            bounds = Some(match bounds {
                None => Bounds { south: point.latitude, west: point.longitude, north: point.latitude, east: point.longitude },
                Some(b) => b.including(point),
            });
        }
        bounds
    }

    /// These bounds widened to hold `point`.
    pub fn including(&self, point: &Coordinate) -> Bounds {
        Bounds {
            south: self.south.min(point.latitude),
            west: self.west.min(point.longitude),
            north: self.north.max(point.latitude),
            east: self.east.max(point.longitude),
        }
    }

    /// The smallest bounds holding both.
    pub fn union(&self, other: &Bounds) -> Bounds {
        Bounds { south: self.south.min(other.south), west: self.west.min(other.west), north: self.north.max(other.north), east: self.east.max(other.east) }
    }

    pub fn contains(&self, point: &Coordinate) -> bool {
        (self.south..=self.north).contains(&point.latitude) && (self.west..=self.east).contains(&point.longitude)
    }

    pub fn center(&self) -> Coordinate {
        Coordinate::new((self.south + self.north) / 2.0, (self.west + self.east) / 2.0)
    }
}

/// A closed ring of points in latitude and longitude; the last point joins the first. Rings that
/// cross the antimeridian are not supported.
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    points: Vec<Coordinate>,
    bounds: Option<Bounds>,
}

impl Polygon {
    pub fn new(points: Vec<Coordinate>) -> Self {
        let bounds = Bounds::of(&points);
        Self { points, bounds }
    }

    pub fn points(&self) -> &[Coordinate] {
        &self.points
    }

    pub fn bounds(&self) -> Option<Bounds> {
        self.bounds
    }

    /// Whether `point` is inside the ring (the even-odd rule, in plate carrée coordinates, which
    /// is exact enough for rings whose edges are a few kilometres long).
    pub fn contains(&self, point: &Coordinate) -> bool {
        if !self.bounds.is_some_and(|b| b.contains(point)) {
            return false;
        }
        let (x, y) = (point.longitude, point.latitude);
        let mut inside = false;
        let mut previous = match self.points.last() {
            Some(p) => p,
            None => return false,
        };
        for current in &self.points {
            let (xi, yi, xj, yj) = (current.longitude, current.latitude, previous.longitude, previous.latitude);
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                inside = !inside;
            }
            previous = current;
        }
        inside
    }

    /// The signed area in square degrees (positive when counter-clockwise), for comparing rings.
    pub fn signed_area(&self) -> f64 {
        let n = self.points.len();
        (0..n)
            .map(|i| {
                let (a, b) = (&self.points[i], &self.points[(i + 1) % n]);
                a.longitude * b.latitude - b.longitude * a.latitude
            })
            .sum::<f64>()
            / 2.0
    }

    /// The centroid of the enclosed area; the mean of the points for a degenerate ring.
    pub fn centroid(&self) -> Option<Coordinate> {
        let n = self.points.len();
        if n == 0 {
            return None;
        }
        let area = self.signed_area();
        if area.abs() < f64::EPSILON {
            let (lat, lon) = self.points.iter().fold((0.0, 0.0), |(a, b), p| (a + p.latitude, b + p.longitude));
            return Some(Coordinate::new(lat / n as f64, lon / n as f64));
        }
        let (mut cx, mut cy) = (0.0, 0.0);
        for i in 0..n {
            let (a, b) = (&self.points[i], &self.points[(i + 1) % n]);
            let cross = a.longitude * b.latitude - b.longitude * a.latitude;
            cx += (a.longitude + b.longitude) * cross;
            cy += (a.latitude + b.latitude) * cross;
        }
        Some(Coordinate::new(cy / (6.0 * area), cx / (6.0 * area)))
    }
}
