//! Geodesy on a sphere of the mean radius of the Earth.
//!
//! The precision is that of the spherical model (about 0.5 % of a distance), which is far finer
//! than the uncertainty of the location of an earthquake that it is used with.

mod coordinate;
mod polygon;
mod projection;

pub use coordinate::Coordinate;
pub use polygon::{Bounds, Polygon};
pub use projection::Projection;
