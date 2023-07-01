use crate::utils::database::Crud;
use crate::AppState;
use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use mongodb::bson;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

// needed to call .next() in mongodb Cursor type
use futures::StreamExt;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub display_name: String,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl Crud<User, User> for User {
    async fn create(
        State(state): State<AppState>,
        Json(json): Json<User>,
    ) -> (StatusCode, Json<User>) {
        let now = Utc::now();
        let user = User {
            id: None,
            display_name: json.display_name,
            created_at: Some(now),
            updated_at: Some(now),
        };

        let insert_result = state.users_collection.insert_one(&user, None).await;

        match insert_result {
            Ok(_) => (StatusCode::CREATED, Json(user)),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(user)),
        }
    }

    async fn read_all(State(state): State<AppState>) -> (StatusCode, Json<Vec<User>>) {
        let mut users_cursor = state
            .users_collection
            .find(None, None)
            .await
            .expect("Failed to execute find.");
        let mut users: Vec<User> = vec![];

        while let Some(user) = users_cursor.next().await {
            users.push(user.expect("Failed to get a user"));
        }

        (StatusCode::FOUND, Json(users))
    }

    async fn read(
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<User>) {
        let user = state
            .users_collection
            .find_one(bson::doc! { "_id": id }, None)
            .await
            .expect("Failed to execute find_one.")
            .expect("Failed to get a user");

        (StatusCode::FOUND, Json(user))
    }

    async fn update(
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
        Json(json): Json<User>,
    ) -> (StatusCode, Json<User>) {
        let user = state.users_collection
            .find_one_and_update(
                bson::doc! { "_id": id },
                bson::doc! { "$set": { "displayName": json.display_name, "updated_at": Utc::now().to_rfc3339() } },
                None
            )
            .await
            .expect("Failed to execute find_one_and_update.")
            .expect("Failed to get a user");

        (StatusCode::OK, Json(user))
    }

    async fn delete(
        Path(id): Path<ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<User>) {
        let user = state
            .users_collection
            .find_one_and_delete(bson::doc! { "_id": id }, None)
            .await
            .expect("Failed to execute find_one_and_delete.")
            .expect("Failed to get a user");

        (StatusCode::OK, Json(user))
    }
}
