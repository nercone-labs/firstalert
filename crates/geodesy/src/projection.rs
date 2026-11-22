use crate::Coordinate;

/// A map projection from latitude and longitude to plane coordinates, `x` to the east and `y` to
/// the south (as on a screen), in units of the projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Web Mercator (EPSG:3857) normalised to `[0, 1]` across the world, the projection of most
    /// map tiles. Latitudes are clamped to ±85.0511°.
    WebMercator,
    /// Plate carrée scaled by the cosine of `reference_latitude`, so that distances near that
    /// latitude keep their proportions. `x` and `y` are in degrees of latitude.
    Equirectangular { reference_latitude: f64 },
}

impl Projection {
    /// The latitude at which Web Mercator reaches the edge of the square world.
    pub const MERCATOR_LIMIT: f64 = 85.051_128_779_806_59;

    pub fn project(&self, point: &Coordinate) -> (f64, f64) {
        match self {
            Projection::WebMercator => {
                let latitude = point.latitude.clamp(-Self::MERCATOR_LIMIT, Self::MERCATOR_LIMIT).to_radians();
                let x = (point.longitude + 180.0) / 360.0;
                let y = (1.0 - (latitude.tan() + 1.0 / latitude.cos()).ln() / std::f64::consts::PI) / 2.0;
                (x, y)
            }
            Projection::Equirectangular { reference_latitude } => (point.longitude * reference_latitude.to_radians().cos(), -point.latitude),
        }
    }

    pub fn unproject(&self, x: f64, y: f64) -> Coordinate {
        match self {
            Projection::WebMercator => {
                let longitude = x * 360.0 - 180.0;
                let n = std::f64::consts::PI * (1.0 - 2.0 * y);
                Coordinate::new(n.sinh().atan().to_degrees(), longitude)
            }
            Projection::Equirectangular { reference_latitude } => Coordinate::new(-y, x / reference_latitude.to_radians().cos()),
        }
    }
}
