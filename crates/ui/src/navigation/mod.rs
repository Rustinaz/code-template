//! Navigation system for cross-platform UI
//!
//! This module provides a type-safe navigation system that works
//! across platforms and UI frameworks.

use parking_lot::RwLock;
use shared::domain::Route;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tracing::{debug, info};

/// Maps the remaining part of a deep link onto a [`Route`].
///
/// Returns `None` when the link does not match, which is how a handler
/// declines a link and lets the next registered pattern try.
pub type DeepLinkResolver = Box<dyn Fn(&str) -> Option<Route> + Send + Sync>;

/// Navigation stack entry
#[derive(Debug, Clone)]
pub struct NavEntry {
    pub route: Route,
    pub params: std::collections::HashMap<String, String>,
    pub timestamp: std::time::Instant,
}

/// Navigation direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
    Forward,
    Back,
    Replace,
    Reset,
}

/// Navigation controller for managing navigation state
pub struct NavigationController {
    stack: Arc<RwLock<VecDeque<NavEntry>>>,
    max_stack_size: usize,
    listeners: Arc<RwLock<Vec<Box<dyn NavigationListener>>>>,
}

impl NavigationController {
    pub fn new() -> Self {
        Self {
            stack: Arc::new(RwLock::new(VecDeque::new())),
            max_stack_size: 100,
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_stack_size = size;
        self
    }

    /// Navigate to a route
    pub fn navigate(&self, route: Route, direction: NavDirection) {
        let mut stack = self.stack.write();
        let entry = NavEntry {
            route: route.clone(),
            params: Self::extract_params(&route),
            timestamp: std::time::Instant::now(),
        };

        match direction {
            NavDirection::Forward => {
                stack.push_back(entry);
                if stack.len() > self.max_stack_size {
                    stack.pop_front();
                }
            }
            NavDirection::Back => {
                if stack.len() > 1 {
                    stack.pop_back();
                }
            }
            NavDirection::Replace => {
                stack.pop_back();
                stack.push_back(entry);
            }
            NavDirection::Reset => {
                stack.clear();
                stack.push_back(entry);
            }
        }

        self.notify_listeners(&route, direction);
        debug!("Navigated to {:?} ({:?})", route, direction);
    }

    /// Navigate forward
    pub fn push(&self, route: Route) {
        self.navigate(route, NavDirection::Forward);
    }

    /// Navigate back
    pub fn pop(&self) {
        self.navigate(Route::Home, NavDirection::Back);
    }

    /// Replace current route
    pub fn replace(&self, route: Route) {
        self.navigate(route, NavDirection::Replace);
    }

    /// Reset navigation stack
    pub fn reset(&self, route: Route) {
        self.navigate(route, NavDirection::Reset);
    }

    /// Get current route
    pub fn current(&self) -> Option<Route> {
        self.stack.read().back().map(|e| e.route.clone())
    }

    /// Get previous route
    pub fn previous(&self) -> Option<Route> {
        let stack = self.stack.read();
        if stack.len() >= 2 {
            let len = stack.len();
            Some(stack[len - 2].route.clone())
        } else {
            None
        }
    }

    /// Get full navigation stack
    pub fn stack(&self) -> Vec<NavEntry> {
        self.stack.read().iter().cloned().collect()
    }

    /// Check if can go back
    pub fn can_go_back(&self) -> bool {
        self.stack.read().len() > 1
    }

    /// Get stack depth
    pub fn depth(&self) -> usize {
        self.stack.read().len()
    }

    /// Clear stack
    pub fn clear(&self) {
        self.stack.write().clear();
    }

    /// Add navigation listener
    pub fn add_listener(&self, listener: Box<dyn NavigationListener>) {
        self.listeners.write().push(listener);
    }

    fn extract_params(route: &Route) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();
        match route {
            Route::Profile { user_id } => {
                params.insert("user_id".to_string(), user_id.to_string());
            }
            Route::Detail { item_id } => {
                params.insert("item_id".to_string(), item_id.to_string());
            }
            Route::Custom(path) => {
                // Parse path parameters
                for segment in path.split('/') {
                    if let Some(key) = segment.strip_prefix(':') {
                        params.insert(key.to_string(), String::new());
                    }
                }
            }
            _ => {}
        }
        params
    }

    fn notify_listeners(&self, route: &Route, direction: NavDirection) {
        for listener in self.listeners.read().iter() {
            listener.on_navigate(route, direction);
        }
    }
}

impl Default for NavigationController {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigation listener trait
pub trait NavigationListener: Send + Sync {
    fn on_navigate(&self, route: &Route, direction: NavDirection);
}

/// Deep link handler
pub struct DeepLinkHandler {
    routes: Arc<RwLock<HashMap<String, DeepLinkResolver>>>,
}

impl DeepLinkHandler {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register<F>(&self, pattern: String, resolver: F)
    where
        F: Fn(&str) -> Option<Route> + Send + Sync + 'static,
    {
        self.routes.write().insert(pattern, Box::new(resolver));
    }

