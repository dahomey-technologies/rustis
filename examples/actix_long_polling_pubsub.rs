use actix_web::{App, HttpResponse, HttpServer, Responder, get, http::StatusCode, post, web};
use futures_util::StreamExt;
use rustis::{
    client::Client,
    commands::{ListCommands, PubSubCommands},
};
use std::{fmt, net::SocketAddr, time::Duration};

const POLL_TIMEOUT: Duration = Duration::from_secs(10);

pub struct RedisClients {
    /// Redis client for regular operations
    pub regular: Client,
    /// Redis client for subscriptions
    pub sub: Client,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // build rustis client in multiplexer mode (a unique rustis instance for all actix workers)
    // build a separated rustis client for subscriptions only in multiplexer mode (a unique rustis instance for all actix workers)
    let redis_uri = "redis://127.0.0.1:6379";
    let redis_clients = web::Data::new(RedisClients {
        regular: Client::connect(redis_uri).await.unwrap(),
        sub: Client::connect(redis_uri).await.unwrap(),
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {addr}");
    HttpServer::new(move || {
        App::new()
            .app_data(redis_clients.clone())
            .service(poll_messages)
            .service(publish)
    })
    .bind(addr)?
    .run()
    .await
}

#[get("/{channel}")]
async fn poll_messages(
    redis: web::Data<RedisClients>,
    channel: web::Path<String>,
) -> Result<impl Responder, ServiceError> {
    let channel = channel.into_inner();

    let messages = get_messages_from_queue(&redis.regular, &channel).await?;
    if POLL_TIMEOUT.is_zero() || !messages.is_empty() {
        return Ok(web::Json(messages));
    }

    let mut sub_stream = redis.sub.subscribe(&channel).await?;
    let msg = tokio::time::timeout(POLL_TIMEOUT, sub_stream.next()).await;

    let messages: Vec<String> = match msg {
        Ok(Some(Ok(_msg))) => get_messages_from_queue(&redis.regular, &channel).await?,
        Ok(Some(Err(e))) => {
            return Err(ServiceError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error received from PubSubStream: {e}"),
            ));
        }
        // stream closed
        Ok(None) => Vec::new(),
        // timeout
        Err(_e) => Vec::new(),
    };

    Ok(web::Json(messages))
}

async fn get_messages_from_queue(
    redis: &Client,
    channel: &str,
) -> Result<Vec<String>, ServiceError> {
    Ok(redis.lpop(channel, i32::MAX as usize).await?)
}

#[post("/{channel}")]
async fn publish(
    redis: web::Data<RedisClients>,
    channel: web::Path<String>,
    message: Option<String>,
) -> Result<impl Responder, ServiceError> {
    let Some(message) = message else {
        return Err(ServiceError::new(
            StatusCode::BAD_REQUEST,
            "Message not provided",
        ));
    };

    let channel = channel.into_inner();

    // data is not sent via pub/sub; the pub/sub API is used only to notify subscriber to check for new notifications
    // the actual data is pushed into a list used as a queue
    redis.regular.lpush(&channel, &message).await?;
    redis.regular.publish(&channel, "new").await?;
    Ok(HttpResponse::Ok())
}

#[derive(Debug)]
struct ServiceError(StatusCode, String);

impl ServiceError {
    fn new(status_code: StatusCode, description: impl ToString) -> Self {
        Self(status_code, description.to_string())
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.1)
    }
}

impl actix_web::error::ResponseError for ServiceError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        self.0
    }
}

impl From<rustis::Error> for ServiceError {
    fn from(e: rustis::Error) -> Self {
        eprintln!("rustis error: {e}");
        ServiceError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
    }
}
