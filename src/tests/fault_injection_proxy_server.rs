//! The one proxy test whose upstream is a real Redis: the others script a fake
//! upstream and need no server.

use crate::{
    Result,
    client::Client,
    commands::StringCommands,
    tests::{fault_injection_proxy::FaultProxy, get_default_addr, log_try_init},
};

/// End-to-end proof the harness is usable by a real client: a transparent
/// proxy in front of Redis must be indistinguishable from a direct
/// connection.
#[tokio::test]
async fn a_real_client_round_trips_through_the_transparent_proxy() -> Result<()> {
    log_try_init();
    let proxy = FaultProxy::start(get_default_addr(), vec![]).await.unwrap();

    let client = Client::connect(format!("redis://{}", proxy.addr)).await?;
    client.set("fault_proxy_smoke_key", "value").await?;
    let value: String = client.get("fault_proxy_smoke_key").await?;
    assert_eq!(value, "value");
    client.close().await?;

    Ok(())
}
