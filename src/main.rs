mod auth;
mod post;
mod session;
mod user;
mod utils;

use crate::auth::Auth;
use crate::post::Post;
use crate::user::User;
use crate::utils::database::Crud;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use axum_sessions::{async_session::MemoryStore, SessionLayer};
use mongodb::{
    bson::doc,
    options::{ClientOptions, ServerApi, ServerApiVersion},
    Client, Collection,
};

#[derive(Clone)]
struct AppState {
    // pub client: Client,
    // pub database: Database,
    pub posts_collection: Collection<Post>,
    pub users_collection: Collection<User>,
    pub auths_collection: Collection<Auth>,
}

#[tokio::main]
async fn main() -> mongodb::error::Result<()> {
    let client = get_database_client()
        .await
        .expect("Failed to connect to database.");
    let store = MemoryStore::new();
    let secret = "1234567891234567891234567891234567891234567891234567891234567891".as_bytes();
    let session_layer = SessionLayer::new(store, secret).with_secure(false);

    let app = Router::new()
        .nest("/api", make_api())
        .with_state(AppState {
            // client: client.clone(),
            // database: client.database("blog"),
            posts_collection: client.database("blog").collection::<Post>("posts"),
            users_collection: client.database("blog").collection::<User>("users"),
            auths_collection: client.database("blog").collection::<Auth>("auths"),
        })
        .layer(session_layer);

    axum::Server::bind(&"0.0.0.0:4000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

fn make_api() -> Router<AppState> {
    Router::new()
        .route("/posts", post(Post::create))
        .route("/posts", get(Post::read_all))
        .route("/posts/:id", get(Post::read))
        .route("/posts/:id", patch(Post::update))
        .route("/posts/:id", delete(Post::delete))
        .route("/users", post(User::create))
        .route("/users", get(User::read_all))
        .route("/users/:id", get(User::read))
        .route("/users/:id", patch(User::update))
        .route("/users/:id", delete(User::delete))
        .route("/auth", post(Auth::create))
        .route("/auth", get(Auth::read_all))
        .route("/auth/:id", get(Auth::read))
        .route("/auth/:id", patch(Auth::update))
        .route("/auth/:id", delete(Auth::delete))
        .route("/auth/sign-in", post(Auth::sign_in))
        .route("/auth/sign-in-session/:id", post(Auth::sign_in_session))
}

async fn get_database_client() -> mongodb::error::Result<Client> {
    let uri = std::env::var("BLOG_DB").expect("BLOG_DB environment variable not set");
    let mut client_options = ClientOptions::parse(uri).await?;

    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    // Get a handle to the cluster
    let client = Client::with_options(client_options)?;
    // Ping the server to see if you can connect to the cluster
    client
        .database("admin")
        .run_command(doc! {"ping": 1}, None)
        .await
        .expect("Failed to connect to cluster.");

    Ok(client)
}
