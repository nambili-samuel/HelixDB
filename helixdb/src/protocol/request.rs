use std::{collections::HashMap};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader, BufWriter, Result};

#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub headers: HashMap<String, String>,
    pub path: String,
    pub body: Vec<u8>,
}

impl Request {
    /// Parse a request from a stream
    /// 
    /// # Example
    /// 
    /// ```rust 
    /// use std::io::Cursor;
    /// use helixdb::protocol::request::Request;
    /// 
    /// let request = Request::from_stream(Cursor::new("GET /test HTTP/1.1\r\n\r\n")).unwrap();
    /// assert_eq!(request.method, "GET");
    /// assert_eq!(request.path, "/test");
    /// ```
    pub async fn from_stream<R: AsyncRead + Unpin>(stream: &mut R) -> Result<Request> {
        let mut reader = BufReader::new(stream);
        let mut first_line = String::new();
        reader.read_line(&mut first_line).await?;

        // Get method and path
        let mut parts = first_line.trim().split_whitespace();
        let method = parts.next()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Missing HTTP method: {}", first_line)
            ))?.to_string();
        let path = parts.next()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Missing path: {}", first_line)
            ))?.to_string();

        // Parse headers
        let mut headers = HashMap::new();
        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 || line.eq("\r\n") || line.eq("\n") {
                break;
            }
            if let Some((key, value)) = line.trim().split_once(':') {
                headers.insert(
                    key.trim().to_lowercase(),
                    value.trim().to_string()
                );
            }
        }

        // Read body
        let mut body = Vec::new();
        if let Some(length) = headers.get("content-length") {
            if let Ok(length) = length.parse::<usize>() {
                let mut buffer = vec![0; length];
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5), 
                    reader.read_exact(&mut buffer)
                ).await {
                    Ok(Ok(_)) => body = buffer,
                    Ok(Err(e)) => {
                        eprintln!("Error reading body: {}", e);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Error reading body"
                        ));
                    },
                    Err(_) => {
                        eprintln!("Timeout reading body");
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "Timeout reading body"
                        ));
                    }
                }
            }
        }

        Ok(Request {
            method,
            headers,
            path,
            body,
        })
    }
}