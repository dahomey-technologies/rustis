cd redis && .\docker_up.cmd && cd .. && cargo test --features tokio-rustls,pool,json,client-cache -- --test-threads=1
