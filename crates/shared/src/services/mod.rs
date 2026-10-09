//! Core services for business logic
//!
//! Services coordinate between repositories, external APIs, and use cases.
//! They contain the core business logic and are platform-agnostic.

#[cfg(feature = "network")]
use crate::config::AppConfig;
use crate::domain::{
    AccessibilitySettings, AppSettings, DeviceInfo, EntityId, Platform, RepositoryEntity,
    ThemePreference, Timestamp, User, UserPreferences, WindowState,
};
use crate::errors::{AppError, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

/// A blocking, TLS-free HTTP client for talking to the bundled backend.
///
/// Gated off for `wasm32`, which has no raw sockets.
#[cfg(not(target_arch = "wasm32"))]
pub mod http;

/// Simple in-memory repository for testing and offline use.
///
/// Bounded on [`RepositoryEntity`] rather than [`Entity`](crate::domain::Entity)
/// because it maintains secondary indexes. Text keys are lowercased on the way
/// in, so a lookup finds an entity no matter how it was capitalised.
pub struct InMemoryRepository<T: RepositoryEntity> {
    data: Arc<RwLock<HashMap<EntityId, T>>>,
    by_email: Arc<RwLock<HashMap<String, EntityId>>>,
    by_username: Arc<RwLock<HashMap<String, EntityId>>>,
    by_owner: Arc<RwLock<HashMap<EntityId, HashMap<EntityId, ()>>>>,
}

impl<T: RepositoryEntity + Clone> InMemoryRepository<T> {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            by_email: Arc::new(RwLock::new(HashMap::new())),
            by_username: Arc::new(RwLock::new(HashMap::new())),
            by_owner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Inserts or replaces an entity, keeping the secondary indexes in step.
    ///
    /// The previous version is dropped from the indexes first, so changing an
    /// email does not leave a stale key pointing at the entity.
    pub async fn save(&self, entity: &T) -> Result<()> {
        let id = entity.id();
        self.unindex(id, self.data.read().get(&id));

        if let Some(email) = entity.email() {
            self.by_email.write().insert(email.to_lowercase(), id);
        }
        if let Some(username) = entity.username() {
            self.by_username.write().insert(username.to_lowercase(), id);
        }
        if let Some(owner) = entity.owner_id() {
            self.by_owner
                .write()
                .entry(owner)
                .or_default()
                .insert(id, ());
        }

        self.data.write().insert(id, entity.clone());
        Ok(())
    }

    /// Drops the id-to-entity mapping and any index entry pointing at it.
    pub async fn delete(&self, id: EntityId) -> Result<()> {
        let previous = self.data.read().get(&id).cloned();
        self.unindex(id, previous.as_ref());
        self.data.write().remove(&id);
        Ok(())
    }

    pub async fn find_by_id(&self, id: EntityId) -> Result<Option<T>> {
        Ok(self.data.read().get(&id).cloned())
    }

    /// Looks an entity up by email, case-insensitively.
    ///
    /// Returns `None` for a miss, and also for an entity type that has no email
    /// index at all.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<T>> {
        let id = self.by_email.read().get(&email.to_lowercase()).copied();
        Ok(id.and_then(|id| self.data.read().get(&id).cloned()))
    }

    /// Looks an entity up by username, case-insensitively.
    pub async fn find_by_username(&self, username: &str) -> Result<Option<T>> {
        let id = self
            .by_username
            .read()
            .get(&username.to_lowercase())
            .copied();
        Ok(id.and_then(|id| self.data.read().get(&id).cloned()))
    }

    /// Every entity belonging to `owner`, oldest insertion order not guaranteed.
    pub async fn find_by_owner(&self, owner: EntityId) -> Result<Vec<T>> {
        let ids: Vec<EntityId> = self
            .by_owner
            .read()
            .get(&owner)
            .map(|set| set.keys().copied().collect())
            .unwrap_or_default();
        let data = self.data.read();
        Ok(ids
            .into_iter()
            .filter_map(|id| data.get(&id).cloned())
            .collect())
    }

    /// Every stored entity matching `predicate`, capped at `limit`.
    ///
    /// A linear scan, which is the right shape for an in-memory store this size
    /// and the only way to match on fields that are not indexed. A real
    /// repository would push this down to a query instead of a full scan.
    pub fn filter(&self, limit: usize, predicate: impl Fn(&T) -> bool) -> Vec<T> {
        let data = self.data.read();
        data.values()
            .filter(|entity| predicate(entity))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Case-insensitive substring match over email and username, capped at `limit`.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<T>> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }

        Ok(self.filter(limit, |entity| {
            entity
                .email()
                .is_some_and(|v| v.to_lowercase().contains(&needle))
                || entity
                    .username()
                    .is_some_and(|v| v.to_lowercase().contains(&needle))
        }))
    }

    /// Number of stored entities. Useful in tests and for an offline cache.
    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    /// Whether the repository holds nothing.
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }

    /// Drops `id` from every secondary index, using `previous` to find the keys.
    ///
    /// The `== Some(&id)` guard matters: two entities can race to claim the same
    /// email, and the loser's index entry must not be stolen.
    fn unindex(&self, id: EntityId, previous: Option<&T>) {
        let Some(previous) = previous else {
            return;
        };

        if let Some(email) = previous.email() {
            let mut by_email = self.by_email.write();
            if by_email.get(&email.to_lowercase()) == Some(&id) {
                by_email.remove(&email.to_lowercase());
            }
        }

        if let Some(username) = previous.username() {
            let mut by_username = self.by_username.write();
            if by_username.get(&username.to_lowercase()) == Some(&id) {
                by_username.remove(&username.to_lowercase());
            }
        }

        if let Some(owner) = previous.owner_id() {
            if let Some(set) = self.by_owner.write().get_mut(&owner) {
                set.remove(&id);
            }
        }
    }
}

impl<T: RepositoryEntity + Clone> Default for InMemoryRepository<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: RepositoryEntity + Clone> AsAny for InMemoryRepository<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

/// Authentication service
#[async_trait]
pub trait AuthService: Send + Sync {
    async fn login(&self, email: &str, password: &str) -> Result<User>;
    async fn register(
        &self,
        email: &str,
        username: &str,
        password: &str,
        display_name: &str,
    ) -> Result<User>;
    async fn logout(&self) -> Result<()>;
    async fn refresh_token(&self) -> Result<String>;
    async fn current_user(&self) -> Result<Option<User>>;
    async fn change_password(&self, old_password: &str, new_password: &str) -> Result<()>;
    async fn request_password_reset(&self, email: &str) -> Result<()>;
    async fn reset_password(&self, token: &str, new_password: &str) -> Result<()>;
}

