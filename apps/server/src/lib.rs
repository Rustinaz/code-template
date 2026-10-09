//! The bundled backend for the template.
//!
//! This crate is the server half of "write the backend and the client in one
//! place": it serves the JSON API whose request and response types live in
//! [`shared::api`], the same types the app in `apps/example` decodes. One
//! definition, two ends.
//!
//! It is intentionally small and dependency-light. There is no async runtime
//! and no web framework: a thread per connection, [`futures::executor::block_on`]
//! to await the `shared` services, and the hand-written HTTP/1.1 layer in
//! [`http`]. That keeps `cargo build` fast and the whole thing readable. A real
//! project would reach for `axum` and a database, and the routing in
//! [`Server::handle`] is the seam where that swap happens.

pub mod http;

use std::io::BufReader;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::executor::block_on;
use shared::api::{endpoints, ErrorResponse, HealthResponse, RegisterRequest, UserList};
use shared::errors::{AppError, AuthError};
use shared::ServiceContainer;

pub use http::{Request, Response};

/// Page size used when a request does not ask for one.
pub const DEFAULT_LIMIT: usize = 50;

/// Largest page a request may ask for.
pub const MAX_LIMIT: usize = 200;

/// How long a single connection may take to read a request or write a response.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// Owns the services and routes requests to them.
pub struct Server {
    services: Arc<ServiceContainer>,
    started: Instant,
    version: &'static str,
}

impl Server {
    /// Builds a server around an already-constructed service container.
    pub fn new(services: Arc<ServiceContainer>) -> Self {
        Self {
            services,
            started: Instant::now(),
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    /// Routes one request. Pure except for the service calls, so it can be
    /// tested without a socket.
    #[must_use]
    pub fn handle(&self, request: &Request) -> Response {
        match request.path.as_str() {
            endpoints::HEALTH => match request.method.as_str() {
                "GET" => self.health(),
                _ => method_not_allowed("GET"),
            },
            endpoints::USERS => match request.method.as_str() {
                "GET" => self.list_users(request),
                "POST" => self.register(request),
                _ => method_not_allowed("GET, POST"),
            },
            endpoints::USERS_SEARCH => match request.method.as_str() {
                "GET" => self.search(request),
                _ => method_not_allowed("GET"),
            },
            _ => Response::json(
                404,
                &ErrorResponse::new(format!("no route for {}", request.path)),
            ),
        }
    }

    /// The services this server routes to. Exposed so `main` can seed demo data.
    pub fn services(&self) -> &Arc<ServiceContainer> {
        &self.services
    }

    fn health(&self) -> Response {
        Response::json(
            200,
            &HealthResponse {
                status: "ok".to_string(),
                version: self.version.to_string(),
                uptime_seconds: self.started.elapsed().as_secs(),
            },
        )
    }

    fn list_users(&self, request: &Request) -> Response {
        match block_on(self.services.users.list_users(limit_of(request))) {
            Ok(users) => Response::json(200, &UserList::new(users)),
            Err(error) => error_response(&error),
        }
    }

    fn search(&self, request: &Request) -> Response {
        let query = request
            .query
            .get("q")
            .map(String::as_str)
            .unwrap_or_default()
            .trim();
        if query.is_empty() {
            return Response::json(
                400,
                &ErrorResponse::new("the `q` query parameter is required"),
            );
        }

        match block_on(self.services.users.search_users(query, limit_of(request))) {
            Ok(users) => Response::json(200, &UserList::new(users)),
            Err(error) => error_response(&error),
        }
    }

    fn register(&self, request: &Request) -> Response {
        let body: RegisterRequest = match serde_json::from_slice(&request.body) {
            Ok(body) => body,
            Err(error) => {
                return Response::json(
                    400,
                    &ErrorResponse::new(format!("invalid JSON body: {error}")),
                )
            }
        };

        match block_on(self.services.auth.register(
            &body.email,
            &body.username,
            &body.password,
            &body.display_name,
        )) {
            Ok(user) => Response::json(201, &user),
            Err(error) => error_response(&error),
        }
    }
}

/// Reads the `limit` query parameter, clamped to a sane page size.
fn limit_of(request: &Request) -> usize {
    request
        .query
        .get("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT)
}

fn method_not_allowed(allowed: &str) -> Response {
    Response::json(
        405,
        &ErrorResponse::new(format!("method not allowed; this route accepts {allowed}")),
    )
    .with_header("Allow", allowed)
}

/// Maps a domain error onto a status code. Unknown errors become a 500 with the
/// error text; nothing here leaks a stack trace.
fn error_response(error: &AppError) -> Response {
    let status = match error {
        AppError::Auth(AuthError::InvalidCredentials | AuthError::NotAuthenticated) => 401,
        AppError::Auth(AuthError::UserNotFound { .. }) => 404,
        AppError::Auth(AuthError::PermissionDenied { .. }) => 403,
        AppError::Auth(_) => 400,
        AppError::NotFound { .. } => 404,
        AppError::Validation(_) => 400,
        AppError::Unsupported { .. } => 501,
        _ => 500,
    };

    Response::json(status, &ErrorResponse::new(error.to_string()))
}

/// Accepts connections forever, one thread each.
///
/// Returns only if [`TcpListener::incoming`] itself fails.
pub fn serve(listener: TcpListener, server: Arc<Server>) -> std::io::Result<()> {
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                let server = Arc::clone(&server);
                std::thread::spawn(move || {
                    if let Err(error) = handle_connection(stream, &server) {
                        tracing::debug!("connection finished with an error: {error}");
                    }
                });
            }
            Err(error) => tracing::warn!("accept failed: {error}"),
        }
    }
    Ok(())
}

