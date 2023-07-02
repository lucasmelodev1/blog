use crate::session::Session;
use crate::utils::database::Crud;
use crate::AppState;
use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::headers::Cookie;
use axum::http::StatusCode;
use axum::{Json, TypedHeader};
use chrono::{DateTime, Utc};
use mongodb::bson;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

// needed to call .next() in mongodb Cursor type
use futures::StreamExt;

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub(crate) enum Role {
    User,
    Developer,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub(crate) struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub auth_id: ObjectId,
    pub display_name: String,
    pub role: Role,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl Crud<User, User> for User {
    async fn create(
        TypedHeader(cookie): TypedHeader<Cookie>,
        State(state): State<AppState>,
        Json(json): Json<User>,
    ) -> (StatusCode, Json<Option<User>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        let auth_id = auth.id.expect("User has no id");

        let user_query = state
            .users_collection
            .find_one(bson::doc! { "auth_id": auth_id }, None)
            .await;
        let Ok(user) = user_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let None = user else { return (StatusCode::FORBIDDEN, Json(None)) };

        let now = Utc::now();
        let mut user = User {
            id: None,
            auth_id,
            display_name: json.display_name,
            role: Role::User,
            created_at: Some(now),
            updated_at: Some(now),
        };

        let query = state.users_collection.insert_one(&user, None).await;

        match query {
            Ok(document) => {
                let id = document.inserted_id;
                user.id = id.as_object_id();
                (StatusCode::CREATED, Json(Some(user)))
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(Some(user))),
        }
    }

    async fn read_all(
        TypedHeader(cookie): TypedHeader<Cookie>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<Vec<User>>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        let user_query = state
            .users_collection
            .find_one(
                bson::doc! { "auth_id": auth.id.expect("User has no id") },
                None,
            )
            .await;
        let Ok(user) = user_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(user) = user else { return (StatusCode::FORBIDDEN, Json(None)) };

        if user.role != Role::Developer {
            return (StatusCode::UNAUTHORIZED, Json(None));
        };

        let users_cursor_query = state.users_collection.find(None, None).await;
        let Ok(mut users_cursor) = users_cursor_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };

        let mut users: Vec<User> = vec![];

        while let Some(user_result) = users_cursor.next().await {
            if let Ok(user) = user_result {
                users.push(user);
            }
        }

        (StatusCode::FOUND, Json(Some(users)))
    }

    async fn read(
        TypedHeader(cookie): TypedHeader<Cookie>,
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<User>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        if auth.id.expect("User has no id") != id {
            return (StatusCode::UNAUTHORIZED, Json(None));
        };

        let user_query = state
            .users_collection
            .find_one(bson::doc! { "_id": id }, None)
            .await;
        let Ok(user) = user_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(user) = user else { return (StatusCode::FORBIDDEN, Json(None)) };

        (StatusCode::FOUND, Json(Some(user)))
    }

    async fn update(
        TypedHeader(cookie): TypedHeader<Cookie>,
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
        Json(json): Json<User>,
    ) -> (StatusCode, Json<Option<User>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        if auth.id.expect("User has no id") != id {
            return (StatusCode::UNAUTHORIZED, Json(None));
        };

        let user_query = state.users_collection
            .find_one_and_update(
                bson::doc! { "_id": id },
                bson::doc! { "$set": { "displayName": json.display_name, "updated_at": Utc::now().to_rfc3339() } },
                None
            )
            .await;
        let Ok(user) = user_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(user) = user else { return (StatusCode::FORBIDDEN, Json(None)) };

        (StatusCode::OK, Json(Some(user)))
    }

    async fn delete(
        TypedHeader(cookie): TypedHeader<Cookie>,
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<User>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        if auth.id.expect("User has no id") != id {
            return (StatusCode::UNAUTHORIZED, Json(None));
        };

        let user_query = state
            .users_collection
            .find_one_and_delete(bson::doc! { "_id": id }, None)
            .await;
        let Ok(user) = user_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(user) = user else { return (StatusCode::FORBIDDEN, Json(None)) };

        (StatusCode::OK, Json(Some(user)))
    }
}
