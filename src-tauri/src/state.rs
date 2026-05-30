//! Shared application state: user settings plus the live connection info that
//! the engine mirrors so commands can issue one-off requests.

use crate::lcu::models::Settings;
use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    /// (port, token) of the current LCU connection, written by the engine so
    /// commands (e.g. manual dodge) can act without rescanning processes.
    pub conn_info: Mutex<Option<(u16, String)>>,
}
