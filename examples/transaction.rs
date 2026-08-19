//! Running commands atomically, with and without an optimistic lock.
//!
//! A transaction is `MULTI`/`EXEC`: the queued commands run as one unit, with no
//! other client's command in between. Unlike a pipeline it isolates; like a
//! pipeline it costs one round trip.
//!
//! `WATCH` adds the optimistic lock — the transaction is abandoned if a watched
//! key changed meanwhile. It attaches state to the connection, so it needs an
//! [`ExclusiveClient`]: on a shared connection the lock would belong to every
//! clone at once.
//!
//! ```sh
//! cargo run --example transaction
//! ```
use rustis::{
    Result,
    client::{BatchPreparedCommand, Client, ExclusiveClient},
    commands::{StringCommands, TransactionCommands},
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    // Atomic, no lock: the two SETs are applied together or not at all.
    let mut transaction = client.create_transaction();
    transaction.set("account:a", 100).forget();
    transaction.set("account:b", 0).forget();
    transaction.execute::<()>().await?;

    // Optimistic lock: read, decide, then write only if nobody touched the key.
    let client = ExclusiveClient::connect("127.0.0.1:6379").await?;
    client.watch("account:a").await?;

    let balance: i64 = client.get("account:a").await?;
    let transfer = 30;

    if balance < transfer {
        client.unwatch().await?;
        println!("not enough on account:a");
        return Ok(());
    }

    let mut transaction = client.create_transaction();
    transaction.set("account:a", balance - transfer).forget();
    transaction.incr("account:b").forget();

    // If another client wrote `account:a` since the WATCH, this fails rather
    // than applying a decision made on a stale read. Retry from the WATCH.
    match transaction.execute::<()>().await {
        Ok(()) => println!("transferred {transfer}"),
        Err(e) => println!("aborted, someone else wrote first: {e}"),
    }

    Ok(())
}
