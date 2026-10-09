//! Platform-specific implementations - Web (WASM)
//!
//! This module provides Web-specific UI implementations using WASM and DOM.

use super::{
    Platform, PlatformColors, PlatformFonts, PlatformMotion, PlatformShapes, PlatformSpacing,
    PlatformTheme, PlatformUi, Window, WindowConfig,
};
use parking_lot::RwLock;
use shared::errors::Result;
use std::sync::Arc;

pub struct WebUi {
    document: Arc<RwLock<Option<WebDocument>>>,
}

/// Handle to the browser document that the UI is mounted into.
///
/// Wraps the live `web_sys::Document` so the type can exist on every target:
/// off the web there is no document, and [`WebUi`] still has to be
/// layout-compatible with the other platforms.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
pub struct WebDocument {
    /// The document the canvas is mounted into.
    inner: web_sys::Document,
}

#[cfg(target_arch = "wasm32")]
impl WebDocument {
    /// The wrapped document.
    #[must_use]
    pub fn inner(&self) -> &web_sys::Document {
        &self.inner
    }
}

/// Handle to the browser document that the UI is mounted into.
///
/// Non-WASM placeholder: there is no document off the web, so this is a unit
/// struct that exists only to keep [`WebUi`] layout-compatible across targets.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct WebDocument;

impl Default for WebUi {
    fn default() -> Self {
        Self::new()
    }
}

impl WebUi {
    pub fn new() -> Self {
        Self {
            document: Arc::new(RwLock::new(None)),
        }
    }

    /// The shared slot holding the host [`WebDocument`], if any.
    ///
    /// Populated by [`Self::set_document`] on WASM and left `None` otherwise.
    /// The lock is exposed rather than the inner value so callers can take a
    /// read or a write guard without this accessor having to guess their
    /// intent.
    pub fn document(&self) -> &Arc<RwLock<Option<WebDocument>>> {
        &self.document
    }

    /// Records the document the UI is mounted into.
    ///
    /// Takes the document by value and keeps it: dropping it here would leave
    /// [`Self::document`] permanently `None` and nothing able to reach the DOM.
    #[cfg(target_arch = "wasm32")]
    pub fn set_document(&self, doc: web_sys::Document) {
        *self.document.write() = Some(WebDocument { inner: doc });
    }
}

impl PlatformUi for WebUi {
    fn platform(&self) -> Platform {
        Platform::Web
    }

    fn create_window(&self, config: WindowConfig) -> Result<Box<dyn Window>> {
        Ok(Box::new(WebWindow::new(config)))
    }

    fn theme(&self) -> PlatformTheme {
        PlatformTheme {
            name: "Web".to_string(),
            colors: PlatformColors {
                primary: "#1976D2".to_string(),
                on_primary: "#FFFFFF".to_string(),
                background: "#FAFAFA".to_string(),
                on_background: "#212121".to_string(),
                surface: "#FFFFFF".to_string(),
                on_surface: "#212121".to_string(),
                error: "#D32F2F".to_string(),
                on_error: "#FFFFFF".to_string(),
            },
            fonts: PlatformFonts {
                family: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto"
                    .to_string(),
                size_scale: 1.0,
                weight_regular: 400,
                weight_medium: 500,
                weight_bold: 600,
            },
            spacing: PlatformSpacing {
                base: 8.0,
                scale: 1.0,
            },
            shapes: PlatformShapes {
                corner_radius_small: 4.0,
                corner_radius_medium: 8.0,
                corner_radius_large: 12.0,
            },
            motion: PlatformMotion {
                duration_short: 100,
                duration_medium: 200,
                duration_long: 300,
                easing_standard: "cubic-bezier(0.4, 0.0, 0.2, 1)".to_string(),
            },
        }
    }

    fn on_resume(&self) {
        // Handle web page visibility change
    }

    fn on_pause(&self) {
        // Handle web page hidden
    }

    fn on_destroy(&self) {
        // Handle web page unload
    }

    fn announce_for_accessibility(&self, _text: &str) {
        // Use ARIA live regions
    }

    fn set_accessibility_focus(&self, _element_id: &str) {
        // Set focus on DOM element
    }
}

