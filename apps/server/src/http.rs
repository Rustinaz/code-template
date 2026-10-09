//! The HTTP/1.1 wire layer: parse a request, write a response.
//!
//! Kept separate from the routing in [`crate::Server`] so that parsing is
//! testable against a `Cursor` instead of a socket. It is deliberately minimal
//! — request line, a handful of headers, a `Content-Length` body — which is
//! everything this API needs and nothing it does not.

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};

use shared::api;

/// Refuse bodies larger than this. A template backend still should not let a
/// single header allocation exhaust the process.
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// A parsed request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Uppercase method as sent, e.g. `GET`.
    pub method: String,
    /// Path with no query string, e.g. `/api/users`.
    pub path: String,
    /// Decoded query parameters, last value wins for a repeated key.
    pub query: BTreeMap<String, String>,
    /// Raw body bytes. Empty when there is no body.
    pub body: Vec<u8>,
}

impl Request {
    /// A `GET` request with no query and no body. Handy in tests.
    pub fn get(path: &str) -> Self {
        Self {
            method: "GET".to_string(),
            path: path.to_string(),
            query: BTreeMap::new(),
            body: Vec::new(),
        }
    }
}

/// A response ready to be serialised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status code.
    pub status: u16,
    /// Value of the `Content-Type` header.
    pub content_type: String,
    /// Extra headers, written as-is.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

impl Response {
    /// A JSON response. Serialisation failure cannot happen for the types in
    /// [`shared::api`], but if it ever does the body says so rather than
    /// panicking inside a request handler.
    pub fn json(status: u16, value: &impl serde::Serialize) -> Self {
        let body = serde_json::to_vec(value)
            .unwrap_or_else(|_| br#"{"error":"response could not be serialized"}"#.to_vec());
        Self {
            status,
            content_type: api::CONTENT_TYPE_JSON.to_string(),
            headers: Vec::new(),
            body,
        }
    }

    /// A plain-text response.
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            headers: Vec::new(),
            body: body.into().into_bytes(),
        }
    }

    /// A bodyless response.
    pub fn empty(status: u16) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Adds a header. Chainable.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Writes the full response, always with `Connection: close`.
    pub fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let reason = reason_phrase(self.status);
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n",
            self.status,
            reason,
            self.content_type,
            self.body.len()
        );
        for (name, value) in &self.headers {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("Connection: close\r\n\r\n");

        writer.write_all(head.as_bytes())?;
        writer.write_all(&self.body)?;
        writer.flush()
    }
}

/// The reason phrase for a status code. `Unknown` for anything unmapped.
#[must_use]
pub fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        501 => "Not Implemented",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

/// Reads one request from `reader`.
///
/// Returns `Ok(None)` on a clean end of stream before any request line, which is
/// how a client that opened a connection and left is handled.
pub fn read_request(reader: &mut impl BufRead) -> io::Result<Option<Request>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }

    let request_line = line.trim_end_matches(['\r', '\n']);
    if request_line.is_empty() {
        return Ok(None);
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or("/").to_string();
    // The HTTP version is not used, but its absence means this is not a request
    // line at all.
    if parts.next().is_none() || method.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "malformed request line",
        ));
    }

    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), parse_query(query)),
        None => (target, BTreeMap::new()),
    };

    let content_length = read_headers(reader)?;

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    Ok(Some(Request {
        method,
        path,
        query,
        body,
    }))
}

/// Reads headers until the blank line, returning the declared body length.
fn read_headers(reader: &mut impl BufRead) -> io::Result<usize> {
    let mut content_length = 0usize;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Content-Length")
            })?;
        }
    }

    if content_length > MAX_BODY_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "request body too large",
        ));
    }

    Ok(content_length)
}

/// Parses `a=1&b=two`, percent-decoding both halves.
#[must_use]
pub fn parse_query(query: &str) -> BTreeMap<String, String> {
    let mut params = BTreeMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        params.insert(api::percent_decode(key), api::percent_decode(value));
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn read(raw: &str) -> io::Result<Option<Request>> {
        read_request(&mut Cursor::new(raw.as_bytes()))
    }

    #[test]
    fn parse_query_decodes_percent_escapes() {
        let params = parse_query("q=ada%20lovelace&limit=10");
        assert_eq!(params.get("q").map(String::as_str), Some("ada lovelace"));
        assert_eq!(params.get("limit").map(String::as_str), Some("10"));
    }

    #[test]
    fn parse_query_keeps_a_key_without_a_value() {
        assert_eq!(
            parse_query("debug").get("debug").map(String::as_str),
            Some("")
        );
    }

    #[test]
    fn read_request_parses_a_get() {
        let request = read("GET /api/health HTTP/1.1\r\nHost: x\r\n\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/health");
        assert!(request.query.is_empty());
        assert!(request.body.is_empty());
    }

    #[test]
    fn read_request_parses_query_and_body() {
        let raw = "POST /api/users?limit=5 HTTP/1.1\r\nContent-Length: 13\r\n\r\nhello, world!";
        let request = read(raw).unwrap().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/users");
        assert_eq!(request.query.get("limit").map(String::as_str), Some("5"));
        assert_eq!(request.body, b"hello, world!");
    }

    #[test]
    fn read_request_returns_none_at_eof() {
        assert!(read("").unwrap().is_none());
    }

    #[test]
    fn read_request_rejects_a_bad_content_length() {
        let raw = "POST /x HTTP/1.1\r\nContent-Length: nope\r\n\r\n";
        assert!(read(raw).is_err());
    }

    #[test]
    fn response_writes_status_headers_and_body() {
        let response =
            Response::json(201, &api::ErrorResponse::new("ok")).with_header("Allow", "GET");
        let mut out = Vec::new();
        response.write_to(&mut out).unwrap();
        let text = String::from_utf8(out).unwrap();

        assert!(text.starts_with("HTTP/1.1 201 Created\r\n"));
        assert!(text.contains("Content-Type: application/json\r\n"));
        assert!(text.contains("Allow: GET\r\n"));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.ends_with("{\"error\":\"ok\"}"));
    }

    #[test]
    fn empty_response_has_zero_length() {
        let mut out = Vec::new();
        Response::empty(204).write_to(&mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Content-Length: 0\r\n"), "{text}");
    }
}
