use serde::{Deserialize, Serialize};

/// A point on the surface of the Earth, in degrees: latitude north and longitude east positive.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinate {
    /// The mean radius of the Earth in kilometres (IUGG, `R1`).
    pub const EARTH_RADIUS_KM: f64 = 6371.0088;

    pub const fn new(latitude: f64, longitude: f64) -> Self {
        Self { latitude, longitude }
    }

    /// The coordinate, when both values are finite and within their ranges.
    pub fn checked(latitude: f64, longitude: f64) -> Option<Self> {
        let valid = latitude.is_finite() && longitude.is_finite() && (-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude);
        valid.then_some(Self { latitude, longitude })
    }

    /// The great-circle distance to `other` in kilometres (the haversine formula, which stays
    /// precise for short distances).
    pub fn distance_km(&self, other: &Coordinate) -> f64 {
        let (phi1, phi2) = (self.latitude.to_radians(), other.latitude.to_radians());
        let dphi = phi2 - phi1;
        let dlambda = (other.longitude - self.longitude).to_radians();
        let a = (dphi / 2.0).sin().powi(2) + phi1.cos() * phi2.cos() * (dlambda / 2.0).sin().powi(2);
        2.0 * Self::EARTH_RADIUS_KM * a.sqrt().min(1.0).asin()
    }

    /// The initial bearing towards `other`, in degrees clockwise from north, in `[0, 360)`.
    pub fn bearing_to(&self, other: &Coordinate) -> f64 {
        let (phi1, phi2) = (self.latitude.to_radians(), other.latitude.to_radians());
        let dlambda = (other.longitude - self.longitude).to_radians();
        let y = dlambda.sin() * phi2.cos();
        let x = phi1.cos() * phi2.sin() - phi1.sin() * phi2.cos() * dlambda.cos();
        (y.atan2(x).to_degrees() + 360.0) % 360.0
    }

    /// The point `distance_km` away along the great circle that leaves at `bearing` degrees.
    pub fn destination(&self, bearing: f64, distance_km: f64) -> Coordinate {
        let delta = distance_km / Self::EARTH_RADIUS_KM;
        let theta = bearing.to_radians();
        let phi1 = self.latitude.to_radians();
        let lambda1 = self.longitude.to_radians();
        let phi2 = (phi1.sin() * delta.cos() + phi1.cos() * delta.sin() * theta.cos()).asin();
        let lambda2 = lambda1 + (theta.sin() * delta.sin() * phi1.cos()).atan2(delta.cos() - phi1.sin() * phi2.sin());
        Coordinate { latitude: phi2.to_degrees(), longitude: ((lambda2.to_degrees() + 540.0) % 360.0) - 180.0 }
    }

    /// The distance in kilometres to a point `depth_km` below `other`: the straight line through
    /// the Earth, for the distance from a site to a hypocentre.
    pub fn slant_distance_km(&self, other: &Coordinate, depth_km: f64) -> f64 {
        let surface = self.distance_km(other);
        (surface * surface + depth_km * depth_km).sqrt()
    }
}
