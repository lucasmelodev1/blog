use crate::session::Session;
use crate::utils::database;
use crate::AppState;

use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::headers::Cookie;
use axum::http::StatusCode;
use axum::{Json, TypedHeader};
use chrono::{DateTime, Utc};
use database::Crud;
use mongodb::bson;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

// needed to call .next() in mongodb Cursor type
use futures::StreamExt;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Post {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub author_id: ObjectId,
    pub title: String,
    pub content: String,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct CreatePost {
    pub title: String,
    pub content: String,
}

#[async_trait]
impl Crud<CreatePost, Post> for Post {
    async fn create(
        TypedHeader(cookie): TypedHeader<Cookie>,
        State(state): State<AppState>,
        Json(json): Json<CreatePost>,
    ) -> (StatusCode, Json<Option<Post>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        let now = Utc::now();
        let mut post = Post {
            id: None,
            author_id: auth.id.expect("User has no id"),
            title: json.title,
            content: json.content,
            created_at: Some(now),
            updated_at: Some(now),
        };

        let query = state.posts_collection.insert_one(&post, None).await;

        match query {
            Ok(document) => {
                let id = document.inserted_id;
                post.id = id.as_object_id();
                (StatusCode::CREATED, Json(Some(post)))
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
        }
    }

    async fn read_all(
        _: TypedHeader<Cookie>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<Vec<Post>>>) {
        let posts_cursor_query = state.posts_collection.find(None, None).await;
        let Ok(mut posts_cursor) = posts_cursor_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };

        let mut posts: Vec<Post> = vec![];

        while let Some(post_result) = posts_cursor.next().await {
            if let Ok(post) = post_result {
                posts.push(post);
            }
        }

        (StatusCode::FOUND, Json(Some(posts)))
    }

    async fn read(
        TypedHeader(_): TypedHeader<Cookie>,
        Path(id): Path<bson::oid::ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<Post>>) {
        let post_query = state
            .posts_collection
            .find_one(bson::doc! { "_id": id }, None)
            .await;
        let Ok(post) = post_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(post) = post else { return (StatusCode::NOT_FOUND, Json(None)) };

        (StatusCode::FOUND, Json(Some(post)))
    }

    async fn update(
        TypedHeader(cookie): TypedHeader<Cookie>,
        Path(id): Path<bson::oid::ObjectId>,
        State(state): State<AppState>,
        Json(json): Json<Post>,
    ) -> (StatusCode, Json<Option<Post>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        let post_query = state
            .posts_collection
            .find_one_and_update(
                bson::doc! { "_id": id, "author_id": auth.id },
                bson::doc! { "$set": { "title": json.title, "content": json.content } },
                None,
            )
            .await;
        let Ok(post) = post_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(post) = post else { return (StatusCode::NOT_FOUND, Json(None)) };

        (StatusCode::OK, Json(Some(post)))
    }

    async fn delete(
        TypedHeader(cookie): TypedHeader<Cookie>,
        Path(id): Path<bson::oid::ObjectId>,
        State(state): State<AppState>,
    ) -> (StatusCode, Json<Option<Post>>) {
        let auth = Session::auth(&cookie, &state).await;
        let Ok(auth) = auth else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(auth) = auth else { return (StatusCode::UNAUTHORIZED, Json(None)) };

        let post_query = state
            .posts_collection
            .find_one_and_delete(bson::doc! { "_id": id, "author_id": auth.id }, None)
            .await;
        let Ok(post) = post_query else { return (StatusCode::INTERNAL_SERVER_ERROR, Json(None)) };
        let Some(post) = post else { return (StatusCode::NOT_FOUND, Json(None)) };

        (StatusCode::OK, Json(Some(post)))
    }
}
