pub mod database {
    use crate::AppState;
    use async_trait::async_trait;
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use axum::Json;
    use mongodb::bson;

    #[async_trait]
    pub(crate) trait Crud<T, U> {
        async fn create(state: State<AppState>, json: Json<T>) -> (StatusCode, Json<U>);
        async fn read_all(state: State<AppState>) -> (StatusCode, Json<Vec<U>>);
        async fn read(
            path: Path<bson::oid::ObjectId>,
            state: State<AppState>,
        ) -> (StatusCode, Json<U>);
        async fn update(
            path: Path<bson::oid::ObjectId>,
            state: State<AppState>,
            json: Json<U>,
        ) -> (StatusCode, Json<U>);
        async fn delete(
            path: Path<bson::oid::ObjectId>,
            state: State<AppState>,
        ) -> (StatusCode, Json<U>);
    }
}

