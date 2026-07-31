use crate::{Future, Result};

/// The credentials a connection authenticates with, as handed to the
/// [`HELLO`](https://redis.io/commands/hello/) handshake.
#[derive(Clone)]
pub struct Credentials {
    /// An optional ACL username. `None` authenticates as `default`.
    pub username: Option<String>,
    /// The password, or the token standing in for one.
    pub password: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            // never leak the password in clear text
            .field("password", &"***")
            .finish()
    }
}

/// A source of credentials consulted at **every** handshake: the initial
/// connection and each reconnection.
///
/// [`Config::password`](crate::client::Config::password) is fixed once and for
/// all, which is enough only while the password itself never changes. Managed
/// Redis offerings authenticate with short-lived tokens instead — AWS
/// ElastiCache IAM (15 minutes), GCP Memorystore IAM, Azure Entra ID, Vault
/// dynamic secrets — and a client replaying the token it was built with fails
/// authentication for good once that token expires. A provider is asked again
/// on each reconnection, so the client picks up the current token.
///
/// The trait is implemented for any `Fn() -> Future<Output = Result<Credentials>>`,
/// so a closure is usually all that is needed:
///
/// ```
/// use rustis::client::{Config, Credentials, IntoConfig};
/// use std::sync::Arc;
///
/// # fn main() -> rustis::Result<()> {
/// let mut config = "redis://127.0.0.1".into_config()?;
/// config.credentials_provider = Some(Arc::new(|| async {
///     // regenerate the token here (IAM, Vault, ...)
///     Ok(Credentials {
///         username: Some("iam-user".to_owned()),
///         password: generate_auth_token().await,
///     })
/// }));
/// # Ok(())
/// # }
/// # async fn generate_auth_token() -> String { String::from("token") }
/// ```
///
/// An error returned by the provider fails the handshake like any other
/// connection error: the [reconnection policy](crate::client::ReconnectionConfig)
/// retries it with its own backoff.
pub trait CredentialsProvider: Send + Sync + 'static {
    /// Yields the credentials to authenticate the connection being established.
    fn credentials(&self) -> Future<'_, Credentials>;
}

impl<F, Fut> CredentialsProvider for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Credentials>> + Send + 'static,
{
    fn credentials(&self) -> Future<'_, Credentials> {
        Box::pin(self())
    }
}
