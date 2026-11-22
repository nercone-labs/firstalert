//! What an earthquake means where the user is.

use crate::event::{EarthquakeEvent, EewArea, Observation};
use crate::settings::Settings;
use jma::{Arrival, GroundMotion, Intensity, IntensityRange, Time};
use serde::{Deserialize, Serialize};

/// The shaking estimated at the user's place from the hypocentre and the magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EstimatedShaking {
    pub measured: f64,
    pub intensity: Intensity,
    pub epicentral_distance_km: f64,
    pub hypocentral_distance_km: f64,
    pub p_arrival: Time,
    pub s_arrival: Time,
}

/// The forecast of the warning for the user's area (or the strongest of the prefecture's).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AreaForecast {
    pub area: String,
    pub intensity: Option<IntensityRange>,
    pub arrival: Option<Time>,
    pub warning: bool,
    pub arrived: bool,
}

/// An earthquake at the user's place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalImpact {
    pub estimate: Option<EstimatedShaking>,
    pub forecast: Option<AreaForecast>,
    /// The strongest intensity observed in the user's area or prefecture.
    pub observed: Option<Intensity>,
    /// The intensity to alert with: the stronger of the estimate and the forecast.
    pub expected: Option<Intensity>,
    /// When the strong shaking arrives: the forecast's time, or the estimate's.
    pub arrival: Option<Time>,
}

impl LocalImpact {
    /// What `event` means at the place of `settings`; `None` without a place.
    pub fn of(event: &EarthquakeEvent, observations: &[Observation], settings: &Settings) -> Option<LocalImpact> {
        let place = settings.place.as_ref()?;
        let estimate = match (event.origin, event.hypocentre.epicentre, event.hypocentre.depth_km, event.hypocentre.magnitude) {
            (Some(origin), Some(epicentre), Some(depth), Some(magnitude)) => {
                GroundMotion::estimate(&place.coordinate, &epicentre, depth, magnitude, settings.ground.amplification()).and_then(|motion| {
                    let arrival = Arrival::at(motion.hypocentral_distance_km);
                    Some(EstimatedShaking {
                        measured: motion.measured_intensity,
                        intensity: Intensity::from_measured(motion.measured_intensity)?,
                        epicentral_distance_km: motion.epicentral_distance_km,
                        hypocentral_distance_km: motion.hypocentral_distance_km,
                        p_arrival: Time(origin.0 + (arrival.p_seconds * 1000.0) as i64),
                        s_arrival: Time(origin.0 + (arrival.s_seconds * 1000.0) as i64),
                    })
                })
            }
            _ => None,
        };
        let forecast = event.eew.as_ref().and_then(|eew| {
            let mine = |a: &&EewArea| match &place.area {
                Some(area) => &a.name == area,
                None => a.prefecture.is_some() && a.prefecture == place.prefecture,
            };
            eew.areas
                .iter()
                .filter(mine)
                .max_by_key(|a| (a.intensity.map(|r| r.strongest_known()), a.intensity.is_some_and(|r| r.upper.is_none())))
                .map(|a| AreaForecast { area: a.name.clone(), intensity: a.intensity, arrival: a.arrival, warning: a.warning, arrived: a.arrived })
        });
        let observed = observations
            .iter()
            .filter(|o| match &place.area {
                Some(area) if o.is_area => &o.name == area,
                _ => o.prefecture.is_some() && o.prefecture == place.prefecture,
            })
            .map(|o| o.intensity)
            .max()
            .or_else(|| event.prefectures.iter().find(|p| !p.forecast && Some(p.prefecture) == place.prefecture).map(|p| p.intensity));
        let expected = [estimate.map(|e| e.intensity), forecast.as_ref().and_then(|f| f.intensity).map(|r| r.strongest_known())].into_iter().flatten().max();
        let arrival = forecast.as_ref().and_then(|f| f.arrival).or(estimate.map(|e| e.s_arrival));
        Some(LocalImpact { estimate, forecast, observed, expected, arrival })
    }

    /// The main shaking has arrived, or is past (by the forecast, or the estimate).
    pub fn arrived(&self, now: Time) -> bool {
        self.forecast.as_ref().is_some_and(|f| f.arrived) || self.arrival.is_some_and(|a| a <= now)
    }
}
