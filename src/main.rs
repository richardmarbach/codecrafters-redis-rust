use std::future::Future;

use anyhow::{bail, Context};
use tokio::{
    io::{
        AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
        Lines,
    },
    net::{TcpListener, TcpStream},
};

enum RESP {
    BulkString(BulkString),
    SimpleString(String),
    Array(Array),
}

struct Array {
    data: Vec<RESP>,
}

impl Array {
    async fn read<R: AsyncBufRead + Unpin>(r: &mut Lines<R>, len: &[u8]) -> anyhow::Result<Self> {
        let len = std::str::from_utf8(len)?.parse::<usize>()?;
        let mut data = Vec::with_capacity(len);
        for _ in 0..len {
            data.push(RESP::read(r).await?.context("incomplete array")?);
        }
        Ok(Array { data })
    }
}

enum BulkString {
    Data(String),
    Empty,
}

impl std::fmt::Display for BulkString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BulkString::Data(text) => write!(f, "${}\r\n{}\r\n", text.len(), text),
            BulkString::Empty => write!(f, "$-1\r\n\r\n"),
        }
    }
}

impl std::fmt::Display for RESP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RESP::BulkString(bulk_string) => write!(f, "{}", bulk_string),
            RESP::SimpleString(string) => write!(f, "+{}\r\n", string),
            RESP::Array(arr) => {
                let len = arr.data.len();
                write!(f, "*{}\r\n", len)?;
                for item in arr.data.iter() {
                    item.fmt(f)?;
                }
                write!(f, "\r\n")
            }
        }
    }
}

impl RESP {
    async fn read<R: AsyncBufRead + Unpin>(
        r: &mut Lines<R>,
    ) -> dyn Future<Output = anyhow::Result<Option<Self>>> {
        let Some(line) = r.next_line().await? else {return Ok(None);};
        let bytes = line.as_bytes();

        match bytes[0] {
            b'*' => Ok(Some(RESP::Array(Array::read(r, &bytes[1..]).await?))),
            _ => bail!("Unknown type"),
        }
    }

    async fn write<W: AsyncWrite + Unpin>(&self, w: &mut W) -> anyhow::Result<usize> {
        w.write(format!("{}", &self).as_bytes())
            .await
            .map_err(Into::into)
    }
}

async fn handle_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    let (reader, writer) = stream.split();
    let reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut lines = reader.lines();

    let command = RESP::read(&mut lines).await?;

    // while let Some(line) = lines.next_line().await? {
    //     println!("GOT: {}", line);
    //     writer.write(b"+PONG\r\n").await?;
    //     writer.flush().await?;
    // }

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
