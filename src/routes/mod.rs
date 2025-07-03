use axum::{Router, routing::get};

use crate::{
    handlers::sse::{post_handler, sse_handler},
    models::App,
};

pub fn create_routes() -> Router {
    let app = App::default();
    Router::new()
        .route("/sse", get(sse_handler).post(post_handler))
        .layer(axum::Extension(app))
}
