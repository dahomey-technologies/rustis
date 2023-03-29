use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{CommandArgs, ToArgs, SingleArg, PrimitiveResponse, RespSerializer},
    Result,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct MyPerson {
    pub name: String,
    pub age: i32,
    pub children: Vec<MyPerson>,
}

 impl ToArgs for MyPerson {
    #[inline]
    fn write_args(&self, args: &mut CommandArgs) {
        // Question: What is proper way to implement this for a Serializable type assuming there
        // is out of the box ability to query types which implements Deserialize? 
        let mut serializer = RespSerializer::new();
        self.serialize(&mut serializer).unwrap();
        let buf = serializer.get_output();
        let slice = &buf[..];
        args.arg(slice);
    }
}

impl SingleArg for MyPerson {}

impl PrimitiveResponse for MyPerson {}

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    let person = MyPerson {
        name: "Dad".to_owned(),
        age: 35,
        children: vec![
            MyPerson {
                name: "Son".to_owned(),
                age: 3,
                children: vec![]
            },
            MyPerson {
                name: "Daughter".to_owned(),
                age: 1,
                children: vec![]
            }
        ],
    };
    client.set("key", person.clone()).await?;
    let saved_person: MyPerson = client.get("key").await?;
    assert!(saved_person == person);

    Ok(())
}