fn handle_connection(stream: std::net::TcpStream, server: &Server) -> std::io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;

    // The reader borrows `stream` immutably; it is dropped before the response
    // is written, which needs `&mut stream`.
    let request = {
        let mut reader = BufReader::new(&stream);
        http::read_request(&mut reader)?
    };

    let Some(request) = request else {
        return Ok(());
    };

    let mut stream = stream;
    server.handle(&request).write_to(&mut stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::config::ConfigBuilder;
    use shared::services::factory;

    fn test_server() -> Server {
        let config = Arc::new(ConfigBuilder::new().build());
        let services = block_on(factory::create_services(config)).unwrap();
        Server::new(Arc::new(services))
    }

    fn request(method: &str, path: &str, body: &[u8]) -> Request {
        Request {
            method: method.to_string(),
            path: path.to_string(),
            query: Default::default(),
            body: body.to_vec(),
        }
    }

    fn parse<T: serde::de::DeserializeOwned>(response: &Response) -> T {
        serde_json::from_slice(&response.body).expect("a JSON body")
    }

    #[test]
    fn health_reports_ok() {
        let server = test_server();
        let response = server.handle(&Request::get(endpoints::HEALTH));
        assert_eq!(response.status, 200);

        let health: HealthResponse = parse(&response);
        assert_eq!(health.status, "ok");
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn registering_then_listing_returns_the_user() {
        let server = test_server();
        let body = serde_json::to_vec(&RegisterRequest {
            email: "ada@example.com".to_string(),
            username: "ada".to_string(),
            password: "correct horse".to_string(),
            display_name: "Ada Lovelace".to_string(),
        })
        .unwrap();

        let created = server.handle(&request("POST", endpoints::USERS, &body));
        assert_eq!(created.status, 201);

        let listed = server.handle(&Request::get(endpoints::USERS));
        assert_eq!(listed.status, 200);
        let list: UserList = parse(&listed);
        assert_eq!(list.total, 1);
        assert_eq!(list.users[0].username, "ada");
    }

    #[test]
    fn a_duplicate_registration_is_a_conflict() {
        let server = test_server();
        block_on(
            server
                .services()
                .auth
                .register("ada@example.com", "ada", "correct horse", "Ada"),
        )
        .unwrap();

        let body = serde_json::to_vec(&RegisterRequest {
            email: "ada@example.com".to_string(),
            username: "ada".to_string(),
            password: "correct horse".to_string(),
            display_name: "Ada".to_string(),
        })
        .unwrap();
        let response = server.handle(&request("POST", endpoints::USERS, &body));
        assert!(
            response.status == 400 || response.status == 409,
            "{}",
            response.status
        );
    }

    #[test]
    fn search_without_a_query_is_a_bad_request() {
        let server = test_server();
        let response = server.handle(&Request::get(endpoints::USERS_SEARCH));
        assert_eq!(response.status, 400);
    }

    #[test]
    fn search_finds_a_registered_user() {
        let server = test_server();
        block_on(server.services().auth.register(
            "ada@example.com",
            "ada",
            "correct horse",
            "Ada Lovelace",
        ))
        .unwrap();

        let mut request = Request::get(endpoints::USERS_SEARCH);
        request
            .query
            .insert("q".to_string(), "lovelace".to_string());
        let response = server.handle(&request);
        assert_eq!(response.status, 200);
        let list: UserList = parse(&response);
        assert_eq!(list.total, 1);
    }

    #[test]
    fn an_unknown_path_is_a_not_found() {
        let server = test_server();
        let response = server.handle(&Request::get("/api/nope"));
        assert_eq!(response.status, 404);
    }

    #[test]
    fn a_wrong_method_is_method_not_allowed() {
        let server = test_server();
        let response = server.handle(&request("DELETE", endpoints::USERS, &[]));
        assert_eq!(response.status, 405);
        assert!(response.headers.iter().any(|(name, _)| name == "Allow"));
    }

    #[test]
    fn a_malformed_body_is_a_bad_request() {
        let server = test_server();
        let response = server.handle(&request("POST", endpoints::USERS, b"{ not json"));
        assert_eq!(response.status, 400);
    }

    #[test]
    fn a_real_socket_round_trip_works() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = Arc::new(test_server());

        let worker = {
            let server = Arc::clone(&server);
            std::thread::spawn(move || {
                if let Ok((stream, _)) = listener.accept() {
                    let _ = handle_connection(stream, &server);
                }
            })
        };

        let client = shared::HttpNetworkService::new(
            format!("http://{address}"),
            std::time::Duration::from_secs(5),
        );
        let health: HealthResponse =
            serde_json::from_value(client.get_json(endpoints::HEALTH).unwrap()).unwrap();
        assert_eq!(health.status, "ok");

        worker.join().unwrap();
    }
}
