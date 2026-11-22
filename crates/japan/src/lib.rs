//! Japan as the information about earthquakes names it: the 47 prefectures with their
//! boundaries, the areas of earthquake early warnings and seismic intensity reports
//! (緊急地震速報や震度情報で用いる区域), and the coastal areas of tsunami forecasts (津波予報区).
//!
//! The data is built in (`data/`, see `data/README.md` for where it comes from), so the crate
//! needs no files or network at run time.

mod areas;
mod boundaries;
mod prefecture;

pub use areas::{SeismicArea, TsunamiArea};
pub use boundaries::{Boundaries, BoundaryError, PrefectureShape};
pub use prefecture::Prefecture;
