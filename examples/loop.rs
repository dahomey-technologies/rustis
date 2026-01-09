use rustis::{
    Result,
    client::Client,
    commands::{StringCommands},
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    // On augmente massivement pour "occuper" le CPU
    for i in 0..10_000 {
        let key = format!("key_{}", i); // Ajoute un peu de travail CPU (allocation)
        client.set(&key, 42.423456).await?;
        let _: f64 = client.get(&key).await?;
        
        if i % 1000 == 0 {
            println!("Iteration {}", i);
        }
    }

    Ok(())
}
