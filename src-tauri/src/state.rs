//! Shared application state: user settings plus one-shot action flags that the
//! engine loop reads each tick.

use crate::lcu::models::Settings;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    /// Set by the "Dodge now" button; consumed by the engine on the next tick.
    pub dodge_requested: AtomicBool,
}
