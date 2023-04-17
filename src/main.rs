use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
};

async fn handle_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    let (reader, writer) = stream.split();
    let reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        println!("GOT: {}", line);
        writer.write(b"+PONG\r\n").await?;
        writer.flush().await?;
    }

    Ok(())
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