/// User service for profile management
#[async_trait]
pub trait UserService: Send + Sync {
    async fn get_profile(&self, user_id: EntityId) -> Result<User>;
    async fn update_profile(
        &self,
        user_id: EntityId,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<User>;
    async fn update_preferences(
        &self,
        user_id: EntityId,
        preferences: UserPreferences,
    ) -> Result<User>;
    async fn delete_account(&self, user_id: EntityId) -> Result<()>;
    async fn search_users(&self, query: &str, limit: usize) -> Result<Vec<User>>;

    /// Every user, capped at `limit`, in arbitrary order.
    ///
    /// Unlike [`search_users`](Self::search_users), an empty call is meaningful
    /// here: it lists rather than returning nothing, which is what the backend's
    /// `GET /api/users` needs.
    async fn list_users(&self, limit: usize) -> Result<Vec<User>>;
}

/// Settings service
#[async_trait]
pub trait SettingsService: Send + Sync {
    async fn get_settings(&self, user_id: EntityId) -> Result<AppSettings>;
    async fn update_theme(&self, user_id: EntityId, theme: ThemePreference) -> Result<()>;
    async fn update_font_scale(&self, user_id: EntityId, scale: f32) -> Result<()>;
    async fn update_ui_scale(&self, user_id: EntityId, scale: f32) -> Result<()>;
    async fn update_accessibility(
        &self,
        user_id: EntityId,
        settings: AccessibilitySettings,
    ) -> Result<()>;
    async fn update_language(&self, user_id: EntityId, language: &str) -> Result<()>;
    async fn reset_to_defaults(&self, user_id: EntityId) -> Result<()>;
}

/// Network service for API communication
#[async_trait]
pub trait NetworkService: Send + Sync {
    async fn get(&self, endpoint: &str) -> Result<serde_json::Value>;
    async fn post(&self, endpoint: &str, body: &serde_json::Value) -> Result<serde_json::Value>;
    async fn put(&self, endpoint: &str, body: &serde_json::Value) -> Result<serde_json::Value>;
    async fn delete(&self, endpoint: &str) -> Result<()>;
    async fn upload(
        &self,
        endpoint: &str,
        file_path: &std::path::Path,
        mime_type: &str,
    ) -> Result<String>;
    async fn download(&self, url: &str, destination: &std::path::Path) -> Result<()>;
}

/// Storage service for local data
#[async_trait]
pub trait StorageService: Send + Sync {
    async fn save_string(&self, key: &str, value: &str) -> Result<()>;
    async fn get_string(&self, key: &str) -> Result<Option<String>>;
    async fn save_bytes(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn clear(&self) -> Result<()>;
    async fn get_size(&self) -> Result<u64>;
}

/// Platform service for platform-specific operations
#[async_trait]
pub trait PlatformService: Send + Sync {
    fn platform(&self) -> Platform;
    async fn open_url(&self, url: &str) -> Result<()>;
    async fn share_text(&self, text: &str, title: Option<&str>) -> Result<()>;
    async fn share_file(
        &self,
        file_path: &std::path::Path,
        mime_type: &str,
        title: Option<&str>,
    ) -> Result<()>;
    async fn get_device_info(&self) -> Result<DeviceInfo>;
    async fn request_permission(&self, permission: &str) -> Result<bool>;
    async fn check_permission(&self, permission: &str) -> Result<bool>;
    async fn vibrate(&self, duration_ms: u64) -> Result<()>;
    async fn set_keep_screen_on(&self, keep_on: bool) -> Result<()>;
    async fn get_battery_level(&self) -> Result<Option<f32>>;
    async fn is_charging(&self) -> Result<Option<bool>>;
}

/// Analytics service
#[async_trait]
pub trait AnalyticsService: Send + Sync {
    async fn track_event(&self, event: &str, properties: Option<serde_json::Value>) -> Result<()>;
    async fn track_screen(&self, screen_name: &str) -> Result<()>;
    async fn set_user_property(&self, key: &str, value: &str) -> Result<()>;
    async fn identify_user(&self, user_id: &str, traits: Option<serde_json::Value>) -> Result<()>;
    async fn reset(&self) -> Result<()>;
    fn is_enabled(&self) -> bool;
}

/// Notification service
#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn show_local_notification(
        &self,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<()>;
    async fn schedule_notification(
        &self,
        title: &str,
        body: &str,
        at: Timestamp,
        data: Option<serde_json::Value>,
    ) -> Result<String>;
    async fn cancel_notification(&self, id: &str) -> Result<()>;
    async fn cancel_all_notifications(&self) -> Result<()>;
    async fn request_permission(&self) -> Result<bool>;
}

/// Service container for dependency injection
pub struct ServiceContainer {
    pub auth: Arc<dyn AuthService>,
    pub users: Arc<dyn UserService>,
    pub settings: Arc<dyn SettingsService>,
    pub network: Arc<dyn NetworkService>,
    pub storage: Arc<dyn StorageService>,
    pub platform: Arc<dyn PlatformService>,
    pub analytics: Arc<dyn AnalyticsService>,
    pub notifications: Arc<dyn NotificationService>,
}

impl ServiceContainer {
    /// Builds the container from the app-facing services and the ambient ones.
    ///
    /// Split into two groups rather than eight positional arguments so that
    /// adding a service cannot silently reorder the others at a call site.
    pub fn new(app: AppServices, platform: PlatformServices) -> Self {
        Self {
            auth: app.auth,
            users: app.users,
            settings: app.settings,
            network: app.network,
            storage: app.storage,
            platform: platform.platform,
            analytics: platform.analytics,
            notifications: platform.notifications,
        }
    }
}

/// The services that implement the app's own behaviour.
#[derive(Clone)]
pub struct AppServices {
    pub auth: Arc<dyn AuthService>,
    pub users: Arc<dyn UserService>,
    pub settings: Arc<dyn SettingsService>,
    pub network: Arc<dyn NetworkService>,
    pub storage: Arc<dyn StorageService>,
}

/// The services that reach outside the app: the host, analytics, notifications.
#[derive(Clone)]
pub struct PlatformServices {
    pub platform: Arc<dyn PlatformService>,
    pub analytics: Arc<dyn AnalyticsService>,
    pub notifications: Arc<dyn NotificationService>,
}

/// Default implementations using in-memory storage (for testing/offline)
pub mod defaults {
    use super::*;
    use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
    use argon2::Argon2;
    use password_hash::rand_core::OsRng;
    // `SystemTime::now()` aborts on `wasm32-unknown-unknown`, so the whole
    // clock -- including the epoch it is measured against -- comes from
    // `web_time`, which re-exports `std::time` on every other target.
    use web_time::{SystemTime, UNIX_EPOCH};

    /// Re-exported so callers find it next to the other default services.
    /// Implemented in [`crate::services::http`] because it does real I/O and
    /// needs socket-level tests.
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::services::http::HttpNetworkService;

    /// Shortest password `register` and `change_password` will accept.
    pub const MIN_PASSWORD_LEN: usize = 8;

