use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures_util::StreamExt;
use rustis::{
    client::Client,
    commands::{ListCommands, PubSubCommands},
};
use std::{net::SocketAddr, sync::Arc, time::Duration};

const POLL_TIMEOUT: Duration = Duration::from_secs(10);

pub struct RedisClients {
    /// Redis client for regular operations
    pub regular: Client,
    /// Redis client for subscriptions
    pub sub: Client,
}

#[tokio::main]
async fn main() {
    // build rustis client in multiplexer mode (a unique rustis instance for all actix workers)
    // build a separated rustis client for subscriptions only in multiplexer mode (a unique rustis instance for all actix workers)
    let redis_uri = "redis://127.0.0.1:6379";
    let redis_clients = Arc::new(RedisClients {
        regular: Client::connect(redis_uri).await.unwrap(),
        sub: Client::connect(redis_uri).await.unwrap(),
    });

    // build our application with a route
    let app = Router::new()
        .route("/:key", get(poll_messages).post(publish))
        .with_state(redis_clients);

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn poll_messages(
    State(redis): State<Arc<RedisClients>>,
    Path(channel): Path<String>,
) -> Result<Json<Vec<String>>, ServiceError> {
    let messages = get_messages_from_queue(&redis.regular, &channel).await?;
    if POLL_TIMEOUT.is_zero() || !messages.is_empty() {
        return Ok(Json(messages));
    }

    let mut sub_stream = redis.sub.subscribe(&channel).await?;
    let msg = tokio::time::timeout(POLL_TIMEOUT, sub_stream.next()).await;

    let messages: Vec<String> = match msg {
        Ok(Some(Ok(_msg))) => get_messages_from_queue(&redis.regular, &channel).await?,
        Ok(Some(Err(e))) => {
            return Err(ServiceError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error received from PubSubStream: {e}"),
            ))
        }
        // stream closed
        Ok(None) => Vec::new(),
        // timeout
        Err(_e) => Vec::new(),
    };

    Ok(Json(messages))
}

async fn get_messages_from_queue(
    redis: &Client,
    channel: &str,
) -> Result<Vec<String>, ServiceError> {
    Ok(redis.lpop(channel, i32::MAX as usize).await?)
}

async fn publish(
    State(redis): State<Arc<RedisClients>>,
    Path(channel): Path<String>,
    message: Option<String>,
) -> Result<(), ServiceError> {
    let Some(message) = message else {
        return Err(ServiceError::new(
            StatusCode::BAD_REQUEST,
            "Message not provided",
        ))
    };

    // data is not sent via pub/sub; the pub/sub API is used only to notify subscriber to check for new notifications
    // the actual data is pushed into a list used as a queue
    redis.regular.lpush(&channel, &message).await?;
    redis.regular.publish(&channel, "new").await?;
    Ok(())
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
