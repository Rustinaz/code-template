//! A blocking, TLS-free HTTP/1.1 client.
//!
//! This is the client half of the bundled backend in `apps/server`. It exists
//! so the template can demonstrate a real client/server round trip without the
//! `reqwest` dependency that the `network` feature pulls in: `reqwest` brings
//! TLS and a native TLS provider, neither of which cross-compiles cleanly to
//! every target this template supports.
//!
//! What it is *not*: a general-purpose HTTP client. It speaks plain HTTP/1.1,
//! closes the connection after every request, and refuses `https://`. That is
//! exactly enough to talk to `apps/server` over localhost or a LAN, and the
//! module is small enough to replace wholesale when a project outgrows it.
//!
//! Only compiled for targets that have raw sockets, so never for
//! `wasm32-unknown-unknown`.

use std::io::{Read, Write};
use std::time::Duration;

use async_trait::async_trait;

use super::NetworkService;
use crate::errors::{AppError, NetworkError, Result};

/// A plain-HTTP client bound to one base URL, e.g. `http://127.0.0.1:8080`.
///
/// Requests are synchronous. [`NetworkService`] is async, and the trait impl
/// below simply wraps these blocking calls; the example app runs them on a
/// worker thread so the UI never blocks on I/O.
pub struct HttpNetworkService {
    base_url: String,
    timeout: Duration,
}

impl HttpNetworkService {
    /// Builds a client. `base_url` must start with `http://`; `https://` is
    /// rejected at request time with a clear error rather than a confusing
    /// connection failure.
    pub fn new(base_url: impl Into<String>, timeout: Duration) -> Self {
        Self {
            base_url: base_url.into(),
            timeout,
        }
    }

    /// The base URL every request is resolved against.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Synchronous `GET`, returning the parsed JSON body.
    pub fn get_json(&self, endpoint: &str) -> Result<serde_json::Value> {
        let response = self.send("GET", endpoint, None)?;
        Self::expect_json(response)
    }

    /// Synchronous `POST` of a JSON body, returning the parsed JSON body.
    pub fn post_json(&self, endpoint: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let response = self.send("POST", endpoint, Some(body))?;
        Self::expect_json(response)
    }

    /// Sends one request and returns `(status, body)` **without** judging the
    /// status. Useful for tests and for callers that want the raw response.
    pub fn send(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<(u16, Vec<u8>)> {
        let (host, port) = parse_base_url(&self.base_url)?;

        let payload = match body {
            Some(value) => serde_json::to_vec(value)?,
            None => Vec::new(),
        };
        let request = build_request(method, endpoint, &host, port, &payload);

        let mut stream = std::net::TcpStream::connect((host.as_str(), port))?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        stream.write_all(&request)?;
        stream.flush()?;

        // The server answers with `Connection: close`, so reading to EOF is how
        // we know the body ended. `Content-Length` is still honoured by
        // [`parse_http_response`] in case a proxy decides to keep the socket up.
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes)?;
        parse_http_response(&bytes)
    }

    fn expect_json((status, body): (u16, Vec<u8>)) -> Result<serde_json::Value> {
        if !(200..=299).contains(&status) {
            let message = String::from_utf8_lossy(&body).trim().to_string();
            return Err(AppError::Network(NetworkError::HttpError {
                status,
                message,
            }));
        }
        if body.is_empty() {
            return Ok(serde_json::Value::Null);
        }
        Ok(serde_json::from_slice(&body)?)
    }
}

#[async_trait]
impl NetworkService for HttpNetworkService {
    async fn get(&self, endpoint: &str) -> Result<serde_json::Value> {
        self.get_json(endpoint)
    }

    async fn post(&self, endpoint: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.post_json(endpoint, body)
    }

    async fn put(&self, endpoint: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let response = self.send("PUT", endpoint, Some(body))?;
        Self::expect_json(response)
    }