    /// In-memory credentials, keyed by user id.
    ///
    /// Kept apart from [`User`] on purpose: the user record is the profile, and
    /// a password hash has no business being serialised alongside a display name
    /// and an avatar URL. In a real app this is a `credentials` table you never
    /// select into a user query.
    #[derive(Default)]
    pub struct CredentialStore {
        hashes: Arc<RwLock<HashMap<EntityId, String>>>,
    }

    impl CredentialStore {
        pub fn new() -> Self {
            Self::default()
        }

        /// Hashes `password` and stores it for `user_id`.
        pub fn set(&self, user_id: EntityId, password: &str) -> Result<()> {
            let hash = hash_password(password)?;
            self.hashes.write().insert(user_id, hash);
            Ok(())
        }

        /// Whether `password` matches the stored hash for `user_id`.
        ///
        /// A user with no stored hash returns `false` rather than succeeding, so
        /// a half-registered account can never be logged into.
        pub fn verify(&self, user_id: EntityId, password: &str) -> Result<bool> {
            let Some(hash) = self.hashes.read().get(&user_id).cloned() else {
                return Ok(false);
            };
            Ok(verify_password(password, &hash))
        }

        /// Drops the stored hash, e.g. on account deletion.
        pub fn remove(&self, user_id: EntityId) {
            self.hashes.write().remove(&user_id);
        }
    }

