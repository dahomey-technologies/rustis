use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use rustis::{
    client::Client,
    commands::{GenericCommands, StringCommands},
};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // build rustis client in multiplexer mode (a unique rustis instance for all axum workers)
    let redis = Arc::new(Client::connect("redis://127.0.0.1:6379").await.unwrap());

    // build our application with a route
    let app = Router::new()
        .route("/:key", get(read).post(update).delete(del))
        .with_state(redis);

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn read(
    State(redis): State<Arc<Client>>,
    Path(key): Path<String>,
) -> Result<String, ServiceError> {
    let value: Option<String> = redis.get(&key).await?;
    value.ok_or_else(|| {
        ServiceError::new(
            StatusCode::NOT_FOUND,
            format!("Key `{key}` does not exist."),
        )
    })
}

async fn update(
    State(redis): State<Arc<Client>>,
    Path(key): Path<String>,
    value: String,
) -> Result<(), ServiceError> {
    if value.is_empty() {
        return Err(ServiceError::new(
            StatusCode::BAD_REQUEST,
            "Value not provided",
        ));
    }
    redis.set(key, value).await?;
    Ok(())
}

async fn del(
    State(redis): State<Arc<Client>>,
    Path(key): Path<String>,
) -> Result<(), ServiceError> {
    let deleted = redis.del(&key).await?;
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
