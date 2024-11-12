use serde::{Deserialize, Serialize};

use crate::primitives::UserId;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CoreUserEvent {
    UserCreated { id: UserId },
    UserRemoved { id: UserId },
}