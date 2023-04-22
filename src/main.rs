use redis_starter_rust::resp;
use tokio::net::{TcpListener, TcpStream};

async fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    let mut conn = resp::Connection::new(stream);

    loop {
        let Some(value) = conn.read_value().await? else { return Ok(()) };
        println!("{:?}", value);

        match value.to_command()? {
            resp::Command::Ping => {
                let value = resp::Value::SimpleString("PONG".to_string());
                conn.write_value(value).await?;
            }
            resp::Command::Echo(value) => {
                let value = resp::Value::BulkString(value);
                conn.write_value(value).await?;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        let incoming = listener.accept().await;
        match incoming {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    handle_connection(stream).await.unwrap_or_else(|err| {
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
