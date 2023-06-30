pub mod database {
    use axum::extract::{Path, State};
    use axum::Json;
    use mongodb::bson;
    use crate::AppState;
    use async_trait::async_trait;
    use axum::http::StatusCode;


    #[async_trait]
    pub(crate) trait Crud<T> {
        async fn create(state: State<AppState>, json: Json<T>) -> (StatusCode, Json<T>);
        async fn read_all(state: State<AppState>) -> (StatusCode, Json<Vec<T>>);
        async fn read(path: Path<bson::oid::ObjectId>, state: State<AppState>) -> (StatusCode, Json<T>);
        async fn update(path: Path<bson::oid::ObjectId>, state: State<AppState>, json: Json<T>) -> (StatusCode, Json<T>);
        async fn delete(path: Path<bson::oid::ObjectId>, state: State<AppState>) -> (StatusCode, Json<T>);
    }
}