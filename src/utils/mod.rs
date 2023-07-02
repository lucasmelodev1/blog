pub mod database {
    use crate::AppState;
    use async_trait::async_trait;
    use axum::extract::{Path, State};
    use axum::headers::Cookie;
    use axum::http::StatusCode;
    use axum::{Json, TypedHeader};
    use mongodb::bson;

    #[async_trait]
    pub(crate) trait Crud<T, U> {
        async fn create(
            cookie: TypedHeader<Cookie>,
            state: State<AppState>,
            json: Json<T>,
        ) -> (StatusCode, Json<Option<U>>);
        async fn read_all(
            cookie: TypedHeader<Cookie>,
            state: State<AppState>,
        ) -> (StatusCode, Json<Option<Vec<U>>>);
        async fn read(
            cookie: TypedHeader<Cookie>,
            path: Path<bson::oid::ObjectId>,
            state: State<AppState>,
        ) -> (StatusCode, Json<Option<U>>);
        async fn update(
            cookie: TypedHeader<Cookie>,
            path: Path<bson::oid::ObjectId>,
            state: State<AppState>,
            json: Json<U>,
        ) -> (StatusCode, Json<Option<U>>);
        async fn delete(
            cookie: TypedHeader<Cookie>,
            path: Path<bson::oid::ObjectId>,
            state: State<AppState>,
        ) -> (StatusCode, Json<Option<U>>);
    }
}
