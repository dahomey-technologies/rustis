use rustis::{
    Result,
    client::Client,
    commands::{GenericCommands, StringCommands},
    resp::BulkString,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    let key = "test_key";
    // {"foo": "bar"} in CBOR
    let cbor_value = b"\xa1\x63\x66\x6F\x6F\x63\x62\x61\x72";

    client.set(key, cbor_value).await?;
    let value: BulkString = client.get(key).await?;
    assert_eq!(cbor_value, value.as_bytes());
    client.del(key).await?;

    Ok(())
}
