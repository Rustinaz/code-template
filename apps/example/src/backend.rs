//! The "Backend" panel: the client half of the bundled API.
//!
//! It talks to `apps/server` through [`shared::HttpNetworkService`], which is a
//! blocking plain-HTTP client. egui's `update` runs on the UI thread, so every
//! request is handed to a worker thread and the result arrives back over a
//! channel; the panel never blocks a frame on the network.
//!
//! A browser tab has no raw sockets, so the web build renders an explanatory
//! panel instead of pretending to connect.

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::fmt::Write as _;
    use std::sync::mpsc::{self, Receiver, TryRecvError};
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use eframe::egui;
    use shared::api::{endpoints, HealthResponse, RegisterRequest, UserList};
    use shared::{HttpNetworkService, User};

    /// Rows shown from a listing before it is summarised.
    const MAX_ROWS: usize = 8;

    /// Password the "add demo user" button uses. Matches the server's seed.
    const DEMO_PASSWORD: &str = "password123";

    /// The native panel state.
    pub struct BackendPanel {
        service: Arc<HttpNetworkService>,
        search: String,
        busy: bool,
        receiver: Option<Receiver<Result<String, String>>>,
        result: String,
        error: Option<String>,
        created: u32,
    }

    impl BackendPanel {
        /// Builds a panel that will talk to `base_url`.
        pub fn new(base_url: String, timeout: Duration) -> Self {
            Self {
                service: Arc::new(HttpNetworkService::new(base_url, timeout)),
                search: "lovelace".to_string(),
                busy: false,
                receiver: None,
                result: String::new(),
                error: None,
                created: 0,
            }
        }

        /// Draws the panel, first collecting any result that arrived.
        pub fn show(&mut self, ui: &mut egui::Ui) {
            self.poll();

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label("Backend");
                ui.separator();
                ui.label(format!("Server: {}", self.service.base_url()));

                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("🩺 Health"))
                        .clicked()
                    {
                        self.spawn(|service| {
                            let value = service.get_json(endpoints::HEALTH).map_err(to_text)?;
                            let health: HealthResponse =
                                serde_json::from_value(value).map_err(to_text)?;
                            Ok(format!(
                                "{} · version {} · up {}s",
                                health.status, health.version, health.uptime_seconds
                            ))
                        });
                    }
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("👥 List users"))
                        .clicked()
                    {
                        self.spawn(|service| {
                            let value = service.get_json(endpoints::USERS).map_err(to_text)?;
                            let list: UserList = serde_json::from_value(value).map_err(to_text)?;
                            Ok(format_users(&list))
                        });
                    }
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("➕ Add demo user"))
                        .clicked()
                    {
                        self.created += 1;
                        let unique = format!("{}{}", nanos(), self.created);
                        self.spawn(move |service| {
                            let request = RegisterRequest {
                                email: format!("demo{unique}@example.com"),
                                username: format!("demo{unique}"),
                                password: DEMO_PASSWORD.to_string(),
                                display_name: "Demo User".to_string(),
                            };
                            let body = serde_json::to_value(request).map_err(to_text)?;
                            let value = service
                                .post_json(endpoints::USERS, &body)
                                .map_err(to_text)?;
                            let user: User = serde_json::from_value(value).map_err(to_text)?;
                            Ok(format!("created {}", user.username))
                        });
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add_enabled(
                        !self.busy,
                        egui::TextEdit::singleline(&mut self.search).desired_width(160.0),
                    );
                    let go = ui
                        .add_enabled(
                            !self.busy && !self.search.trim().is_empty(),
                            egui::Button::new("🔎 Search"),
                        )
                        .clicked();
                    if go {
                        let query = self.search.clone();
                        self.spawn(move |service| {
                            let value = service
                                .get_json(&endpoints::users_search(&query))
                                .map_err(to_text)?;
                            let list: UserList = serde_json::from_value(value).map_err(to_text)?;
                            Ok(format_users(&list))
                        });
                    }
                });

                if self.busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("requesting…");
                    });
                    // Keep repainting until the worker answers.
                    ui.ctx().request_repaint();
                }

                if let Some(error) = &self.error {
                    ui.colored_label(egui::Color32::from_rgb(0xD3, 0x2F, 0x2F), error);
                }
                if !self.result.is_empty() {
                    ui.separator();
                    ui.label(&self.result);
                }
            });
        }

        /// Starts one request on a worker thread.
        fn spawn(
            &mut self,
            job: impl FnOnce(&HttpNetworkService) -> Result<String, String> + Send + 'static,
        ) {
            let (sender, receiver) = mpsc::channel();
            let service = Arc::clone(&self.service);
            std::thread::spawn(move || {
                let _ = sender.send(job(&service));
            });
            self.receiver = Some(receiver);
            self.busy = true;
            self.error = None;
        }

        /// Collects a finished request, if there is one.
        fn poll(&mut self) {
            let Some(receiver) = self.receiver.take() else {
                return;
            };

            match receiver.try_recv() {
                Ok(Ok(text)) => {
                    self.result = text;
                    self.busy = false;
                }
                Ok(Err(error)) => {
                    self.error = Some(error);
                    self.busy = false;
                }
                Err(TryRecvError::Empty) => self.receiver = Some(receiver),
                Err(TryRecvError::Disconnected) => {
                    self.error = Some("the request thread stopped unexpectedly".to_string());
                    self.busy = false;
                }
            }
        }
    }

    /// Renders a page of users as a short, readable block.
    fn format_users(list: &UserList) -> String {
        if list.users.is_empty() {
            return "no users matched".to_string();
        }

        let mut out = format!("{} user(s):", list.total);
        for user in list.users.iter().take(MAX_ROWS) {
            let _ = write!(out, "\n• {} <{}>", user.display_name, user.email);
        }
        if list.users.len() > MAX_ROWS {
            let _ = write!(out, "\n… and {} more", list.users.len() - MAX_ROWS);
        }
        out
    }

    fn nanos() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    }

    fn to_text(error: impl std::fmt::Display) -> String {
        error.to_string()
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::egui;

    /// The web placeholder. Kept deliberately tiny.
    pub struct BackendPanel;

    impl BackendPanel {
        /// Accepts the same arguments as the native panel so the app does not
        /// need a `cfg` at the call site.
        pub fn new(_base_url: String, _timeout: std::time::Duration) -> Self {
            Self
        }

        /// Explains, rather than fakes, the missing capability.
        pub fn show(&mut self, ui: &mut egui::Ui) {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label("Backend");
                ui.separator();
                ui.label("Native only in this template: a browser tab has no raw sockets.");
                ui.label(
                    "The web build would call the same API with `fetch` — the routing and the \
                     JSON types in `shared::api` are already shared.",
                );
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::BackendPanel;

#[cfg(target_arch = "wasm32")]
pub use web::BackendPanel;
