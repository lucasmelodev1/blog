use crate::utils::database::Crud;
use crate::AppState;
use crate::{session::Session, user::Role};
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
pub(crate) struct Auth {
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
pub(crate) struct SignInAuth {
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
        let session_id = session_id.expect("Session not found and not caught at compile time");
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

        let user_query = state
            .users_collection
            .find_one(bson::doc! { "auth_id": auth.id }, None)
            .await;
        let Ok(user) = user_query else { return (StatusCode::INTERNAL_SERVER_ERROR, AppendHeaders([(SET_COOKIE, String::from(""))])) };

        let password_is_correct = verify(&json.password, &auth.password_hash).unwrap();
        let session_id = nanoid!();
        let now = Utc::now();
        if password_is_correct {
            let user_session: Option<Session> = state
                .sessions_collection
                .find_one(bson::doc! { "auth_id": auth.id }, None)
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
                        auth_id: auth.id.expect("User has no id"),
                        user_id: match user {
                            Some(user) => user.id,
                            None => None,
                        },
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
        TypedHeader(_): TypedHeader<Cookie>,
        State(state): State<AppState>,
        Json(json): Json<SignInAuth>,
    ) -> (StatusCode, Json<Option<Auth>>) {
        let now = Utc::now();
        let password_hash =
            bcrypt::hash(json.password, DEFAULT_COST).expect("Failed hashing password");
        let mut auth = Auth {
            id: None,
            email: json.email,
            password_hash,
            created_at: Some(now),
            updated_at: Some(now),
        };

        let query = state.auths_collection.insert_one(&auth, None).await;

        match query {
            Ok(document) => {
                let id = document.inserted_id;
                auth.id = id.as_object_id();
                (StatusCode::CREATED, Json(Some(auth)))
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(Some(auth))),
        }
    }

    async fn read_all(
        TypedHeader(cookie): TypedHeader<Cookie>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<Vec<Auth>>>) {
        let user = Session::user(&cookie, &state).await;
        let Ok(user) = user else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(user) = user else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        if user.role != Role::Developer {
            return (StatusCode::UNAUTHORIZED, Json(None));
        }

        let auths_cursor_query = state.auths_collection.find(None, None).await;
        let Ok(mut auths_cursor) = auths_cursor_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };

        let mut auths: Vec<Auth> = vec![];

        while let Some(auth_result) = auths_cursor.next().await {
            if let Ok(auth) = auth_result {
                auths.push(auth);
            }
        }

        (StatusCode::FOUND, Json(Some(auths)))
    }

    async fn read(
        TypedHeader(cookie): TypedHeader<Cookie>,
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<Auth>>) {
        let user = Session::user(&cookie, &state).await;
        let Ok(user) = user else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(user) = user else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        if user.role != Role::Developer {
            return (StatusCode::UNAUTHORIZED, Json(None));
        }

        let auth_query = state
            .auths_collection
            .find_one(bson::doc! { "_id": id }, None)
            .await;
        let Ok(auth) = auth_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        (StatusCode::FOUND, Json(Some(auth)))
    }

    async fn update(
        TypedHeader(cookie): TypedHeader<Cookie>,
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
        Json(json): Json<Auth>,
    ) -> (StatusCode, Json<Option<Auth>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        let user = Session::user(&cookie, &state).await;
        let Ok(user) = user else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(user) = user else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        if auth.id != Some(id) || user.role != Role::Developer {
            return (StatusCode::UNAUTHORIZED, Json(None));
        }

        let hashed = hash(json.password_hash, DEFAULT_COST).expect("Failed to hash password");
        let auth_query = state
            .auths_collection
            .find_one_and_update(
                bson::doc! { "_id": id },
                bson::doc! { "$set": { "email": json.email, "password": hashed } },
                None,
            )
            .await;
        let Ok(auth) = auth_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        (StatusCode::OK, Json(Some(auth)))
    }

    async fn delete(
        TypedHeader(cookie): TypedHeader<Cookie>,
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<Auth>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        let user = Session::user(&cookie, &state).await;
        let Ok(user) = user else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(user) = user else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        if auth.id != Some(id) || user.role != Role::Developer {
            return (StatusCode::UNAUTHORIZED, Json(None));
        }

        let auth_query = state
            .auths_collection
            .find_one_and_delete(bson::doc! { "_id": id }, None)
            .await;
        let Ok(auth) = auth_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        (StatusCode::OK, Json(Some(auth)))
    }
}
