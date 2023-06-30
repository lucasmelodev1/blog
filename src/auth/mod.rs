use crate::session;
use crate::utils::database::Crud;
use crate::AppState;
use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_sessions::extractors::{ReadableSession, WritableSession};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use mongodb::bson;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Auth {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub password: String,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Auth {
    pub(crate) async fn sign_in_session(
        Path(cookie): Path<String>,
        session: ReadableSession,
        State(_): State<AppState>,
    ) -> (StatusCode, String) {
        let cookie = session.get::<session::Session>(cookie.as_ref());
        match cookie {
            Some(session) => {
                if Utc::now() < session.valid_until {
                    (StatusCode::OK, "Session is Valid".to_string())
                } else {
                    (StatusCode::UNAUTHORIZED, "Session is Invalid".to_string())
                }
            }
            None => (
                StatusCode::UNAUTHORIZED,
                "Session does not exist".to_string(),
            ),
        }
    }

    pub(crate) async fn sign_in(
        mut session: WritableSession,
        State(state): State<AppState>,
        Json(json): Json<Auth>,
    ) {
        let auth = state
            .auths_collection
            .find_one(bson::doc! { "email": &json.email }, None)
            .await
            .expect("Failed to execute find_one")
            .expect("Failed to get an auth");
        let hashed = hash(&json.password, DEFAULT_COST).expect("Failed to hash password");
        if auth.email == json.email && verify(&json.password, &hashed).unwrap() {
            session
                .insert(&auth.id.unwrap().to_hex(), session::Session::new(None))
                .expect("Failed to insert user_id into session");
        }
    }
}

#[async_trait]
impl Crud<Auth> for Auth {
    async fn create(
        State(state): State<AppState>,
        Json(json): Json<Auth>,
    ) -> (StatusCode, Json<Auth>) {
        let now = Utc::now();
        let auth = Auth {
            id: None,
            email: json.email,
            password: json.password,
            created_at: Some(now),
            updated_at: Some(now),
        };

        let insert_result = state.auths_collection.insert_one(&auth, None).await;

        match insert_result {
            Ok(_) => (StatusCode::CREATED, Json(auth)),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(auth)),
        }
    }

    async fn read_all(State(state): State<AppState>) -> (StatusCode, Json<Vec<Auth>>) {
        let mut auths_cursor = state
            .auths_collection
            .find(None, None)
            .await
            .expect("Failed to execute find.");
        let mut auths: Vec<Auth> = vec![];

        while let Some(auth) = auths_cursor.next().await {
            auths.push(auth.expect("Failed to get a user"));
        }

        (StatusCode::FOUND, Json(auths))
    }

    async fn read(
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Auth>) {
        let auth = state
            .auths_collection
            .find_one(bson::doc! { "_id": id }, None)
            .await
            .expect("Failed to execute find_one")
            .expect("Failed to get an auth");

        (StatusCode::FOUND, Json(auth))
    }

    async fn update(
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
        Json(json): Json<Auth>,
    ) -> (StatusCode, Json<Auth>) {
        let hashed = hash(json.password, DEFAULT_COST).expect("Failed to hash password");
        let auth = state
            .auths_collection
            .find_one_and_update(
                bson::doc! { "_id": id },
                bson::doc! { "$set": { "email": json.email, "password": hashed } },
                None,
            )
            .await
            .expect("Failed to execute find_one_and_update")
            .expect("Failed to get an auth");

        (StatusCode::OK, Json(auth))
    }

    async fn delete(
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Auth>) {
        let auth = state
            .auths_collection
            .find_one_and_delete(bson::doc! { "_id": id }, None)
            .await
            .expect("Failed to execute find_one_and_delete")
            .expect("Failed to get an auth");

        (StatusCode::OK, Json(auth))
    }
}
