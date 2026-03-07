use super::{Config, IntoConfig};
use crate::{Future, Result};
use std::{fmt, future::Future as StdFuture, sync::Arc};

/// Fresh authentication material for a newly established Redis TCP session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub username: Option<String>,
    pub password: String,
}

impl Credentials {
    #[must_use]
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: Some(username.into()),
            password: password.into(),
        }
    }

    #[must_use]
    pub fn for_default_user(password: impl Into<String>) -> Self {
        Self {
            username: None,
            password: password.into(),
        }
    }
}

/// Why a new TCP session is being authenticated.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialsReason {
    InitialConnect,
    Reconnect,
    TopologyRefresh,
}

/// Which kind of server socket is being authenticated.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialsTarget {
    DataNode,
    SentinelNode,
}

/// The higher-level topology that triggered this authentication request.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerKind {
    Standalone,
    Sentinel,
    Cluster,
}

/// Connection metadata passed to a [`CredentialsProvider`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialsContext {
    pub host: String,
    pub port: u16,
    pub reason: CredentialsReason,
    pub target: CredentialsTarget,
    pub server_kind: ServerKind,
    pub tls_enabled: bool,
}

/// Async credentials source used to authenticate every new TCP session.
pub trait CredentialsProvider: Send + Sync + 'static {
    fn resolve(&self, context: CredentialsContext) -> Future<'_, Credentials>;
}

/// Cloneable handle to a shared [`CredentialsProvider`].
#[derive(Clone)]
pub struct SharedCredentialsProvider(Arc<dyn CredentialsProvider>);

impl SharedCredentialsProvider {
    #[must_use]
    pub fn new<P: CredentialsProvider>(provider: P) -> Self {
        Self(Arc::new(provider))
    }

    pub(crate) fn resolve(&self, context: CredentialsContext) -> Future<'_, Credentials> {
        self.0.resolve(context)
    }
}

impl fmt::Debug for SharedCredentialsProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SharedCredentialsProvider(..)")
    }
}

impl<P: CredentialsProvider> From<P> for SharedCredentialsProvider {
    fn from(provider: P) -> Self {
        Self::new(provider)
    }
}

impl<F, Fut> CredentialsProvider for F
where
    F: Fn(CredentialsContext) -> Fut + Send + Sync + 'static,
    Fut: StdFuture<Output = Result<Credentials>> + Send + 'static,
{
    fn resolve(&self, context: CredentialsContext) -> Future<'_, Credentials> {
        Box::pin((self)(context))
    }
}

/// Wrap an async closure into a [`SharedCredentialsProvider`].
#[must_use]
pub fn credentials_provider_fn<F, Fut>(f: F) -> SharedCredentialsProvider
where
    F: Fn(CredentialsContext) -> Fut + Send + Sync + 'static,
    Fut: StdFuture<Output = Result<Credentials>> + Send + 'static,
{
    SharedCredentialsProvider::new(f)
}

/// `Internal Use`
///
/// Connection inputs after resolving an [`IntoConfig`] implementation and layering
/// any dynamic credentials providers on top of it.
///
/// This type is public because it appears in the hidden
/// [`IntoConfig::into_connection_setup`](crate::client::IntoConfig::into_connection_setup)
/// plumbing, but it is not intended to be constructed, matched on, or stored directly
/// by end users.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ConnectionSetup {
    pub(crate) config: Config,
    pub(crate) credentials_provider: Option<SharedCredentialsProvider>,
    pub(crate) sentinel_credentials_provider: Option<SharedCredentialsProvider>,
}

impl ConnectionSetup {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            credentials_provider: None,
            sentinel_credentials_provider: None,
        }
    }

    #[must_use]
    pub(crate) fn with_credentials_provider(mut self, provider: SharedCredentialsProvider) -> Self {
        self.credentials_provider = Some(provider);
        self
    }

    #[must_use]
    pub(crate) fn with_sentinel_credentials_provider(
        mut self,
        provider: SharedCredentialsProvider,
    ) -> Self {
        self.sentinel_credentials_provider = Some(provider);
        self
    }
}

impl IntoConfig for ConnectionSetup {
    fn into_config(self) -> Result<Config> {
        Ok(self.config)
    }

    fn into_connection_setup(self) -> Result<ConnectionSetup> {
        Ok(self)
    }
}

/// Wrapper returned by [`WithCredentialsProvider`] to attach dynamic auth providers
/// to any [`IntoConfig`] input.
///
/// End users should normally obtain this type via the extension-trait methods rather
/// than naming it directly.
#[derive(Debug, Clone)]
pub struct ConfigWithCredentialsProvider<C> {
    inner: C,
    credentials_provider: Option<SharedCredentialsProvider>,
    sentinel_credentials_provider: Option<SharedCredentialsProvider>,
}

impl<C> ConfigWithCredentialsProvider<C> {
    fn new(inner: C) -> Self {
        Self {
            inner,
            credentials_provider: None,
            sentinel_credentials_provider: None,
        }
    }

    /// Use this provider for every new Redis data-node TCP session.
    #[must_use]
    pub fn with_credentials_provider(
        mut self,
        provider: impl Into<SharedCredentialsProvider>,
    ) -> Self {
        self.credentials_provider = Some(provider.into());
        self
    }

    /// Use this provider for every new Sentinel control-plane TCP session.
    #[must_use]
    pub fn with_sentinel_credentials_provider(
        mut self,
        provider: impl Into<SharedCredentialsProvider>,
    ) -> Self {
        self.sentinel_credentials_provider = Some(provider.into());
        self
    }
}

impl<C: IntoConfig> IntoConfig for ConfigWithCredentialsProvider<C> {
    fn into_config(self) -> Result<Config> {
        Ok(self.into_connection_setup()?.config)
    }

    fn into_connection_setup(self) -> Result<ConnectionSetup> {
        let mut setup = self.inner.into_connection_setup()?;

        if let Some(provider) = self.credentials_provider {
            setup = setup.with_credentials_provider(provider);
        }

        if let Some(provider) = self.sentinel_credentials_provider {
            setup = setup.with_sentinel_credentials_provider(provider);
        }

        Ok(setup)
    }
}

/// Extension methods for attaching dynamic credentials providers to any
/// [`IntoConfig`] input accepted by `rustis`.
pub trait WithCredentialsProvider: IntoConfig + Sized {
    /// Use this provider for every new Redis data-node TCP session.
    #[must_use]
    fn with_credentials_provider(
        self,
        provider: impl Into<SharedCredentialsProvider>,
    ) -> ConfigWithCredentialsProvider<Self> {
        ConfigWithCredentialsProvider::new(self).with_credentials_provider(provider)
    }

    /// Use this provider for every new Sentinel control-plane TCP session.
    #[must_use]
    fn with_sentinel_credentials_provider(
        self,
        provider: impl Into<SharedCredentialsProvider>,
    ) -> ConfigWithCredentialsProvider<Self> {
        ConfigWithCredentialsProvider::new(self).with_sentinel_credentials_provider(provider)
    }
}

impl<T: IntoConfig + Sized> WithCredentialsProvider for T {}