    pub fn handle(&self, url: &str) -> Option<Route> {
        let url = url.strip_prefix("app://").unwrap_or(url);

        // Stub awaiting the real implementation: `_pattern` is not consulted
        // yet because a registered resolver takes the raw path. The real
        // implementation matches the path against the pattern first and only
        // then hands it to that pattern's resolver.
        for resolver in self.routes.read().values() {
            if let Some(route) = resolver(url) {
                info!("Deep link matched: {} -> {:?}", url, route);
                return Some(route);
            }
        }

        // Try to parse as route
        Self::parse_route(url)
    }

    fn parse_route(url: &str) -> Option<Route> {
        let path = url.strip_prefix('/').unwrap_or(url);
        let parts: Vec<&str> = path.split('/').collect();

        match parts.as_slice() {
            [""] | [] => Some(Route::Home),
            ["settings"] => Some(Route::Settings),
            ["profile", user_id] => user_id
                .parse()
                .ok()
                .map(|id| Route::Profile { user_id: id }),
            ["detail", item_id] => item_id.parse().ok().map(|id| Route::Detail { item_id: id }),
            _ => Some(Route::Custom(url.to_string())),
        }
    }
}

impl Default for DeepLinkHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigation builder for declarative navigation setup
pub struct NavigationBuilder {
    initial_route: Route,
    routes: HashMap<String, Route>,
    deep_links: Vec<(String, DeepLinkResolver)>,
}

impl NavigationBuilder {
    pub fn new(initial_route: Route) -> Self {
        Self {
            initial_route,
            routes: HashMap::new(),
            deep_links: Vec::new(),
        }
    }

    pub fn route(mut self, name: &str, route: Route) -> Self {
        self.routes.insert(name.to_string(), route);
        self
    }

    pub fn deep_link<F>(mut self, pattern: &str, resolver: F) -> Self
    where
        F: Fn(&str) -> Option<Route> + Send + Sync + 'static,
    {
        self.deep_links
            .push((pattern.to_string(), Box::new(resolver)));
        self
    }

    pub fn build(self) -> NavigationController {
        let controller = NavigationController::new();
        controller.reset(self.initial_route);

        let handler = DeepLinkHandler::new();
        for (pattern, resolver) in self.deep_links {
            handler.register(pattern, resolver);
        }

        controller
    }
}

/// Bottom navigation bar configuration
#[derive(Debug, Clone)]
pub struct BottomNavConfig {
    pub items: Vec<BottomNavItem>,
    pub selected_index: usize,
    pub show_labels: bool,
    pub background_color: Option<String>,
    pub selected_color: Option<String>,
    pub unselected_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BottomNavItem {
    pub route: Route,
    pub label: String,
    pub icon: String,
    pub active_icon: Option<String>,
    pub badge: Option<String>,
}

/// Navigation drawer configuration
#[derive(Debug, Clone)]
pub struct NavigationDrawerConfig {
    pub header: Option<DrawerHeader>,
    pub items: Vec<DrawerItem>,
    pub footer: Option<DrawerFooter>,
    pub width: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct DrawerHeader {
    pub title: String,
    pub subtitle: Option<String>,
    pub avatar: Option<String>,
    pub action: Option<Route>,
}

#[derive(Debug, Clone)]
pub struct DrawerItem {
    pub route: Route,
    pub label: String,
    pub icon: String,
    pub badge: Option<String>,
    pub divider_after: bool,
}

#[derive(Debug, Clone)]
pub struct DrawerFooter {
    pub items: Vec<DrawerItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    // Only the tests below need `EntityId`; importing it at module scope would
    // be an unused import in the non-test build.
    use shared::domain::EntityId;

    #[test]
    fn test_navigation_controller() {
        let nav = NavigationController::new();

        // A fresh controller is not on a screen yet, so it has nothing to pop.
        assert_eq!(nav.current(), None);
        assert_eq!(nav.depth(), 0);
        assert!(!nav.can_go_back());

        // The first push is the root of the stack, so there is still nothing
        // behind it.
        nav.push(Route::Settings);
        assert_eq!(nav.current(), Some(Route::Settings));
        assert_eq!(nav.depth(), 1);
        assert!(!nav.can_go_back());

        nav.push(Route::Profile {
            user_id: EntityId::new_v4(),
        });
        assert_eq!(nav.depth(), 2);
        assert!(nav.can_go_back());

        nav.pop();
        assert_eq!(nav.current(), Some(Route::Settings));
        // Popping the root is a no-op rather than emptying the stack.
        assert_eq!(nav.depth(), 1);
        assert!(!nav.can_go_back());
    }

    #[test]
    fn test_deep_link_handler() {
        let handler = DeepLinkHandler::new();

        assert_eq!(handler.handle("app://"), Some(Route::Home));
        assert_eq!(handler.handle("app://settings"), Some(Route::Settings));

        let profile_route = handler.handle("app://profile/123e4567-e89b-12d3-a456-426614174000");
        assert!(matches!(profile_route, Some(Route::Profile { .. })));
    }

    #[test]
    fn test_navigation_builder() {
        let nav = NavigationBuilder::new(Route::Home)
            .route("home", Route::Home)
            .route("settings", Route::Settings)
            .deep_link("profile/:id", |url| {
                let id = url.strip_prefix("profile/")?;
                let user_id = id.parse().ok()?;
                Some(Route::Profile { user_id })
            })
            .build();

        assert_eq!(nav.current(), Some(Route::Home));
    }
}
