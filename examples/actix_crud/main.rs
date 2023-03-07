use actix_web::{delete, get, http::StatusCode, post, web, App, HttpServer, HttpResponseBuilder, HttpResponse};
use rustis::{
    client::Client,
    commands::{GenericCommands, StringCommands},
};
use std::{
    cell::{RefCell, RefMut},
    fmt::{self, Display},
    net::SocketAddr,
};

#[derive(Clone)]
struct ThreadLocalRedis {
    client: RefCell<Client>
}

impl ThreadLocalRedis {
    pub fn new(client: Client) -> Self {
        Self {
            client: RefCell::new(client)
        }
    }

    pub fn get_client_mut(&self) -> RefMut<'_, Client> {
        self.client.borrow_mut()
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // build rustis client
    let client = Client::connect("redis://127.0.0.1:6379").await.unwrap();
    let thread_local_redis = ThreadLocalRedis::new(client);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(thread_local_redis.clone()))
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
    redis: web::Data<ThreadLocalRedis>,
    key: web::Path<String>,
) -> Result<String, ServiceError> {
    let value: Option<String> = redis.get_client_mut().get(key.clone()).await?;
    value.ok_or_else(|| {
        ServiceError::new(
            StatusCode::NOT_FOUND,
            format!("Key `{key}` does not exist."),
        )
    })
}

#[post("/{key}")]
async fn update(
    redis: web::Data<ThreadLocalRedis>,
    key: web::Path<String>,
    value: Option<String>,
) -> Result<HttpResponseBuilder, ServiceError> {
    if value.is_none() {
        return Err(ServiceError::new(
            StatusCode::BAD_REQUEST,
            "Value not provided",
        ));
    }
    redis.get_client_mut().set(key.into_inner(), value).await?;
    Ok(HttpResponse::Ok())
}

#[delete("/{key}")]
async fn delete(
    redis: web::Data<ThreadLocalRedis>,
    key: web::Path<String>,
) -> Result<HttpResponseBuilder, ServiceError> {
    let deleted = redis.get_client_mut().del(key.clone()).await?;
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