struct WebWindow {
    config: WindowConfig,
}

impl WebWindow {
    fn new(config: WindowConfig) -> Self {
        Self { config }
    }
}

impl Window for WebWindow {
    fn show(&self) {
        // Show web app (remove hidden attribute)
    }

    fn hide(&self) {
        // Hide web app (add hidden attribute)
    }

    fn close(&self) {
        // Close web app (window.close())
    }

    fn set_title(&self, _title: &str) {
        // Set document.title
    }

    fn set_size(&self, _width: u32, _height: u32) {
        // Resize window (if popup) or adjust canvas
    }

    fn set_min_size(&self, _width: u32, _height: u32) {
        // Set CSS min-width/min-height
    }

    fn set_max_size(&self, _width: u32, _height: u32) {
        // Set CSS max-width/max-height
    }

    fn set_fullscreen(&self, _fullscreen: bool) {
        // Use Fullscreen API
    }

    fn set_maximized(&self, _maximized: bool) {
        // Not applicable on web
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn set_position(&self, _x: i32, _y: i32) {
        // Not typically used on web
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn is_fullscreen(&self) -> bool {
        self.config.fullscreen
    }

    fn is_maximized(&self) -> bool {
        false
    }
}

/// Web-specific DOM utilities
#[cfg(target_arch = "wasm32")]
pub mod dom {
    use wasm_bindgen::prelude::*;
    use web_sys::{window, Document, Element, HtmlElement, Node};

    pub fn document() -> Option<Document> {
        window()?.document()
    }

    /// The document, or a `JsValue` error explaining why there isn't one.
    ///
    /// A page loaded with a restrictive CSP or before the DOM is ready has a
    /// window but no document, so `document()` alone cannot distinguish "not yet"
    /// from "never".
    pub fn require_document() -> Result<Document, JsValue> {
        document().ok_or_else(|| JsValue::from_str("window has no document"))
    }

    /// The document body as a generic [`Element`].
    ///
    /// `Document::body` is typed `HtmlElement`; widening it to `Element` is
    /// what every caller in this file actually wants, since they go on to call
    /// the `Element`-level methods.
    pub fn body() -> Option<Element> {
        document()?.body().map(Into::into)
    }

    pub fn create_element(tag: &str) -> Result<Element, JsValue> {
        require_document()?.create_element(tag)
    }

    pub fn query_selector(selector: &str) -> Result<Option<Element>, JsValue> {
        require_document()?.query_selector(selector)
    }

    pub fn query_selector_all(selector: &str) -> Result<Vec<Element>, JsValue> {
        let node_list = require_document()?.query_selector_all(selector)?;
        let mut elements = Vec::new();
        for index in 0..node_list.length() {
            // `NodeList::item` is typed as `Node` because a NodeList can hold
            // text nodes too. A selector match is always an Element, but the
            // cast is a downcast rather than a conversion, so it is checked.
            if let Some(node) = node_list.item(index) {
                if let Ok(element) = node.dyn_into::<Element>() {
                    elements.push(element);
                }
            }
        }
        Ok(elements)
    }

    pub fn set_attribute(element: &Element, name: &str, value: &str) -> Result<(), JsValue> {
        element.set_attribute(name, value)
    }

    pub fn get_attribute(element: &Element, name: &str) -> Option<String> {
        element.get_attribute(name)
    }

    pub fn add_class(element: &Element, class: &str) -> Result<(), JsValue> {
        let class_list = element.class_list();
        class_list.add_1(class)
    }

    pub fn remove_class(element: &Element, class: &str) -> Result<(), JsValue> {
        let class_list = element.class_list();
        class_list.remove_1(class)
    }

    /// Adds `class` if absent, removes it if present.
    ///
    /// `DomTokenList` exposes `toggle` (flip) and `toggle_with_force` (set), not
    /// an arity-overloaded `toggle_1`, so the flip form is called directly.
    pub fn toggle_class(element: &Element, class: &str) -> Result<bool, JsValue> {
        let class_list = element.class_list();
        class_list.toggle(class)
    }

    /// Adds or removes `class` to reach `present`, reporting the resulting state.
    pub fn set_class(element: &Element, class: &str, present: bool) -> Result<bool, JsValue> {
        let class_list = element.class_list();
        class_list.toggle_with_force(class, present)
    }

    pub fn set_style(element: &HtmlElement, property: &str, value: &str) {
        let style = element.style();
        let _ = style.set_property(property, value);
    }

    pub fn append_child(parent: &Node, child: &Node) -> Result<Node, JsValue> {
        parent.append_child(child)
    }

    pub fn remove_child(parent: &Node, child: &Node) -> Result<Node, JsValue> {
        parent.remove_child(child)
    }

    pub fn set_text_content(element: &Element, text: &str) {
        element.set_text_content(Some(text));
    }
}

/// Web-specific event handling
#[cfg(target_arch = "wasm32")]
pub mod events {
    use wasm_bindgen::prelude::*;
    use web_sys::{Event, EventTarget};

    pub fn add_event_listener<F>(
        target: &EventTarget,
        event: &str,
        callback: F,
    ) -> Result<(), JsValue>
    where
        F: FnMut(Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(Event)>);
        target.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())?;
        closure.forget();
        Ok(())
    }

