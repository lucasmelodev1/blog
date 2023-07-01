use crate::utils::database;
use crate::AppState;

use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use database::Crud;
use mongodb::bson;
use serde::{Deserialize, Serialize};

// needed to call .next() in mongodb Cursor type
use futures::StreamExt;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Post {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub author_id: String,
    pub title: String,
    pub content: String,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl Crud<Post, Post> for Post {
    async fn create(
        State(state): State<AppState>,
        Json(payload): Json<Post>,
    ) -> (StatusCode, Json<Post>) {
        let now = Utc::now();
        let post = Post {
            id: None,
            author_id: payload.author_id,
            title: payload.title,
            content: payload.content,
            created_at: Some(now),
            updated_at: Some(now),
        };

        let insert_result = state.posts_collection.insert_one(&post, None).await;

        match insert_result {
            Ok(_) => (StatusCode::CREATED, Json(post)),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(post)),
        }
    }

    async fn read_all(State(state): State<AppState>) -> (StatusCode, Json<Vec<Post>>) {
        let mut posts_cursor = state
            .posts_collection
            .find(None, None)
            .await
            .expect("Failed to execute find.");
        let mut posts: Vec<Post> = vec![];

        while let Some(post) = posts_cursor.next().await {
            posts.push(post.expect("Failed to get a post"));
        }

        (StatusCode::FOUND, Json(posts))
    }

    async fn read(
        Path(id): Path<bson::oid::ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Post>) {
        let post = state
            .posts_collection
            .find_one(bson::doc! { "_id": id }, None)
            .await
            .expect("Failed to execute find_one.")
            .expect("Failed to get a post");

        (StatusCode::FOUND, Json(post))
    }

    async fn update(
        Path(id): Path<bson::oid::ObjectId>,
        State(state): State<AppState>,
        Json(payload): Json<Post>,
    ) -> (StatusCode, Json<Post>) {
        let post = state.posts_collection
            .find_one_and_update(
                bson::doc! { "_id": id },
                bson::doc! { "$set": bson::to_document(&payload).expect("Body could not be serialized") },
                None
            )
            .await
            .expect("Failed to execute find_one_and_update.")
            .expect("Failed to get a post");

        (StatusCode::OK, Json(post))
    }

    async fn delete(
        Path(id): Path<bson::oid::ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Post>) {
        let post = state
            .posts_collection
            .find_one_and_delete(bson::doc! { "_id": id }, None)
            .await
            .expect("Failed to execute find_one_and_delete.")
            .expect("Failed to get a post");

        (StatusCode::OK, Json(post))
    }
}
