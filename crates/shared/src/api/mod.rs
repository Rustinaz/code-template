//! The HTTP contract shared by the backend and every client.
//!
//! This module is the single place the two ends agree. [`endpoints`] holds the
//! paths, the request and response types below are what travels over them, and
//! both `apps/server` and `apps/example` import them rather than restating the
//! shape. A renamed field is then a compile error on both sides instead of a
//! 400 at runtime.
//!
//! The types are plain `serde` structs, so the same definition is the JSON body
//! on the wire *and* the request/response argument in Rust.

use serde::{Deserialize, Serialize};

/// Every path the backend serves.
///
/// Clients build requests from these constants, so a path can never drift
/// between the two crates.
pub mod endpoints {
    /// `GET` — liveness and build information.
    pub const HEALTH: &str = "/api/health";
    /// `GET` to list, `POST` to register.
    pub const USERS: &str = "/api/users";
    /// `GET` with a `?q=` query — substring search over users.
    pub const USERS_SEARCH: &str = "/api/users/search";

    /// Builds the search path for `query`, percent-encoding the value.
    #[must_use]
    pub fn users_search(query: &str) -> String {
        format!("{USERS_SEARCH}?q={}", super::percent_encode(query))
    }
}

/// The `Content-Type` both ends use. There is no HTML here on purpose.
pub const CONTENT_TYPE_JSON: &str = "application/json";

/// Body of `GET /api/health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Always `"ok"` when the process is answering at all.
    pub status: String,
    /// The backend crate version, from `CARGO_PKG_VERSION`.
    pub version: String,
    /// Whole seconds since the process started.
    pub uptime_seconds: u64,
}

/// Body of `POST /api/users`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterRequest {
    /// Login email. Must be unique.
    pub email: String,
    /// Login handle. Must be unique.
    pub username: String,
    /// At least [`MIN_PASSWORD_LEN`](crate::services::defaults::MIN_PASSWORD_LEN)
    /// characters; the server enforces it.
    pub password: String,
    /// Human-readable name shown in the UI.
    pub display_name: String,
}

/// Body returned by both user-listing endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserList {
    /// The page of users, in whatever order the store produced.
    pub users: Vec<crate::domain::User>,
    /// Number of users in `users`, so a client need not re-count.
    pub total: usize,
}

impl UserList {
    /// Wraps a page of users, filling in `total`.
    #[must_use]
    pub fn new(users: Vec<crate::domain::User>) -> Self {
        Self {
            total: users.len(),
            users,
        }
    }
}

/// Body returned for any non-2xx response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Human-readable reason. Safe to show, never a stack trace.
    pub error: String,
}

impl ErrorResponse {
    /// Builds an error body from anything displayable.
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

/// Percent-encodes a query-string value.
///
/// The unreserved set is kept verbatim, a space becomes `%20`, and everything
/// else becomes uppercase `%XX`. Space is deliberately *not* turned into `+`:
/// the decoder in `apps/server` treats `+` literally, so encoding it as `%20`
/// is what makes the round trip lossless.
#[must_use]
pub fn percent_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    out
}

/// Reverses [`percent_encode`].
///
/// A stray `%` that is not followed by two hex digits is kept as a literal `%`
/// rather than dropping the request, and invalid UTF-8 is replaced, because a
/// malformed query should produce a miss, not a crash.
#[must_use]
pub fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        let decoded = if bytes[index] == b'%' && index + 2 < bytes.len() {
            match (hex_value(bytes[index + 1]), hex_value(bytes[index + 2])) {
                (Some(high), Some(low)) => Some((high << 4) | low),
                _ => None,
            }
        } else {
            None
        };

        match decoded {
            Some(byte) => {
                out.push(byte);
                index += 3;
            }
            None => {
                out.push(bytes[index]);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_keeps_the_unreserved_set() {
        assert_eq!(percent_encode("Az-9_.~"), "Az-9_.~");
    }

    #[test]
    fn percent_encode_escapes_a_space_as_percent_twenty() {
        assert_eq!(percent_encode("a b"), "a%20b");
    }

    #[test]
    fn percent_encode_escapes_utf8_byte_by_byte() {
        // "é" is two UTF-8 bytes: C3 A9.
        assert_eq!(percent_encode("café"), "caf%C3%A9");
    }

    #[test]
    fn percent_encode_escapes_a_literal_plus() {
        // The decoder treats `+` literally, so encoding must too.
        assert_eq!(percent_encode("a+b"), "a%2Bb");
    }

    #[test]
    fn percent_round_trips_including_spaces_and_percent() {
        for value in ["", "a b", "100%", "café", "a+b", "x/y?z=1&w"] {
            assert_eq!(percent_decode(&percent_encode(value)), value, "{value:?}");
        }
    }

    #[test]
    fn percent_decode_keeps_a_stray_percent() {
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn users_search_builds_an_encoded_path() {
        assert_eq!(
            endpoints::users_search("ada lovelace"),
            "/api/users/search?q=ada%20lovelace"
        );
    }

    #[test]
    fn user_list_total_is_derived_from_the_items() {
        let list = UserList::new(Vec::new());
        assert_eq!(list.total, 0);
        assert!(list.users.is_empty());
    }

    #[test]
    fn health_response_round_trips_as_json() {
        let health = HealthResponse {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
            uptime_seconds: 3,
        };
        let json = serde_json::to_string(&health).unwrap();
        assert_eq!(
            serde_json::from_str::<HealthResponse>(&json).unwrap(),
            health
        );
    }

    #[test]
    fn register_request_round_trips_as_json() {
        let request = RegisterRequest {
            email: "ada@example.com".to_string(),
            username: "ada".to_string(),
            password: "correct horse".to_string(),
            display_name: "Ada Lovelace".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serde_json::from_str::<RegisterRequest>(&json).unwrap(),
            request
        );
    }
}
