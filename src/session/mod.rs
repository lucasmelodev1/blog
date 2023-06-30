use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Session {
    pub created_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
}

impl Session {
    pub(crate) fn new(valid_until: Option<DateTime<Utc>>) -> Self {
        Self {
            created_at: Utc::now(),
            valid_until: match valid_until {
                Some(valid_until) => valid_until,
                None => Utc::now() + chrono::Duration::days(7),
            },
        }
    }
}

