use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Update(u128),
    Quit,
    Up,
    Down,
    PageUp,
    PageDown,
    ClearScreen,
    Error(String),
    Help,
}
