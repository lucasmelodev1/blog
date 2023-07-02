use axum::headers::Cookie;
use chrono::{DateTime, Utc};
use mongodb::bson::{self, oid::ObjectId};
use serde::{Deserialize, Serialize};

use crate::{auth::Auth, user::User, AppState};

#[derive(Serialize, Deserialize)]
pub(crate) struct Session {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub auth_id: ObjectId,
    pub user_id: Option<ObjectId>,
    pub session_id: String,
    pub valid_until: DateTime<Utc>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Session {
    pub(crate) async fn user(cookie: &Cookie, state: &AppState) -> Result<Option<User>, ()> {
        let Some(session_id) = cookie.get("session_id") else { return Ok(None) };
        let session_query = state
            .sessions_collection
            .find_one(bson::doc! { "session_id": session_id }, None)
            .await;
        let Ok(session) = session_query else { return Err(()) };
        let Some(session) = session else { return Ok(None) };

        let user_query = state
            .users_collection
            .find_one(bson::doc! { "_id": session.user_id }, None)
            .await;
        let Ok(user) = user_query else { return Err(()) };
        let Some(user) = user else { return Ok(None) };

        Ok(Some(user))
    }

    pub(crate) async fn auth(cookie: &Cookie, state: &AppState) -> Result<Option<Auth>, ()> {
        let Some(session_id) = cookie.get("session_id") else { return Ok(None) };
        let session_query = state
            .sessions_collection
            .find_one(bson::doc! { "session_id": session_id }, None)
            .await;
        let Ok(session) = session_query else { return Err(()) };
        let Some(session) = session else { return Ok(None) };

        let auth_query = state
            .auths_collection
            .find_one(bson::doc! { "_id": session.auth_id }, None)
            .await;
        let Ok(auth) = auth_query else { return Err(()) };
        let Some(auth) = auth else { return Ok(None) };

        Ok(Some(auth))
    }
}
