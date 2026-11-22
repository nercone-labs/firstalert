//! The core of FirstAlert.
//!
//! The core receives the information of the Japan Meteorological Agency as two relays send it
//! (Wolfx and P2P Quake), joins what both say of one earthquake, estimates the shaking where the
//! user is, decides how strongly to alert, and words the alert. It is the same on every platform.
//!
//! The core performs no input or output of the network: it is a state machine (`Core`) that
//! takes `Input`s (a message received, a connection opened or closed, a response, the passing of
//! time) and returns `Effect`s for the platform to carry out (open a connection, send a text,
//! fetch a URL, show a notification, wake it up later). Every platform has a mature WebSocket and
//! HTTP client with the system's TLS, and the core stays deterministic and testable: the time is
//! always an input. Files are the exception: the core keeps its settings and its history in the
//! directory the platform gives it.
//!
//! `protocol` defines the inputs, effects and the snapshot of the state, which the C interface
//! (`firstalert-ffi`) carries as JSON.

pub mod alert;
pub mod connection;
pub mod engine;
pub mod event;
pub mod impact;
pub mod protocol;
pub mod settings;
pub mod store;
pub mod tsunami;
pub mod words;

pub use engine::Core;
pub use protocol::{Effect, Input, Response, Snapshot};
pub use settings::Settings;
