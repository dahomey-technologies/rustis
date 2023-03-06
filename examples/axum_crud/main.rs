use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Router,
};
use rustis::{
    client::Client,
    commands::{GenericCommands, StringCommands},
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // build rustis client
    let redis = Client::connect("redis://127.0.0.1:6379").await.unwrap();

    // build our application with a route
    let app = Router::new()
        .route("/:key", get(read))
        .route("/:key", post(update))
        .route("/:key", delete(del))
        .with_state(redis);

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn read(
    State(mut redis): State<Client>,
    Path(key): Path<String>,
) -> Result<String, ServiceError> {
    let value: Option<String> = redis.get(key.clone()).await?;
    value.ok_or_else(|| {
        ServiceError::new(
            StatusCode::NOT_FOUND,
            format!("Key `{key}` does not exist."),
        )
    })
}

async fn update(
    State(mut redis): State<Client>,
    Path(key): Path<String>,
    value: Option<String>,
) -> Result<(), ServiceError> {
    if value.is_none() {
        return Err(ServiceError::new(
            StatusCode::BAD_REQUEST,
            "Value not provided",
        ));
    }
    redis.set(key, value).await?;
    Ok(())
}

async fn del(State(mut redis): State<Client>, Path(key): Path<String>) -> Result<(), ServiceError> {
    let deleted = redis.del(key.clone()).await?;
    if deleted > 0 {
        Ok(())
    } else {
        Err(ServiceError::new(
            StatusCode::NOT_FOUND,
            format!("Key `{key}` does not exist."),
        ))
    }
}

struct ServiceError(StatusCode, String);

impl ServiceError {
    fn new(status_code: StatusCode, description: impl ToString) -> Self {
        Self(status_code, description.to_string())
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

impl From<rustis::Error> for ServiceError {
    fn from(e: rustis::Error) -> Self {
        eprintln!("rustis error: {e}");
        ServiceError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
    }
}
