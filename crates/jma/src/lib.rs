//! Conventions of the Japan Meteorological Agency (JMA) that every source of its information
//! shares, whoever relays it: the seismic intensity scale (震度階級), the grades of tsunami
//! warnings, Japan Standard Time, and an estimate of the intensity of the ground motion at a site.

mod estimate;
mod intensity;
mod time;
mod tsunami;

pub use estimate::{Arrival, GroundMotion, SiteAmplification};
pub use intensity::{Intensity, IntensityRange};
pub use time::Time;
pub use tsunami::TsunamiGrade;
