//! Sending a batch of commands in one round trip.
//!
//! A pipeline writes every queued command before reading any reply, so N
//! commands cost one round trip instead of N. The server runs them in order but
//! does **not** isolate them: another client's commands may interleave. Use a
//! transaction when that matters — see `transaction.rs`.
//!
//! ```sh
//! cargo run --example pipelining
//! ```
use rustis::{
    Result,
    client::{BatchPreparedCommand, Client},
    commands::StringCommands,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("pipelined:1", "one").queue();
    pipeline.set("pipelined:2", "two").queue();
    pipeline.get::<()>("pipelined:1").queue();
    pipeline.get::<()>("pipelined:2").queue();

    // The type here decides the decoding: one element per queued command, in
    // order. The `::<()>` written above is discarded on queuing.
    let (_, _, first, second): (String, String, String, String) = pipeline.execute().await?;
    println!("{first} {second}");

    // `forget_command` queues a command whose reply is dropped, so it takes no
    // slot in the tuple above. Useful for the writes you do not read back.
    let mut pipeline = client.create_pipeline();
    pipeline.set("pipelined:3", "three").forget();
    pipeline.get::<()>("pipelined:3").queue();
    let (third,): (String,) = pipeline.execute().await?;
    println!("{third}");

    Ok(())
}