    /// Hashes a password with a fresh random salt.
    fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| AppError::Internal(format!("password hashing failed: {error}")))
    }

    /// Checks a password against a stored PHC hash.
    ///
    /// A malformed stored hash verifies as `false` rather than erroring, so a
    /// corrupted row locks the account instead of crashing the login path.
    fn verify_password(password: &str, hash: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(hash) else {
            warn!("stored password hash is malformed; refusing the login");
            return false;
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    }

    pub struct DefaultAuthService {
        users: Arc<InMemoryRepository<User>>,
        credentials: Arc<CredentialStore>,
        current_user: Arc<RwLock<Option<EntityId>>>,
    }

    impl DefaultAuthService {
        /// Builds a service with its own credential store.
        pub fn new(users: Arc<InMemoryRepository<User>>) -> Self {
            Self::with_credentials(users, Arc::new(CredentialStore::new()))
        }

        /// Builds a service sharing an existing credential store.
        ///
        /// Use this when another service, such as the user service, also has to
        /// reach the credentials — deleting an account has to remove its hash,
        /// and a store private to auth would leave that hash behind.
        pub fn with_credentials(
            users: Arc<InMemoryRepository<User>>,
            credentials: Arc<CredentialStore>,
        ) -> Self {
            Self {
                users,
                credentials,
                current_user: Arc::new(RwLock::new(None)),
            }
        }

        /// The credential store, exposed so an app can seed or clear accounts.
        pub fn credentials(&self) -> &Arc<CredentialStore> {
            &self.credentials
        }

        /// Replaces a user's password after checking the current one.
        async fn set_password(
            &self,
            user_id: EntityId,
            old_password: &str,
            new_password: &str,
        ) -> Result<()> {
            if !self.credentials.verify(user_id, old_password)? {
                return Err(AppError::Auth(crate::errors::AuthError::InvalidCredentials));
            }
            if new_password.len() < MIN_PASSWORD_LEN {
                return Err(AppError::Validation(
                    crate::errors::ValidationError::TooShort {
                        field: "password".to_string(),
                        min: MIN_PASSWORD_LEN,
                    },
                ));
            }
            self.credentials.set(user_id, new_password)
        }
    }

    #[async_trait]
    impl AuthService for DefaultAuthService {
        async fn login(&self, email: &str, password: &str) -> Result<User> {
            let user = self
                .users
                .find_by_email(email)
                .await?
                .ok_or_else(|| AppError::Auth(crate::errors::AuthError::InvalidCredentials))?;

            // Verified before the session is set, so a wrong password leaves the
            // service logged out.
            if !self.credentials.verify(user.id, password)? {
                return Err(AppError::Auth(crate::errors::AuthError::InvalidCredentials));
            }

            *self.current_user.write() = Some(user.id);
            Ok(user)
        }

        async fn register(
            &self,
            email: &str,
            username: &str,
            password: &str,
            display_name: &str,
        ) -> Result<User> {
            if password.len() < MIN_PASSWORD_LEN {
                return Err(AppError::Validation(
                    crate::errors::ValidationError::TooShort {
                        field: "password".to_string(),
                        min: MIN_PASSWORD_LEN,
                    },
                ));
            }
            if self.users.find_by_email(email).await?.is_some() {
                return Err(AppError::Validation(
                    crate::errors::ValidationError::Duplicate {
                        field: "email".to_string(),
                    },
                ));
            }
            if self.users.find_by_username(username).await?.is_some() {
                return Err(AppError::Validation(
                    crate::errors::ValidationError::Duplicate {
                        field: "username".to_string(),
                    },
                ));
            }

            let user = User {
                id: Uuid::new_v4(),
                username: username.to_string(),
                email: email.to_string(),
                display_name: display_name.to_string(),
                avatar_url: None,
                preferences: UserPreferences::default(),
                created_at: Timestamp::default(),
                updated_at: Timestamp::default(),
            };

            self.users.save(&user).await?;
            self.credentials.set(user.id, password)?;
            *self.current_user.write() = Some(user.id);
            Ok(user)
        }

        async fn logout(&self) -> Result<()> {
            *self.current_user.write() = None;
            Ok(())
        }

        async fn refresh_token(&self) -> Result<String> {
            Ok(format!(
                "token_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ))
        }

        async fn current_user(&self) -> Result<Option<User>> {
            let current = *self.current_user.read();
            if let Some(id) = current {
                Ok(self.users.find_by_id(id).await?)
            } else {
                Ok(None)
            }
        }

        async fn change_password(&self, old: &str, new: &str) -> Result<()> {
            let Some(id) = *self.current_user.read() else {
                return Err(AppError::Auth(crate::errors::AuthError::NotAuthenticated));
            };
            self.set_password(id, old, new).await
        }

        async fn request_password_reset(&self, email: &str) -> Result<()> {
            let _ = self.users.find_by_email(email).await?.ok_or_else(|| {
                AppError::Auth(crate::errors::AuthError::UserNotFound {
                    identifier: email.to_string(),
                })
            })?;
            Ok(())
        }

        /// Sets a new password without checking the old one.
        ///
        /// In a real app this is only reachable after a single-use reset token has
        /// been redeemed, and the token is checked before this runs. With no token
        /// store to check against, it refuses rather than granting a password
        /// reset to anyone who calls it.
        async fn reset_password(&self, _token: &str, _new_password: &str) -> Result<()> {
            Err(AppError::Unsupported {
                operation: "reset_password".to_string(),
                platform: "in-memory".to_string(),
            })
        }
    }

    pub struct DefaultUserService {
        users: Arc<InMemoryRepository<User>>,
        credentials: Arc<CredentialStore>,
    }

    impl DefaultUserService {
        pub fn new(users: Arc<InMemoryRepository<User>>) -> Self {
            Self::with_credentials(users, Arc::new(CredentialStore::new()))
        }

        /// Builds a service that shares the auth service's credential store, so
        /// deleting an account also drops its password hash.
        pub fn with_credentials(
            users: Arc<InMemoryRepository<User>>,
            credentials: Arc<CredentialStore>,
        ) -> Self {
            Self { users, credentials }
        }
    }

    #[async_trait]
    impl UserService for DefaultUserService {
        async fn get_profile(&self, user_id: EntityId) -> Result<User> {
            self.users
                .find_by_id(user_id)
                .await?
                .ok_or_else(|| AppError::NotFound {
                    resource: "User".to_string(),
                    id: user_id.to_string(),
                })
        }

        async fn update_profile(
            &self,
            user_id: EntityId,
            display_name: Option<&str>,
            avatar_url: Option<&str>,
        ) -> Result<User> {
            let mut user = self.get_profile(user_id).await?;
            if let Some(name) = display_name {
                user.display_name = name.to_string();
            }
            if let Some(url) = avatar_url {
                user.avatar_url = Some(url.to_string());
            }
            user.updated_at = Timestamp::default();
            self.users.save(&user).await?;
            Ok(user)
        }

        async fn update_preferences(
            &self,
            user_id: EntityId,
            preferences: UserPreferences,
        ) -> Result<User> {
            let mut user = self.get_profile(user_id).await?;
            user.preferences = preferences;
            user.updated_at = Timestamp::default();
            self.users.save(&user).await?;
            Ok(user)
        }

        async fn delete_account(&self, user_id: EntityId) -> Result<()> {
            self.users.delete(user_id).await?;
            self.credentials.remove(user_id);
            Ok(())
        }

        /// Searches by email, username, or display name.
        ///
        /// This scans rather than using the email and username indexes, because
        /// a display name is not unique and so has no index to consult. Every
        /// term in the query must appear somewhere in the record, so "alicia
        /// wonderland" narrows rather than widening the result.
        async fn search_users(&self, query: &str, limit: usize) -> Result<Vec<User>> {
            let terms: Vec<String> = query
                .split_whitespace()
                .map(|term| term.to_lowercase())
                .filter(|term| !term.is_empty())
                .collect();

            if terms.is_empty() {
                return Ok(Vec::new());
            }

            Ok(self.users.filter(limit, |u| {
                terms.iter().all(|term| {
                    u.username.to_lowercase().contains(term)
                        || u.email.to_lowercase().contains(term)
                        || u.display_name.to_lowercase().contains(term)
                })
            }))
        }

        async fn list_users(&self, limit: usize) -> Result<Vec<User>> {
            Ok(self.users.filter(limit, |_| true))
        }
    }

    /// Settings for one user.
    ///
    /// Theme and language live on [`UserPreferences`], not on [`AppSettings`],
    /// so this holds the user repository too. Splitting them across two
    /// services would mean the two can disagree, and a template that lets a
    /// user's dark-mode choice and their stored settings drift apart is worse
    /// than no template at all.
    pub struct DefaultSettingsService {
        settings: Arc<InMemoryRepository<AppSettings>>,
        users: Arc<InMemoryRepository<User>>,
    }

    impl DefaultSettingsService {
        pub fn new(
            settings: Arc<InMemoryRepository<AppSettings>>,
            users: Arc<InMemoryRepository<User>>,
        ) -> Self {
            Self { settings, users }
        }

        /// Loads the settings row, creating a default one on first use.
        ///
        /// Creating rather than erroring means a fresh account works without a
        /// separate provisioning step; callers never see a missing-settings
        /// failure just because nobody wrote the row yet.
        async fn settings_or_default(&self, user_id: EntityId) -> Result<AppSettings> {
            if let Some(existing) = self
                .settings
                .find_by_owner(user_id)
                .await?
                .into_iter()
                .next()
            {
                return Ok(existing);
            }

            let settings = AppSettings {
                id: Uuid::new_v4(),
                user_id,
                window_state: WindowState {
                    width: 1024,
                    height: 768,
                    maximized: false,
                    fullscreen: false,
                    position: None,
                },
                ui_scale: 1.0,
                font_scale: 1.0,
                reduced_motion: false,
                accessibility: AccessibilitySettings::default(),
                created_at: Timestamp::default(),
                updated_at: Timestamp::default(),
            };
            self.settings.save(&settings).await?;
            Ok(settings)
        }

        /// Applies `edit` to the user's settings and persists the result.
        async fn mutate(
            &self,
            user_id: EntityId,
            edit: impl FnOnce(&mut AppSettings),
        ) -> Result<()> {
            let mut settings = self.settings_or_default(user_id).await?;
            edit(&mut settings);
            settings.updated_at = Timestamp::default();
            self.settings.save(&settings).await
        }

        /// Applies `edit` to the user's preferences and persists the result.
        async fn mutate_preferences(
            &self,
            user_id: EntityId,
            edit: impl FnOnce(&mut UserPreferences),
        ) -> Result<()> {
            let mut user =
                self.users
                    .find_by_id(user_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound {
                        resource: "User".to_string(),
                        id: user_id.to_string(),
                    })?;
            edit(&mut user.preferences);
            user.updated_at = Timestamp::default();
            self.users.save(&user).await
        }
    }

    #[async_trait]
    impl SettingsService for DefaultSettingsService {
        async fn get_settings(&self, user_id: EntityId) -> Result<AppSettings> {
            self.settings_or_default(user_id).await
        }

        async fn update_theme(&self, user_id: EntityId, theme: ThemePreference) -> Result<()> {
            self.mutate_preferences(user_id, |prefs| prefs.theme = theme)
                .await
        }

        async fn update_font_scale(&self, user_id: EntityId, scale: f32) -> Result<()> {
            self.mutate(user_id, |s| s.font_scale = scale.clamp(0.5, 3.0))
                .await
        }

        async fn update_ui_scale(&self, user_id: EntityId, scale: f32) -> Result<()> {
            self.mutate(user_id, |s| s.ui_scale = scale.clamp(0.5, 3.0))
                .await
        }

        async fn update_accessibility(
            &self,
            user_id: EntityId,
            accessibility: AccessibilitySettings,
        ) -> Result<()> {
            self.mutate(user_id, |s| s.accessibility = accessibility)
                .await
        }

        async fn update_language(&self, user_id: EntityId, language: &str) -> Result<()> {
            self.mutate_preferences(user_id, |prefs| {
                prefs.language = language.to_string();
            })
            .await
        }

        /// Resets what this service owns, and nothing else.
        ///
        /// Notification and sync flags live in `UserPreferences` too, but they
        /// are the user's explicit choices rather than app defaults, so a
        /// settings reset leaves them alone.
        async fn reset_to_defaults(&self, user_id: EntityId) -> Result<()> {
            self.mutate(user_id, |s| {
                s.font_scale = 1.0;
                s.ui_scale = 1.0;
                s.reduced_motion = false;
                s.accessibility = AccessibilitySettings::default();
            })
            .await?;

            self.mutate_preferences(user_id, |prefs| {
                prefs.theme = ThemePreference::System;
                prefs.language = "en".to_string();
            })
            .await
        }
    }

    /// The `reqwest`-backed [`NetworkService`].
    ///
    /// Only compiled with the `network` feature. `reqwest` is a native HTTP
    /// client: it needs sockets, TLS and blocking thread pools, none of which
    /// exist in a browser tab. A web build gets [`StubNetworkService`] instead.
    #[cfg(feature = "network")]
    pub struct DefaultNetworkService {
        config: Arc<AppConfig>,
        client: reqwest::Client,
    }

    #[cfg(feature = "network")]
    impl DefaultNetworkService {
        pub fn new(config: Arc<AppConfig>) -> Self {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(config.network.timeout_ms))
                .danger_accept_invalid_certs(!config.network.tls_verification)
                .build()
                .unwrap();
            Self { config, client }
        }
    }

    #[async_trait]
    #[cfg(feature = "network")]
    impl NetworkService for DefaultNetworkService {
        async fn get(&self, endpoint: &str) -> Result<serde_json::Value> {
            let url = format!("{}{}", self.config.network.base_url, endpoint);
            let response = self.client.get(&url).send().await?;
            let status = response.status().as_u16();
            if !(200..=299).contains(&status) {
                return Err(AppError::Network(crate::errors::NetworkError::HttpError {
                    status,
                    message: response.text().await.unwrap_or_default(),
                }));
            }
            Ok(response.json().await?)
        }

        async fn post(
            &self,
            endpoint: &str,
            body: &serde_json::Value,
        ) -> Result<serde_json::Value> {
            let url = format!("{}{}", self.config.network.base_url, endpoint);
            let response = self.client.post(&url).json(body).send().await?;
            let status = response.status().as_u16();
            if !(200..=299).contains(&status) {
                return Err(AppError::Network(crate::errors::NetworkError::HttpError {
                    status,
                    message: response.text().await.unwrap_or_default(),
                }));
            }
            Ok(response.json().await?)
        }

        async fn put(&self, endpoint: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
            let url = format!("{}{}", self.config.network.base_url, endpoint);
            let response = self.client.put(&url).json(body).send().await?;
            let status = response.status().as_u16();
            if !(200..=299).contains(&status) {
                return Err(AppError::Network(crate::errors::NetworkError::HttpError {
                    status,
                    message: response.text().await.unwrap_or_default(),
                }));
            }
            Ok(response.json().await?)
        }

        async fn delete(&self, endpoint: &str) -> Result<()> {
            let url = format!("{}{}", self.config.network.base_url, endpoint);
            let response = self.client.delete(&url).send().await?;
            let status = response.status().as_u16();
            if !(200..=299).contains(&status) {
                return Err(AppError::Network(crate::errors::NetworkError::HttpError {
                    status,
                    message: response.text().await.unwrap_or_default(),
                }));
            }
            Ok(())
        }

        async fn upload(
            &self,
            _endpoint: &str,
            _file_path: &std::path::Path,
            _mime_type: &str,
        ) -> Result<String> {
            Err(AppError::Unsupported {
                operation: "upload".to_string(),
                platform: "default".to_string(),
            })
        }

        async fn download(&self, _url: &str, _destination: &std::path::Path) -> Result<()> {
            Err(AppError::Unsupported {
                operation: "download".to_string(),
                platform: "default".to_string(),
            })
        }
    }

    /// The [`NetworkService`] used when the `network` feature is off.
    ///
    /// Every call reports [`AppError::Unsupported`] instead of silently
    /// pretending to succeed. A template should not ship a fake HTTP client
    /// that returns made-up data, and a browser tab has no business making
    /// cross-origin requests from a native stack anyway -- wire this to
    /// `fetch` if the web build actually needs networking.
    #[cfg(not(feature = "network"))]
    pub struct StubNetworkService;

    #[cfg(not(feature = "network"))]
    impl StubNetworkService {
        /// Construct the stub. Takes no configuration, since it ignores it.
        pub const fn new() -> Self {
            Self
        }

        fn unsupported(operation: &str) -> AppError {
            AppError::Unsupported {
                operation: operation.to_string(),
                platform: "no-network-feature".to_string(),
            }
        }
    }

    #[cfg(not(feature = "network"))]
    impl Default for StubNetworkService {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(not(feature = "network"))]
    #[async_trait]
    impl NetworkService for StubNetworkService {
        async fn get(&self, _endpoint: &str) -> Result<serde_json::Value> {
            Err(Self::unsupported("get"))
        }

        async fn post(
            &self,
            _endpoint: &str,
            _body: &serde_json::Value,
        ) -> Result<serde_json::Value> {
            Err(Self::unsupported("post"))
        }

        async fn put(
            &self,
            _endpoint: &str,
            _body: &serde_json::Value,
        ) -> Result<serde_json::Value> {
            Err(Self::unsupported("put"))
        }

        async fn delete(&self, _endpoint: &str) -> Result<()> {
            Err(Self::unsupported("delete"))
        }

        async fn upload(
            &self,
            _endpoint: &str,
            _file_path: &std::path::Path,
            _mime_type: &str,
        ) -> Result<String> {
            Err(Self::unsupported("upload"))
        }

        async fn download(&self, _url: &str, _destination: &std::path::Path) -> Result<()> {
            Err(Self::unsupported("download"))
        }
    }

    pub struct DefaultStorageService {
        data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    }

    impl DefaultStorageService {
        pub fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl StorageService for DefaultStorageService {
        async fn save_string(&self, key: &str, value: &str) -> Result<()> {
            self.data
                .write()
                .insert(key.to_string(), value.as_bytes().to_vec());
            Ok(())
        }

        async fn get_string(&self, key: &str) -> Result<Option<String>> {
            Ok(self
                .data
                .read()
                .get(key)
                .and_then(|v| String::from_utf8(v.clone()).ok()))
        }

        async fn save_bytes(&self, key: &str, value: &[u8]) -> Result<()> {
            self.data.write().insert(key.to_string(), value.to_vec());
            Ok(())
        }

        async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>> {
            Ok(self.data.read().get(key).cloned())
        }

        async fn delete(&self, key: &str) -> Result<bool> {
            Ok(self.data.write().remove(key).is_some())
        }

        async fn clear(&self) -> Result<()> {
            self.data.write().clear();
            Ok(())
        }

        async fn get_size(&self) -> Result<u64> {
            let total: usize = self.data.read().values().map(|v| v.len()).sum();
            Ok(total as u64)
        }
    }

    impl Default for DefaultStorageService {
        fn default() -> Self {
            Self::new()
        }
    }

    pub struct DefaultPlatformService {
        platform: Platform,
    }

    impl DefaultPlatformService {
        pub fn new(platform: Platform) -> Self {
            Self { platform }
        }
    }

    #[async_trait]
    impl PlatformService for DefaultPlatformService {
        fn platform(&self) -> Platform {
            self.platform
        }

        async fn open_url(&self, url: &str) -> Result<()> {
            #[cfg(not(target_arch = "wasm32"))]
            {
                opener::open(url)?;
            }
            #[cfg(target_arch = "wasm32")]
            {
                web_sys::window().unwrap().open_with_url(url).map_err(|e| {
                    AppError::Platform(crate::errors::PlatformError::Web {
                        source: anyhow::anyhow!("{:?}", e),
                    })
                })?;
            }
            Ok(())
        }

        async fn share_text(&self, _text: &str, _title: Option<&str>) -> Result<()> {
            Err(AppError::Unsupported {
                operation: "share_text".to_string(),
                platform: format!("{:?}", self.platform),
            })
        }

        async fn share_file(
            &self,
            _file_path: &std::path::Path,
            _mime_type: &str,
            _title: Option<&str>,
        ) -> Result<()> {
            Err(AppError::Unsupported {
                operation: "share_file".to_string(),
                platform: format!("{:?}", self.platform),
            })
        }

        async fn get_device_info(&self) -> Result<DeviceInfo> {
            Ok(DeviceInfo {
                platform: self.platform,
                form_factor: crate::domain::FormFactor::Desktop,
                screen_size: crate::domain::ScreenSize::Large,
                pixel_density: 1.0,
                orientation: crate::domain::Orientation::Landscape,
                has_touch: false,
                has_keyboard: true,
                has_mouse: true,
                safe_area: crate::domain::SafeArea::default(),
            })
        }

        async fn request_permission(&self, _permission: &str) -> Result<bool> {
            Ok(true)
        }

        async fn check_permission(&self, _permission: &str) -> Result<bool> {
            Ok(true)
        }

        async fn vibrate(&self, _duration_ms: u64) -> Result<()> {
            Ok(())
        }

        async fn set_keep_screen_on(&self, _keep_on: bool) -> Result<()> {
            Ok(())
        }

        async fn get_battery_level(&self) -> Result<Option<f32>> {
            Ok(None)
        }

        async fn is_charging(&self) -> Result<Option<bool>> {
            Ok(None)
        }
    }

    pub struct DefaultAnalyticsService {
        enabled: bool,
    }

    impl DefaultAnalyticsService {
        pub fn new(enabled: bool) -> Self {
            Self { enabled }
        }
    }

    #[async_trait]
    impl AnalyticsService for DefaultAnalyticsService {
        async fn track_event(
            &self,
            _event: &str,
            _properties: Option<serde_json::Value>,
        ) -> Result<()> {
            Ok(())
        }

        async fn track_screen(&self, _screen_name: &str) -> Result<()> {
            Ok(())
        }

        async fn set_user_property(&self, _key: &str, _value: &str) -> Result<()> {
            Ok(())
        }

        async fn identify_user(
            &self,
            _user_id: &str,
            _traits: Option<serde_json::Value>,
        ) -> Result<()> {
            Ok(())
        }

        async fn reset(&self) -> Result<()> {
            Ok(())
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }
    }

    pub struct DefaultNotificationService;

    impl DefaultNotificationService {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl NotificationService for DefaultNotificationService {
        async fn show_local_notification(
            &self,
            _title: &str,
            _body: &str,
            _data: Option<serde_json::Value>,
        ) -> Result<()> {
            Ok(())
        }

        async fn schedule_notification(
            &self,
            _title: &str,
            _body: &str,
            _at: Timestamp,
            _data: Option<serde_json::Value>,
        ) -> Result<String> {
            Ok(Uuid::new_v4().to_string())
        }

        async fn cancel_notification(&self, _id: &str) -> Result<()> {
            Ok(())
        }

        async fn cancel_all_notifications(&self) -> Result<()> {
            Ok(())
        }

        async fn request_permission(&self) -> Result<bool> {
            Ok(true)
        }
    }

    impl Default for DefaultNotificationService {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The salt must be random per call, or two users with the same password
        /// would produce the same stored string and a leaked table would give up
        /// every account at once.
        #[test]
        fn hashing_the_same_password_twice_gives_different_hashes() {
            let first = hash_password("same password").unwrap();
            let second = hash_password("same password").unwrap();
            assert_ne!(first, second);

            // Both still verify, so the difference really is the salt.
            assert!(verify_password("same password", &first));
            assert!(verify_password("same password", &second));
        }

        #[test]
        fn verify_rejects_the_wrong_password() {
            let hash = hash_password("correct horse").unwrap();
            assert!(!verify_password("wrong horse", &hash));
            assert!(!verify_password("", &hash));
        }

        /// A corrupted row must lock the account rather than crash the login path.
        #[test]
        fn verify_treats_a_malformed_hash_as_a_miss() {
            assert!(!verify_password("anything", "not-a-phc-string"));
            assert!(!verify_password("anything", ""));
        }

        #[test]
        fn stored_passwords_are_never_in_plaintext() {
            let hash = hash_password("correct horse").unwrap();
            assert!(!hash.contains("correct horse"));
        }
    }
}

/// Factory for creating service containers
pub mod factory {
    use super::*;
    use crate::config::AppConfig;

    pub async fn create_services(config: Arc<AppConfig>) -> Result<ServiceContainer> {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let settings = Arc::new(InMemoryRepository::<AppSettings>::new());

        // Shared by auth and user services so deleting an account also drops its
        // password hash.
        let credentials = Arc::new(defaults::CredentialStore::new());

        let auth = Arc::new(defaults::DefaultAuthService::with_credentials(
            users.clone(),
            credentials.clone(),
        ));
        let users_service = Arc::new(defaults::DefaultUserService::with_credentials(
            users.clone(),
            credentials,
        ));
        let settings_service = Arc::new(defaults::DefaultSettingsService::new(
            settings,
            users.clone(),
        ));

        // `DefaultNetworkService` is an alias for the stub when `network` is
        // off, and its `new()` takes no config in that case -- see the alias in
        // `lib.rs`. Building it here keeps the factory identical either way.
        #[cfg(feature = "network")]
        let network = Arc::new(defaults::DefaultNetworkService::new(config.clone()));
        #[cfg(not(feature = "network"))]
        let network = Arc::new(defaults::StubNetworkService::new());
        let storage = Arc::new(defaults::DefaultStorageService::new());
        let platform = Arc::new(defaults::DefaultPlatformService::new(
            config.platform.platform,
        ));
        let analytics = Arc::new(defaults::DefaultAnalyticsService::new(
            config.features.analytics,
        ));
        let notifications = Arc::new(defaults::DefaultNotificationService::new());

        Ok(ServiceContainer::new(
            AppServices {
                auth,
                users: users_service,
                settings: settings_service,
                network,
                storage,
            },
            PlatformServices {
                platform,
                analytics,
                notifications,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EntityId, Timestamp, User, UserPreferences};
    use uuid::Uuid;

    fn user(username: &str, email: &str) -> User {
        User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: email.to_string(),
            display_name: username.to_string(),
            avatar_url: None,
            preferences: UserPreferences::default(),
            created_at: Timestamp::default(),
            updated_at: Timestamp::default(),
        }
    }

    fn settings_for(user_id: EntityId) -> AppSettings {
        AppSettings {
            id: Uuid::new_v4(),
            user_id,
            window_state: WindowState {
                width: 800,
                height: 600,
                maximized: false,
                fullscreen: false,
                position: None,
            },
            ui_scale: 1.0,
            font_scale: 1.0,
            reduced_motion: false,
            accessibility: AccessibilitySettings::default(),
            created_at: Timestamp::default(),
            updated_at: Timestamp::default(),
        }
    }

    #[tokio::test]
    async fn test_default_auth_service() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users.clone());

        let registered = auth
            .register("test@example.com", "testuser", "correct horse", "Test User")
            .await
            .unwrap();

        let logged_in = auth
            .login("test@example.com", "correct horse")
            .await
            .unwrap();
        assert_eq!(logged_in.id, registered.id);

        let current = auth.current_user().await.unwrap();
        assert_eq!(current.map(|u| u.id), Some(registered.id));

        auth.logout().await.unwrap();
        assert!(auth.current_user().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn login_rejects_a_wrong_password_and_stays_logged_out() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users.clone());
        auth.register("test@example.com", "testuser", "correct horse", "Test User")
            .await
            .unwrap();

        // `register` signs the user in, so log out first: what matters is that
        // a failed login does not itself create a session.
        auth.logout().await.unwrap();

        let err = auth
            .login("test@example.com", "wrong password")
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            AppError::Auth(crate::errors::AuthError::InvalidCredentials)
        ));
        assert!(
            auth.current_user().await.unwrap().is_none(),
            "a failed login must not leave a session behind"
        );
    }

    #[tokio::test]
    async fn a_user_with_no_stored_hash_cannot_log_in() {
        // The half-registered case: a row in the user repository with no
        // matching credential. Registering the address directly must not be
        // enough to get in.
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users.clone());
        users
            .save(&user("testuser", "test@example.com"))
            .await
            .unwrap();

        assert!(auth.login("test@example.com", "").await.is_err());
        assert!(auth.login("test@example.com", "anything").await.is_err());
    }

    #[tokio::test]
    async fn register_rejects_a_short_password() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users.clone());

        let err = auth
            .register("test@example.com", "testuser", "short", "Test User")
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            AppError::Validation(crate::errors::ValidationError::TooShort { .. })
        ));
        assert!(users.is_empty(), "a rejected registration must not persist");
    }

    #[tokio::test]
    async fn register_rejects_a_duplicate_email_or_username() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users.clone());
        auth.register("test@example.com", "testuser", "correct horse", "Test User")
            .await
            .unwrap();

        assert!(auth
            .register("test@example.com", "other", "correct horse", "Other")
            .await
            .is_err());
        assert!(auth
            .register("other@example.com", "testuser", "correct horse", "Other")
            .await
            .is_err());
        assert_eq!(users.len(), 1);
    }

    #[tokio::test]
    async fn change_password_swaps_the_credential() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users.clone());
        auth.register("test@example.com", "testuser", "correct horse", "Test User")
            .await
            .unwrap();

        auth.change_password("correct horse", "new battery staple")
            .await
            .unwrap();

        assert!(auth
            .login("test@example.com", "correct horse")
            .await
            .is_err());
        assert!(auth
            .login("test@example.com", "new battery staple")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn change_password_requires_the_current_one() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users.clone());
        auth.register("test@example.com", "testuser", "correct horse", "Test User")
            .await
            .unwrap();

        assert!(auth
            .change_password("not the password", "new battery staple")
            .await
            .is_err());
        // The old password must still work after a rejected change.
        assert!(auth
            .login("test@example.com", "correct horse")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn change_password_requires_a_signed_in_user() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users);

        let err = auth
            .change_password("old", "new battery staple")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::Auth(crate::errors::AuthError::NotAuthenticated)
        ));
    }

    #[tokio::test]
    async fn reset_password_refuses_without_a_token_store() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users);

        assert!(auth
            .reset_password("any-token", "new battery staple")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn deleting_an_account_drops_its_credential() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let credentials = Arc::new(defaults::CredentialStore::new());
        let auth =
            defaults::DefaultAuthService::with_credentials(users.clone(), credentials.clone());
        let user_service =
            defaults::DefaultUserService::with_credentials(users, credentials.clone());

        let registered = auth
            .register("test@example.com", "testuser", "correct horse", "Test User")
            .await
            .unwrap();
        assert!(credentials.verify(registered.id, "correct horse").unwrap());

        user_service.delete_account(registered.id).await.unwrap();

        assert!(
            !credentials.verify(registered.id, "correct horse").unwrap(),
            "the password hash must not outlive the account"
        );
    }

    #[tokio::test]
    async fn a_user_id_with_no_credential_never_verifies() {
        let store = defaults::CredentialStore::new();
        store.set(Uuid::new_v4(), "correct horse").unwrap();

        assert!(!store.verify(Uuid::new_v4(), "correct horse").unwrap());
    }

    #[tokio::test]
    async fn search_users_matches_display_name() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let service = defaults::DefaultUserService::new(users.clone());

        let mut alicia = user("alicia", "alicia@example.com");
        alicia.display_name = "Alicia Wonderland".to_string();
        users.save(&alicia).await.unwrap();
        users.save(&user("bob", "bob@example.com")).await.unwrap();

        let hits = service.search_users("wonderland", 10).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].username, "alicia");

        assert_eq!(service.search_users("bob", 10).await.unwrap().len(), 1);
        assert_eq!(service.search_users("", 10).await.unwrap().len(), 0);
        assert_eq!(service.search_users("nobody", 10).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn list_users_returns_everyone_up_to_the_limit() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let service = defaults::DefaultUserService::new(users.clone());

        for name in ["alice", "bob", "carol"] {
            users
                .save(&user(name, &format!("{name}@example.com")))
                .await
                .unwrap();
        }

        assert_eq!(service.list_users(10).await.unwrap().len(), 3);
        assert_eq!(service.list_users(2).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn login_by_email_is_case_insensitive() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users);
        auth.register("Test@Example.com", "testuser", "correct horse", "Test User")
            .await
            .unwrap();

        let logged_in = auth
            .login("test@example.COM", "correct horse")
            .await
            .unwrap();
        assert_eq!(logged_in.username, "testuser");
    }

    #[tokio::test]
    async fn login_for_unknown_email_is_invalid_credentials() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let auth = defaults::DefaultAuthService::new(users);
        auth.register("test@example.com", "testuser", "correct horse", "Test User")
            .await
            .unwrap();

        let err = auth
            .login("nobody@example.com", "correct horse")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::Auth(crate::errors::AuthError::InvalidCredentials)
        ));
    }

    #[tokio::test]
    async fn find_by_username_is_case_insensitive() {
        let repo = InMemoryRepository::<User>::new();
        let u = user("TestUser", "test@example.com");
        repo.save(&u).await.unwrap();

        assert_eq!(
            repo.find_by_username("testuser")
                .await
                .unwrap()
                .map(|f| f.id),
            Some(u.id)
        );
    }

    #[tokio::test]
    async fn changing_email_drops_the_stale_index_entry() {
        let repo = InMemoryRepository::<User>::new();
        let mut u = user("testuser", "old@example.com");
        repo.save(&u).await.unwrap();

        u.email = "new@example.com".to_string();
        repo.save(&u).await.unwrap();

        assert!(repo
            .find_by_email("old@example.com")
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            repo.find_by_email("new@example.com")
                .await
                .unwrap()
                .map(|f| f.id),
            Some(u.id)
        );
    }

    #[tokio::test]
    async fn changing_username_drops_the_stale_index_entry() {
        let repo = InMemoryRepository::<User>::new();
        let mut u = user("oldname", "test@example.com");
        repo.save(&u).await.unwrap();

        u.username = "newname".to_string();
        repo.save(&u).await.unwrap();

        assert!(repo.find_by_username("oldname").await.unwrap().is_none());
        assert!(repo.find_by_username("newname").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn delete_removes_index_entries() {
        let repo = InMemoryRepository::<User>::new();
        let u = user("testuser", "test@example.com");
        repo.save(&u).await.unwrap();

        repo.delete(u.id).await.unwrap();

        assert!(repo.is_empty());
        assert!(repo.find_by_id(u.id).await.unwrap().is_none());
        assert!(repo
            .find_by_email("test@example.com")
            .await
            .unwrap()
            .is_none());
        assert!(repo.find_by_username("testuser").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_owner_returns_only_that_users_settings() {
        let repo = InMemoryRepository::<AppSettings>::new();
        let alice = Uuid::new_v4();
        let bob = Uuid::new_v4();

        let mut a = settings_for(alice);
        a.ui_scale = 1.5;
        let b = settings_for(bob);
        repo.save(&a).await.unwrap();
        repo.save(&b).await.unwrap();

        let alice_settings = repo.find_by_owner(alice).await.unwrap();
        assert_eq!(alice_settings.len(), 1);
        assert_eq!(alice_settings[0].ui_scale, 1.5);

        assert_eq!(repo.find_by_owner(bob).await.unwrap().len(), 1);
        assert!(repo.find_by_owner(Uuid::new_v4()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn search_matches_email_and_username_and_respects_limit() {
        let repo = InMemoryRepository::<User>::new();
        repo.save(&user("alice", "alice@example.com"))
            .await
            .unwrap();
        repo.save(&user("bob", "bob@example.com")).await.unwrap();

        assert_eq!(repo.search("alice", 10).await.unwrap().len(), 1);
        assert_eq!(repo.search("example.com", 10).await.unwrap().len(), 2);
        assert_eq!(repo.search("example.com", 1).await.unwrap().len(), 1);
        assert!(repo.search("nothing", 10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn settings_service_creates_defaults_on_first_read() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let settings = Arc::new(InMemoryRepository::<AppSettings>::new());
        let service = defaults::DefaultSettingsService::new(settings.clone(), users.clone());

        let u = user("testuser", "test@example.com");
        users.save(&u).await.unwrap();

        let loaded = service.get_settings(u.id).await.unwrap();
        assert_eq!(loaded.user_id, u.id);
        assert_eq!(loaded.font_scale, 1.0);

        // Reading twice must not create a second row.
        service.get_settings(u.id).await.unwrap();
        assert_eq!(settings.find_by_owner(u.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn settings_service_persists_every_update() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let settings = Arc::new(InMemoryRepository::<AppSettings>::new());
        let service = defaults::DefaultSettingsService::new(settings, users.clone());

        let u = user("testuser", "test@example.com");
        users.save(&u).await.unwrap();

        service.update_font_scale(u.id, 1.75).await.unwrap();
        service.update_ui_scale(u.id, 2.0).await.unwrap();
        service
            .update_theme(u.id, ThemePreference::Dark)
            .await
            .unwrap();
        service.update_language(u.id, "de").await.unwrap();

        let loaded = service.get_settings(u.id).await.unwrap();
        assert_eq!(loaded.font_scale, 1.75);
        assert_eq!(loaded.ui_scale, 2.0);

        let stored = users.find_by_id(u.id).await.unwrap().unwrap();
        assert_eq!(stored.preferences.theme, ThemePreference::Dark);
        assert_eq!(stored.preferences.language, "de");
    }

    #[tokio::test]
    async fn settings_service_clamps_scales() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let settings = Arc::new(InMemoryRepository::<AppSettings>::new());
        let service = defaults::DefaultSettingsService::new(settings, users.clone());

        let u = user("testuser", "test@example.com");
        users.save(&u).await.unwrap();

        service.update_font_scale(u.id, 99.0).await.unwrap();
        service.update_ui_scale(u.id, 0.0).await.unwrap();

        let loaded = service.get_settings(u.id).await.unwrap();
        assert_eq!(loaded.font_scale, 3.0);
        assert_eq!(loaded.ui_scale, 0.5);
    }

    #[tokio::test]
    async fn settings_service_reports_a_missing_user() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let settings = Arc::new(InMemoryRepository::<AppSettings>::new());
        let service = defaults::DefaultSettingsService::new(settings, users);

        let err = service
            .update_theme(Uuid::new_v4(), ThemePreference::Dark)
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::NotFound { .. }));
    }

    #[tokio::test]
    async fn reset_to_defaults_restores_scales_and_theme() {
        let users = Arc::new(InMemoryRepository::<User>::new());
        let settings = Arc::new(InMemoryRepository::<AppSettings>::new());
        let service = defaults::DefaultSettingsService::new(settings, users.clone());

        let u = user("testuser", "test@example.com");
        users.save(&u).await.unwrap();

        service.update_font_scale(u.id, 2.5).await.unwrap();
        service
            .update_theme(u.id, ThemePreference::HighContrast)
            .await
            .unwrap();
        service.update_language(u.id, "fr").await.unwrap();
        service.reset_to_defaults(u.id).await.unwrap();

        let loaded = service.get_settings(u.id).await.unwrap();
        assert_eq!(loaded.font_scale, 1.0);
        assert_eq!(loaded.ui_scale, 1.0);

        let stored = users.find_by_id(u.id).await.unwrap().unwrap();
        assert_eq!(stored.preferences.theme, ThemePreference::System);
        assert_eq!(stored.preferences.language, "en");
    }
}
