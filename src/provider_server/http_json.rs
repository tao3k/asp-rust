//! Minimal bounded loopback HTTP/JSON server for the provider executable.

use std::future::Future;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

const MAX_REQUEST_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub(super) struct HttpJsonRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) body: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct HttpJsonResponse {
    pub(super) status: u16,
    pub(super) body: Vec<u8>,
}

impl HttpJsonResponse {
    pub(super) fn json(status: u16, value: &serde_json::Value) -> Result<Self, String> {
        serde_json::to_vec(value)
            .map(|body| Self { status, body })
            .map_err(|error| format!("encode HTTP JSON response: {error}"))
    }
}

pub(super) async fn serve_http_json<H, F>(
    listener: TcpListener,
    shutdown: watch::Receiver<bool>,
    handler: H,
) -> Result<(), String>
where
    H: Fn(HttpJsonRequest) -> F + Send + Sync + 'static,
    F: Future<Output = Result<HttpJsonResponse, String>> + Send,
{
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|error| format!("accept HTTP JSON connection: {error}"))?;
        serve_connection(stream, &handler).await?;
        if *shutdown.borrow() {
            return Ok(());
        }
    }
}

async fn serve_connection<H, F>(mut stream: TcpStream, handler: &H) -> Result<(), String>
where
    H: Fn(HttpJsonRequest) -> F,
    F: Future<Output = Result<HttpJsonResponse, String>>,
{
    let request = read_request(&mut stream).await?;
    let response = handler(request)
        .await
        .unwrap_or_else(|error| HttpJsonResponse {
            status: 500,
            body: serde_json::to_vec(&serde_json::json!({"error": error}))
                .unwrap_or_else(|_| b"{\"error\":\"internal-server-error\"}".to_vec()),
        });
    write_response(&mut stream, response).await
}

async fn read_request(stream: &mut TcpStream) -> Result<HttpJsonRequest, String> {
    let mut bytes = Vec::with_capacity(4096);
    let header_end = loop {
        if bytes.len() >= MAX_REQUEST_BYTES {
            return Err("HTTP JSON request exceeds the bounded frame limit".to_owned());
        }
        let mut chunk = [0_u8; 4096];
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|error| format!("read HTTP JSON request: {error}"))?;
        if read == 0 {
            return Err("HTTP JSON request ended before its headers".to_owned());
        }
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let header = std::str::from_utf8(&bytes[..header_end])
        .map_err(|error| format!("HTTP JSON request header is not UTF-8: {error}"))?;
    let mut lines = header.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| "HTTP JSON request line is absent".to_owned())?;
    let mut request_line = request_line.split_whitespace();
    let method = request_line.next().unwrap_or_default().to_owned();
    let path = request_line.next().unwrap_or_default().to_owned();
    if method.is_empty() || path.is_empty() || request_line.next() != Some("HTTP/1.1") {
        return Err("HTTP JSON request line is invalid".to_owned());
    }
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.trim().parse::<usize>())
        .transpose()
        .map_err(|error| format!("HTTP JSON content-length is invalid: {error}"))?
        .unwrap_or(0);
    if header_end + content_length > MAX_REQUEST_BYTES {
        return Err("HTTP JSON request exceeds the bounded frame limit".to_owned());
    }
    while bytes.len() < header_end + content_length {
        let mut chunk = [0_u8; 4096];
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|error| format!("read HTTP JSON request body: {error}"))?;
        if read == 0 {
            return Err("HTTP JSON request body ended early".to_owned());
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    Ok(HttpJsonRequest {
        method,
        path,
        body: bytes[header_end..header_end + content_length].to_vec(),
    })
}

async fn write_response(stream: &mut TcpStream, response: HttpJsonResponse) -> Result<(), String> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.body.len()
    );
    stream
        .write_all(header.as_bytes())
        .await
        .map_err(|error| format!("write HTTP JSON response header: {error}"))?;
    stream
        .write_all(&response.body)
        .await
        .map_err(|error| format!("write HTTP JSON response body: {error}"))?;
    stream
        .shutdown()
        .await
        .map_err(|error| format!("shutdown HTTP JSON connection: {error}"))
}
