use actix_web::{delete, get, http::StatusCode, post, web, App, HttpServer, HttpResponse, Responder};
use rustis::{
    client::Client,
    commands::{GenericCommands, StringCommands},
};
use std::{
    fmt::{self, Display},
    net::SocketAddr,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // build rustis client in multiplexer mode (a unique rustis instance for all actix workers)
    let redis = web::Data::new(Client::connect("redis://127.0.0.1:6379").await.unwrap());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    HttpServer::new(move || {
        App::new()
            .app_data(redis.clone())
            .service(read)
            .service(update)
            .service(delete)
    })
    .bind(&addr)?
    .run()
    .await
}

#[get("/{key}")]
async fn read(
    redis: web::Data<Client>,
    key: web::Path<String>,
) -> Result<String, ServiceError> {
    let value: Option<String> = redis.get(key.clone()).await?;
    value.ok_or_else(|| {
        ServiceError::new(
            StatusCode::NOT_FOUND,
            format!("Key `{key}` does not exist."),
        )
    })
}

#[post("/{key}")]
async fn update(
    redis: web::Data<Client>,
    key: web::Path<String>,
    value: Option<String>,
) -> Result<impl Responder, ServiceError> {
    if value.is_none() {
        return Err(ServiceError::new(
            StatusCode::BAD_REQUEST,
            "Value not provided",
        ));
    }
    redis.set(key.into_inner(), value).await?;
    Ok(HttpResponse::Ok())
}

#[delete("/{key}")]
async fn delete(
    redis: web::Data<Client>,
    key: web::Path<String>,
) -> Result<impl Responder, ServiceError> {
    let deleted = redis.del(key.clone()).await?;
    if deleted > 0 {
        Ok(HttpResponse::Ok())
    } else {
        Err(ServiceError::new(
            StatusCode::NOT_FOUND,
            format!("Key `{key}` does not exist."),
        ))
    }
}

#[derive(Debug)]
struct ServiceError(StatusCode, String);

impl ServiceError {
    fn new(status_code: StatusCode, description: impl ToString) -> Self {
        Self(status_code, description.to_string())
    }
}

impl Display for ServiceError {
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
