use crate::{
    Error, Result, RetryReason, StandaloneConnection,
    client::{
        Config, CredentialsReason, CredentialsTarget, SentinelConfig, SharedCredentialsProvider,
    },
    commands::{RoleResult, SentinelCommands, ServerCommands},
    resp::{Command, RespResponse},
    sleep,
};
use log::debug;
use std::{sync::Arc, task::Poll};

pub struct SentinelConnection {
    sentinel_config: SentinelConfig,
    config: Config,
    credentials_provider: Option<SharedCredentialsProvider>,
    sentinel_credentials_provider: Option<SharedCredentialsProvider>,
    pub inner_connection: StandaloneConnection,
}

impl SentinelConnection {
    #[inline]
    pub async fn feed(&mut self, command: &Command, retry_reasons: &[RetryReason]) -> Result<()> {
        self.inner_connection.feed(command, retry_reasons).await
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<()> {
        self.inner_connection.flush().await
    }

    #[inline]
    pub async fn read(&mut self) -> Option<Result<RespResponse>> {
        self.inner_connection.read().await
    }

    #[inline]
    pub fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
        self.inner_connection.try_read()
    }

    #[inline]
    pub async fn reconnect(&mut self) -> Result<()> {
        self.inner_connection = Self::connect_to_sentinel(
            &self.sentinel_config,
            &self.config,
            self.credentials_provider.clone(),
            self.sentinel_credentials_provider.clone(),
            CredentialsReason::Reconnect,
        )
        .await?;

        Ok(())
    }

    /// Follow `Redis service discovery via Sentinel` documentation
    /// #See <https://redis.io/docs/reference/sentinel-clients/#redis-service-discovery-via-sentinel>
    ///
    /// # Remark
    /// this function must be desugared because of async recursion:
    /// <https://doc.rust-lang.org/error-index.html#E0733>
    pub async fn connect(
        sentinel_config: &SentinelConfig,
        config: &Config,
        credentials_provider: Option<SharedCredentialsProvider>,
        sentinel_credentials_provider: Option<SharedCredentialsProvider>,
    ) -> Result<SentinelConnection> {
        let inner_connection = Self::connect_to_sentinel(
            sentinel_config,
            config,
            credentials_provider.clone(),
            sentinel_credentials_provider.clone(),
            CredentialsReason::InitialConnect,
        )
        .await?;

        Ok(SentinelConnection {
            sentinel_config: sentinel_config.clone(),
            config: config.clone(),
            credentials_provider,
            sentinel_credentials_provider,
            inner_connection,
        })
    }

    async fn connect_to_sentinel(
        sentinel_config: &SentinelConfig,
        config: &Config,
        credentials_provider: Option<SharedCredentialsProvider>,
        sentinel_credentials_provider: Option<SharedCredentialsProvider>,
        credentials_reason: CredentialsReason,
    ) -> Result<StandaloneConnection> {
        let mut restart = false;
        let mut unreachable_sentinel = true;

        let mut sentinel_node_config = config.clone();
        sentinel_node_config
            .username
            .clone_from(&sentinel_config.username);
        sentinel_node_config
            .password
            .clone_from(&sentinel_config.password);

        loop {
            for sentinel_instance in &sentinel_config.instances {
                // Step 1: connecting to Sentinel
                let (host, port) = sentinel_instance;

                let mut sentinel_connection = match StandaloneConnection::connect_with_context(
                    host,
                    *port,
                    &sentinel_node_config,
                    sentinel_credentials_provider.clone(),
                    credentials_reason,
                    CredentialsTarget::SentinelNode,
                )
                .await
                {
                    Ok(sentinel_connection) => sentinel_connection,
                    Err(e) => {
                        debug!("Cannot connect to Sentinel {}:{} : {}", *host, *port, e);
                        continue;
                    }
                };

                // Step 2: ask for master address
                let (master_host, master_port) = match sentinel_connection
                    .sentinel_get_master_addr_by_name(sentinel_config.service_name.clone())
                    .await
                {
                    Ok(Some((master_host, master_port))) => (master_host, master_port),
                    Ok(None) => {
                        debug!(
                            "Sentinel {}:{} does not know master `{}`",
                            *host, *port, sentinel_config.service_name
                        );
                        unreachable_sentinel = false;
                        continue;
                    }
                    Err(e) => {
                        debug!(
                            "Cannot execute command `SENTINEL get-master-addr-by-name` with Sentinel {}:{}: {}",
                            *host, *port, e
                        );
                        continue;
                    }
                };

                // Step 3: call the ROLE command in the target instance
                let mut master_connection = StandaloneConnection::connect_with_context(
                    &master_host,
                    master_port,
                    config,
                    credentials_provider.clone(),
                    credentials_reason,
                    CredentialsTarget::DataNode,
                )
                .await?;

                let role: RoleResult = master_connection.role().await?;

                if let RoleResult::Master {
                    master_replication_offset: _,
                    replica_infos: _,
                } = role
                {
                    return Ok(master_connection);
                } else {
                    sleep(sentinel_config.wait_between_failures).await;
                    // restart from the beginning
                    restart = true;
                    break;
                }
            }

            if !restart {
                break;
            } else {
                restart = false;
            }
        }

        if unreachable_sentinel {
            Err(Error::Sentinel(
                "All Sentinel instances are unreachable".to_owned(),
            ))
        } else {
            Err(Error::Sentinel(format!(
                "master {} is unknown by all Sentinel instances",
                sentinel_config.service_name
            )))
        }
    }

    pub(crate) fn tag(&self) -> Arc<str> {
        self.inner_connection.tag()
    }
}