    async fn delete(&self, endpoint: &str) -> Result<()> {
        let (status, body) = self.send("DELETE", endpoint, None)?;
        Self::expect_json((status, body)).map(|_| ())
    }

    async fn upload(
        &self,
        _endpoint: &str,
        _file_path: &std::path::Path,
        _mime_type: &str,
    ) -> Result<String> {
        Err(AppError::Unsupported {
            operation: "upload".to_string(),
            platform: "http-network-service".to_string(),
        })
    }

    async fn download(&self, _url: &str, _destination: &std::path::Path) -> Result<()> {
        Err(AppError::Unsupported {
            operation: "download".to_string(),
            platform: "http-network-service".to_string(),
        })
    }
}

fn invalid_url(url: &str) -> AppError {
    AppError::Network(NetworkError::InvalidUrl {
        url: url.to_string(),
    })
}

fn malformed(reason: &str) -> AppError {
    AppError::Network(NetworkError::ConnectionFailed {
        reason: format!("malformed HTTP response: {reason}"),
    })
}

/// Splits `http://host[:port][/path]` into a host and a port.
///
/// Deliberately narrow: no userinfo, no query on the base URL, no scheme other
/// than `http`. Anything else is a configuration mistake worth reporting early.
fn parse_base_url(base_url: &str) -> Result<(String, u16)> {
    let authority = base_url
        .strip_prefix("http://")
        .ok_or_else(|| invalid_url(base_url))?
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");

    if authority.is_empty() {
        return Err(invalid_url(base_url));
    }

    // A bracketed IPv6 literal, e.g. `[::1]:9000`.
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']').ok_or_else(|| invalid_url(base_url))?;
        let port = match tail.strip_prefix(':') {
            Some(port) => port.parse().map_err(|_| invalid_url(base_url))?,
            None => 80,
        };
        if host.is_empty() {
            return Err(invalid_url(base_url));
        }
        return Ok((host.to_string(), port));
    }

    match authority.split_once(':') {
        Some((host, port)) => {
            if host.is_empty() {
                return Err(invalid_url(base_url));
            }
            let port = port.parse().map_err(|_| invalid_url(base_url))?;
            Ok((host.to_string(), port))
        }
        None => Ok((authority.to_string(), 80)),
    }
}

/// Serialises one HTTP/1.1 request. Always `Connection: close`.
fn build_request(method: &str, endpoint: &str, host: &str, port: u16, body: &[u8]) -> Vec<u8> {
    let target = if endpoint.is_empty() { "/" } else { endpoint };
    let host_header = if port == 80 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };

    let mut request = Vec::with_capacity(body.len() + 256);
    let _ = write!(request, "{method} {target} HTTP/1.1\r\n");
    let _ = write!(request, "Host: {host_header}\r\n");
    let _ = write!(request, "Accept: application/json\r\n");
    let _ = write!(request, "User-Agent: rust-crossplatform-template\r\n");
    let _ = write!(request, "Connection: close\r\n");
    if body.is_empty() {
        let _ = write!(request, "Content-Length: 0\r\n");
    } else {
        let _ = write!(request, "Content-Type: application/json\r\n");
        let _ = write!(request, "Content-Length: {}\r\n", body.len());
    }
    let _ = write!(request, "\r\n");
    request.extend_from_slice(body);
    request
}

