//! An estimate of the seismic intensity at a site from the hypocentre and the magnitude.
//!
//! The chain is the one JMA's own earthquake early warning has long used, with published
//! empirical relations:
//!
//! 1. The peak ground velocity on the engineering bedrock (Vs = 600 m/s) by the attenuation
//!    relation of Si and Midorikawa (1999):
//!    `log PGV600 = 0.58 Mw + 0.0038 D + d − 1.29 − log(X + 0.0028·10^(0.5 Mw)) − 0.002 X`,
//!    with the depth `D` and the fault distance `X` in km, `d` = 0 (crustal earthquakes), and
//!    `Mw` at most 8.3 (the limit of the relation's data).
//! 2. Amplification to a surface of Vs = 400 m/s by 1.41 (the factor NIED's national seismic
//!    hazard maps use), times the amplification of the site relative to that surface.
//! 3. The measured intensity from the surface PGV by Fujimoto and Midorikawa (2005):
//!    `I = 2.002 + 2.603 log PGV − 0.213 (log PGV)²`.
//!
//! The JMA magnitude is taken for the moment magnitude, and the fault distance is the distance
//! to the hypocentre less half the length of the fault by Matsuda (1975), `log L = 0.6 M − 2.9`,
//! and never less than 3 km. Both choices err on the side of stronger shaking.
//!
//! The arrival times use straight rays at constant speeds (7.8 km/s for P, 4.5 km/s for S, the
//! speeds of the upper mantle). They are earlier than the true arrivals for nearly every site,
//! which is the safe side for a countdown, and are only ever a guide.

use geodesy::Coordinate;
use serde::{Deserialize, Serialize};

/// How much the ground of a site amplifies the shaking, relative to a firm surface
/// (Vs = 400 m/s), as a factor of the peak ground velocity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SiteAmplification(pub f64);

impl SiteAmplification {
    /// Firm ground (Vs ≈ 400 m/s): the reference.
    pub const FIRM: SiteAmplification = SiteAmplification(1.0);
}

impl Default for SiteAmplification {
    fn default() -> Self {
        Self::FIRM
    }
}

/// The estimated ground motion of an earthquake at one site.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundMotion {
    /// The distance from the site to the hypocentre, km.
    pub hypocentral_distance_km: f64,
    /// The distance from the site to the epicentre, km.
    pub epicentral_distance_km: f64,
    /// The peak ground velocity at the surface, cm/s.
    pub peak_ground_velocity: f64,
    /// The measured seismic intensity (計測震度) it corresponds to.
    pub measured_intensity: f64,
}

impl GroundMotion {
    /// The amplification from Vs = 600 m/s to Vs = 400 m/s.
    pub const BEDROCK_TO_SURFACE: f64 = 1.41;
    /// The largest magnitude the attenuation relation was fitted to.
    pub const MAX_MAGNITUDE: f64 = 8.3;
    /// The shortest fault distance used, km.
    pub const MIN_FAULT_DISTANCE_KM: f64 = 3.0;

    /// The estimate at `site` for an earthquake at `epicentre`, `depth_km` deep, of
    /// `magnitude`. Values that are not finite, a negative depth or a magnitude below zero give
    /// `None`: there is nothing to estimate from.
    pub fn estimate(site: &Coordinate, epicentre: &Coordinate, depth_km: f64, magnitude: f64, amplification: SiteAmplification) -> Option<GroundMotion> {
        if !(depth_km.is_finite() && depth_km >= 0.0 && magnitude.is_finite() && magnitude >= 0.0 && amplification.0.is_finite() && amplification.0 > 0.0) {
            return None;
        }
        let epicentral = site.distance_km(epicentre);
        let hypocentral = (epicentral * epicentral + depth_km * depth_km).sqrt();
        let mw = magnitude.min(Self::MAX_MAGNITUDE);
        let fault_length = 10f64.powf(0.6 * magnitude - 2.9);
        let x = (hypocentral - fault_length / 2.0).max(Self::MIN_FAULT_DISTANCE_KM);
        let log_pgv600 = 0.58 * mw + 0.0038 * depth_km - 1.29 - (x + 0.0028 * 10f64.powf(0.5 * mw)).log10() - 0.002 * x;
        let pgv = 10f64.powf(log_pgv600) * Self::BEDROCK_TO_SURFACE * amplification.0;
        Some(GroundMotion {
            hypocentral_distance_km: hypocentral,
            epicentral_distance_km: epicentral,
            peak_ground_velocity: pgv,
            measured_intensity: Self::intensity_from_pgv(pgv),
        })
    }

    /// The measured intensity of a peak ground velocity in cm/s (Fujimoto and Midorikawa, 2005).
    pub fn intensity_from_pgv(pgv: f64) -> f64 {
        let l = pgv.log10();
        2.002 + 2.603 * l - 0.213 * l * l
    }
}

/// When the waves of an earthquake reach a site, estimated (see the module documentation).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Arrival {
    /// Seconds from the origin time to the arrival of the P wave.
    pub p_seconds: f64,
    /// Seconds from the origin time to the arrival of the S wave (the strong shaking).
    pub s_seconds: f64,
}

impl Arrival {
    pub const P_SPEED_KM_S: f64 = 7.8;
    pub const S_SPEED_KM_S: f64 = 4.5;

    pub fn at(hypocentral_distance_km: f64) -> Arrival {
        Arrival { p_seconds: hypocentral_distance_km / Self::P_SPEED_KM_S, s_seconds: hypocentral_distance_km / Self::S_SPEED_KM_S }
    }

    /// How far the P and S waves have travelled `elapsed_seconds` after the origin, along the
    /// surface above a hypocentre `depth_km` deep: the radii of the circles a map draws. A wave
    /// that has not yet reached the surface has no radius.
    pub fn radii_km(elapsed_seconds: f64, depth_km: f64) -> (Option<f64>, Option<f64>) {
        let radius = |speed: f64| {
            let travelled = speed * elapsed_seconds;
            (travelled > depth_km).then(|| (travelled * travelled - depth_km * depth_km).sqrt())
        };
        (radius(Self::P_SPEED_KM_S), radius(Self::S_SPEED_KM_S))
    }
}