    pub fn add_event_listener_with_options<F>(
        target: &EventTarget,
        event: &str,
        options: &web_sys::AddEventListenerOptions,
        callback: F,
    ) -> Result<(), JsValue>
    where
        F: FnMut(Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(Event)>);
        target.add_event_listener_with_callback_and_add_event_listener_options(
            event,
            closure.as_ref().unchecked_ref(),
            options,
        )?;
        closure.forget();
        Ok(())
    }

    pub fn remove_event_listener(
        target: &EventTarget,
        event: &str,
        callback: &js_sys::Function,
    ) -> Result<(), JsValue> {
        target.remove_event_listener_with_callback(event, callback)
    }

    pub fn prevent_default(event: &Event) {
        event.prevent_default();
    }

    pub fn stop_propagation(event: &Event) {
        event.stop_propagation();
    }
}

/// Web-specific CSS utilities
#[cfg(target_arch = "wasm32")]
pub mod css {
    use wasm_bindgen::prelude::*;
    use web_sys::{window, Element, HtmlElement};

    /// The inline-style declaration of `element`.
    ///
    /// `style` is a property of `HtmlElement` and `SvgElement`, not of the
    /// generic `Element`, so this downcasts and reports the miss rather than
    /// reaching for a method that does not exist. SVG and MathML roots are the
    /// elements that legitimately fail here.
    fn style_of(element: &Element) -> Result<web_sys::CssStyleDeclaration, JsValue> {
        element
            .dyn_ref::<HtmlElement>()
            .map(HtmlElement::style)
            .ok_or_else(|| JsValue::from_str("element is not an HtmlElement"))
    }

    /// Sets a CSS custom property on the document root, i.e. a global variable.
    pub fn set_css_variable(name: &str, value: &str) {
        let Some(root) = window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        else {
            return;
        };
        if let Ok(style) = style_of(&root) {
            let _ = style.set_property(name, value);
        }
    }

    pub fn get_css_variable(name: &str) -> Option<String> {
        let root = window()?.document()?.document_element()?;
        style_of(&root).ok()?.get_property_value(name).ok()
    }

    pub fn set_element_style(
        element: &Element,
        property: &str,
        value: &str,
    ) -> Result<(), JsValue> {
        style_of(element)?.set_property(property, value)
    }

    pub fn get_element_style(element: &Element, property: &str) -> Result<String, JsValue> {
        style_of(element)?.get_property_value(property)
    }

    pub fn add_stylesheet(css: &str) -> Result<(), JsValue> {
        let document = super::dom::require_document()?;
        let style = document.create_element("style")?;
        style.set_text_content(Some(css));
        // A `Document` with no `<head>` is possible under a hostile CSP, so this
        // reports rather than panics the way `unwrap()` would.
        let head = document
            .head()
            .ok_or_else(|| JsValue::from_str("document has no head"))?;
        head.append_child(&style)?;
        Ok(())
    }
}