/// Reads the status code and body out of a raw response.
fn parse_http_response(bytes: &[u8]) -> Result<(u16, Vec<u8>)> {
    let split = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| malformed("no blank line after the headers"))?;

    let head =
        std::str::from_utf8(&bytes[..split]).map_err(|_| malformed("headers are not UTF-8"))?;
    let mut body = bytes[split + 4..].to_vec();

    let mut lines = head.split("\r\n");
    let status_line = lines.next().ok_or_else(|| malformed("empty response"))?;
    let mut parts = status_line.splitn(3, ' ');
    let _version = parts.next();
    let status: u16 = parts
        .next()
        .and_then(|code| code.trim().parse().ok())
        .ok_or_else(|| malformed("status line has no numeric code"))?;

    // Truncate to Content-Length when present. Without this a peer that ignores
    // `Connection: close` would leave us trailing bytes from the next response.
    let declared = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    });
    if let Some(length) = declared {
        body.truncate(length);
    }

    Ok((status, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_base_url_reads_host_and_port() {
        assert_eq!(
            parse_base_url("http://127.0.0.1:8080").unwrap(),
            ("127.0.0.1".to_string(), 8080)
        );
        assert_eq!(
            parse_base_url("http://localhost").unwrap(),
            ("localhost".to_string(), 80)
        );
        assert_eq!(
            parse_base_url("http://[::1]:9000").unwrap(),
            ("::1".to_string(), 9000)
        );
    }

    #[test]
    fn parse_base_url_ignores_a_path() {
        assert_eq!(
            parse_base_url("http://example.test:1234/api").unwrap(),
            ("example.test".to_string(), 1234)
        );
    }

    #[test]
    fn parse_base_url_rejects_https_and_nonsense() {
        assert!(parse_base_url("https://example.test").is_err());
        assert!(parse_base_url("example.test").is_err());
        assert!(parse_base_url("http://").is_err());
        assert!(parse_base_url("http://host:notaport").is_err());
    }

    #[test]
    fn build_request_writes_a_json_post() {
        let body = b"{\"k\":1}";
        let request = build_request("POST", "/api/users", "127.0.0.1", 8080, body);
        let text = String::from_utf8(request).unwrap();

        assert!(text.starts_with("POST /api/users HTTP/1.1\r\n"));
        assert!(text.contains("Host: 127.0.0.1:8080\r\n"));
        assert!(text.contains("Content-Type: application/json\r\n"));
        assert!(text.contains("Content-Length: 7\r\n"));
        assert!(text.ends_with("{\"k\":1}"));
    }

    #[test]
    fn build_request_omits_the_port_for_eighty() {
        let request = build_request("GET", "/api/health", "example.test", 80, &[]);
        let text = String::from_utf8(request).unwrap();
        assert!(text.contains("Host: example.test\r\n"), "{text}");
        assert!(text.contains("Content-Length: 0\r\n"), "{text}");
    }

    #[test]
    fn parse_http_response_reads_status_and_body() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\":1}";
        let (status, body) = parse_http_response(raw).unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, b"{\"a\":1}");
    }

    #[test]
    fn parse_http_response_honours_content_length() {
        // Trailing bytes past Content-Length must be dropped.
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\n{\"a\":1}EXTRA";
        let (_, body) = parse_http_response(raw).unwrap();
        assert_eq!(body, b"{\"a\":1}");
    }

    #[test]
    fn parse_http_response_rejects_a_response_without_a_blank_line() {
        assert!(parse_http_response(b"HTTP/1.1 200 OK\r\n").is_err());
    }

    #[test]
    fn get_json_reads_a_real_socket_round_trip() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 2048];
            let read = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]).into_owned();

            let body = r#"{"status":"ok","version":"0.1.0","uptime_seconds":1}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            request
        });

        let client = HttpNetworkService::new(format!("http://{address}"), Duration::from_secs(5));
        let value = client.get_json(crate::api::endpoints::HEALTH).unwrap();
        let request = server.join().unwrap();

        assert_eq!(value["status"], "ok");
        assert!(
            request.starts_with("GET /api/health HTTP/1.1\r\n"),
            "{request:?}"
        );
    }

    #[test]
    fn a_non_2xx_status_becomes_an_http_error() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            let body = r#"{"error":"nope"}"#;
            let response = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        });

        let client = HttpNetworkService::new(format!("http://{address}"), Duration::from_secs(5));
        let error = client.get_json("/api/missing").unwrap_err();
        match error {
            AppError::Network(NetworkError::HttpError { status, .. }) => assert_eq!(status, 404),
            other => panic!("expected an HTTP error, got {other:?}"),
        }
    }
}
