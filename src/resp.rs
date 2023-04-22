use anyhow::{anyhow, Result};
use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct Connection {
    buffer: BytesMut,
    inner: TcpStream,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            buffer: BytesMut::with_capacity(4096),
            inner: stream,
        }
    }

    pub async fn read_value(&mut self) -> Result<Option<Value>> {
        loop {
            let bytes = self.inner.read_buf(&mut self.buffer).await?;

            // Connection closed
            if bytes == 0 {
                return Ok(None);
            }

            if let Some((value, _)) = decode_message(&self.buffer.split())? {
                return Ok(Some(value));
            }
        }
    }

    pub async fn write_value(&mut self, value: Value) -> Result<()> {
        self.inner.write_all(value.encode().as_bytes()).await?;
        Ok(())
    }
}

fn decode_message(buffer: &[u8]) -> Result<Option<(Value, usize)>> {
    match buffer[0] as char {
        '+' => decode_simple_string(buffer),
        '-' => decode_error(buffer),
        ':' => decode_integer(buffer),
        '$' => decode_bulk_string(buffer),
        '*' => decode_array(buffer),
        _ => Err(anyhow!("invalid RESP message")),
    }
}

fn decode_array(buffer: &[u8]) -> Result<Option<(Value, usize)>> {
    let Some((line, size)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };

    let Some(len) = read_number(&line) else {
        return Err(anyhow!("invalid array"));
    };

    if len < 0 {
        return Err(anyhow!("invalid array"));
    }

    let mut values = Vec::with_capacity(len as usize);

    let mut offset = size + 1;
    for _ in 0..len {
        let Some((value, size)) = decode_message(&buffer[offset..])? else {
            return Ok(None);
        };

        values.push(value);
        offset += size + 1;
    }
    Ok(Some((Value::Array(values), offset)))
}

fn decode_bulk_string(buffer: &[u8]) -> Result<Option<(Value, usize)>> {
    let Some((line, size)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };

    let Some(len) = read_number(&line) else {
        return Err(anyhow!("invalid bulk string"));
    };

    if len == -1 {
        return Ok(Some((Value::Null, size)));
    }

    let start = size + 1;
    if buffer.len() < start + (len as usize) + 2 {
        return Err(anyhow!("bulk string too long"));
    }

    let string = String::from_utf8(buffer[start..start + len as usize].to_vec())?;
    Ok(Some((Value::BulkString(string), size + (len as usize) + 2)))
}

fn decode_error(buffer: &[u8]) -> Result<Option<(Value, usize)>> {
    let Some((line, size)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };
    let string = String::from_utf8(line.to_vec())?;
    Ok(Some((Value::SimpleString(string), size)))
}

fn decode_simple_string(buffer: &[u8]) -> Result<Option<(Value, usize)>> {
    let Some((line, size)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };

    let string = String::from_utf8(line.to_vec())?;
    Ok(Some((Value::SimpleString(string), size)))
}

fn decode_integer(buffer: &[u8]) -> Result<Option<(Value, usize)>> {
    let Some((line, size)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };

    let Some(num) = read_number(&line[1..]) else {
        return Err(anyhow!("invalid integer"));
    };
    Ok(Some((Value::Integer(num), size)))
}

fn read_number(buffer: &[u8]) -> Option<i64> {
    let mut size = 0;
    let sign = if buffer[0] == b'-' { -1 } else { 1 };
    let iter = if buffer[0] == b'-' {
        buffer[1..].iter()
    } else {
        buffer.iter()
    };

    for byte in iter {
        if byte < &b'0' || byte > &b'9' {
            return None;
        }

        size = size * 10 + (byte - b'0') as usize;
    }
    return Some(sign * size as i64);
}

fn read_line(buffer: &[u8]) -> Option<(&[u8], usize)> {
    for (i, &byte) in buffer.iter().enumerate() {
        if byte == b'\r' && buffer.get(i + 1) == Some(&b'\n') {
            return Some((&buffer[..i], i + 2));
        }
    }
    return None;
}

#[derive(Debug)]
pub enum Value {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(String),
    Null,
    Array(Vec<Value>),
}

impl Value {
    pub fn to_command(&self) -> Result<Command> {
        match self {
            Value::Array(values) => {
                let mut iter = values.iter();
                let name = match iter.next() {
                    Some(Value::BulkString(s)) => s,
                    _ => return Err(anyhow!("invalid command")),
                };

                let mut args: Vec<&str> = Vec::new();
                for value in iter {
                    match value {
                        Value::BulkString(ref s) => args.push(s),
                        _ => return Err(anyhow!("invalid command")),
                    }
                }

                match name.to_ascii_lowercase().as_str() {
                    "ping" => Ok(Command::Ping),
                    "echo" => {
                        if args.len() < 1 {
                            return Err(anyhow!("echo takes 1 argument"));
                        }

                        Ok(Command::Echo(args[0].into()))
                    }
                    _ => return Err(anyhow!("invalid command")),
                }
            }
            _ => Err(anyhow!("invalid command")),
        }
    }

    fn encode(&self) -> String {
        match self {
            Value::SimpleString(s) => format!("+{}\r\n", s),
            Value::Null => "$-1\r\n".to_string(),
            Value::Integer(i) => format!(":{}\r\n", i),
            Value::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s),
            Value::Error(s) => format!("-{}\r\n", s),
            Value::Array(values) => {
                let mut string = format!("*{}\r\n", values.len());
                for value in values {
                    string.push_str(&value.encode());
                }
                string
            }
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Ping,
    Echo(String),
}
