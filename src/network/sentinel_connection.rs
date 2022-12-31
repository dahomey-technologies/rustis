use crate::{
    client::{Config, SentinelConfig},
    commands::{RoleResult, SentinelCommands, ServerCommands},
    resp::{Command, Value},
    sleep, Error, Result, RetryReason, StandaloneConnection,
};
use log::debug;

pub struct SentinelConnection {
    pub inner_connection: StandaloneConnection,
}

impl SentinelConnection {
    #[inline]
    pub async fn write(&mut self, command: &Command) -> Result<()> {
        self.inner_connection.write(command).await
    }

    #[inline]
    pub async fn write_batch(
        &mut self,
        commands: impl Iterator<Item = &mut Command>,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
        self.inner_connection
            .write_batch(commands, retry_reasons)
            .await
    }

    #[inline]
    pub async fn read(&mut self) -> Option<Result<Value>> {
        self.inner_connection.read().await
    }

    #[inline]
    pub async fn reconnect(&mut self) -> Result<()> {
        self.inner_connection.reconnect().await
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
    ) -> Result<SentinelConnection> {
        let mut restart = false;
        let mut unreachable_sentinel = true;

        let mut config = config.clone();
        config.username = sentinel_config.username.clone();
        config.password = sentinel_config.password.clone();

        loop {
            for sentinel_instance in &sentinel_config.instances {
                // Step 1: connecting to Sentinel
                let (host, port) = sentinel_instance;

                let mut sentinel_connection =
                    match StandaloneConnection::connect(host, *port, &config).await {
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
                        debug!("Cannot execute command `SENTINEL get-master-addr-by-name` with Sentinel {}:{}: {}", *host, *port, e);
                        continue;
                    }
                };

                // Step 3: call the ROLE command in the target instance
                let mut master_connection =
                    StandaloneConnection::connect(&master_host, master_port, &config).await?;

                let role: RoleResult = master_connection.role().await?;

                if let RoleResult::Master {
                    master_replication_offset: _,
                    replica_infos: _,
                } = role
                {
                    return Ok(SentinelConnection {
                        inner_connection: master_connection,
                    });
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
}
