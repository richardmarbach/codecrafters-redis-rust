use std::sync::Arc;

use redis_starter_rust::{
    resp,
    store::{self, Store},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

async fn handle_connection(stream: TcpStream, store: Arc<Mutex<Store>>) -> anyhow::Result<()> {
    let mut conn = resp::Connection::new(stream);

    loop {
        let Some(value) = conn.read_value().await? else { return Ok(()) };

        match value.to_command()? {
            resp::Command::Ping => {
                let value = resp::Value::SimpleString("PONG".to_string());
                conn.write_value(value).await?;
            }
            resp::Command::Echo(value) => {
                let value = resp::Value::BulkString(value);
                conn.write_value(value).await?;
            }
            resp::Command::Get(key) => {
                let mut store = store.lock().await;
                let value = store.get(&key);
                conn.write_value(value).await?;
            }
            resp::Command::Set(key, value, px) => {
                let mut store = store.lock().await;

                match px {
                    Some(px) => store.set_px(key, value, px),
                    None => store.set(key, value),
                };
                let value = resp::Value::SimpleString("OK".to_string());
                conn.write_value(value).await?;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    let store = Arc::new(Mutex::new(store::Store::new()));

    loop {
        let incoming = listener.accept().await;
        match incoming {
            Ok((stream, _)) => {
                let store = store.clone();
                tokio::spawn(async move {
                    handle_connection(stream, store)
                        .await
                        .unwrap_or_else(|err| {
                            eprintln!("{}", err);
                        });
                });
            }
            Err(err) => {
                eprintln!("{}", err);
            }
        };
    }
}
