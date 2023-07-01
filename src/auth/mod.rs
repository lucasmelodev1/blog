use crate::session::Session;
use crate::utils::database::Crud;
use crate::AppState;
use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    headers::Cookie,
    http::{header::SET_COOKIE, StatusCode},
    response::{AppendHeaders, IntoResponse},
    Json, TypedHeader,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use mongodb::bson;
use mongodb::bson::oid::ObjectId;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Auth {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub password_hash: String,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct SignInAuth {
    pub email: String,
    pub password: String,
}

impl Auth {
    pub(crate) async fn sign_in_session(
        TypedHeader(typed_header): TypedHeader<Cookie>,
        State(state): State<AppState>,
    ) -> (StatusCode, impl IntoResponse) {
        let session_id = typed_header.get("session_id");
        if let None = session_id {
            return (
                StatusCode::UNAUTHORIZED,
                AppendHeaders([(SET_COOKIE, String::from(""))]),
            );
        }
        let session_user = state
            .sessions_collection
            .find_one(bson::doc! { "session_id": session_id }, None)
            .await
            .expect("Failed to execute find_one");
        match session_user {
            Some(session) => {
                if Utc::now() <= session.valid_until {
                    return (
                        StatusCode::UNAUTHORIZED,
                        AppendHeaders([(SET_COOKIE, String::from(""))]),
                    );
                }
            }
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    AppendHeaders([(SET_COOKIE, String::from(""))]),
                );
            }
        }

        let new_session_id = nanoid!();
        let _ = state
            .sessions_collection
            .update_one(
                bson::doc! { "session_id": session_id },
                bson::doc! { "$set": { "session_id": &new_session_id }},
                None,
            )
            .await
            .expect("Failed to execute update_one");

        (
            StatusCode::OK,
            AppendHeaders([(SET_COOKIE, format!("session_id={}", &new_session_id))]),
        )
    }

    pub(crate) async fn sign_in(
        State(state): State<AppState>,
        Json(json): Json<SignInAuth>,
    ) -> (StatusCode, impl IntoResponse) {
        let auth = state
            .auths_collection
            .find_one(bson::doc! { "email": &json.email }, None)
            .await
            .expect("Failed to execute find_one")
            .expect("Failed to get an auth");
        let password_is_correct = verify(&json.password, &auth.password_hash).unwrap();
        let session_id = nanoid!();
        let now = Utc::now();
        if password_is_correct {
            let user_session: Option<Session> = state
                .sessions_collection
                .find_one(bson::doc! { "user_id": auth.id }, None)
                .await
                .expect("Failed to execute find_one");
            match user_session {
                Some(session) => {
                    state
                        .sessions_collection
                        .delete_one(bson::doc! { "_id": session.id }, None)
                        .await
                        .expect("Failed to delete existent user session");
                }
                None => (),
            };
            state
                .sessions_collection
                .insert_one(
                    Session {
                        id: None,
                        user_id: auth.id.expect("User has not an id"),
                        session_id: session_id.to_string(),
                        valid_until: now + Duration::days(7),
                        created_at: Some(now),
                        updated_at: Some(now),
                    },
                    None,
                )
                .await
                .expect("Failed to execute insert_one");
        };
        (
            StatusCode::OK,
            AppendHeaders([(SET_COOKIE, format!("session_id={}", session_id))]),
        )
    }
}

#[async_trait]
impl Crud<SignInAuth, Auth> for Auth {
    async fn create(
        State(state): State<AppState>,
        Json(json): Json<SignInAuth>,
    ) -> (StatusCode, Json<Auth>) {
        let now = Utc::now();
        let password_hash =
            bcrypt::hash(json.password, DEFAULT_COST).expect("Failed hashing password");
        let auth = Auth {
            id: None,
            email: json.email,
            password_hash,
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
        let hashed = hash(json.password_hash, DEFAULT_COST).expect("Failed to hash password");
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
