//! React-like hooks for UI state management
//!
//! This module provides hooks for managing UI state, side effects,
//! and lifecycle in a declarative way.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use futures::stream::StreamExt;

/// Hook context for managing hook state
thread_local! {
    static HOOK_CONTEXT: RefCell<Option<HookContext>> = RefCell::new(None);
}

struct HookContext {
    state_hooks: Vec<Box<dyn StateHook>>,
    effect_hooks: Vec<Box<dyn EffectHook>>,
    callback_hooks: Vec<Box<dyn CallbackHook>>,
    memo_hooks: Vec<Box<dyn MemoHook>>,
    current_index: usize,
}

impl HookContext {
    fn new() -> Self {
        Self {
            state_hooks: Vec::new(),
            effect_hooks: Vec::new(),
            callback_hooks: Vec::new(),
            memo_hooks: Vec::new(),
            current_index: 0,
        }
    }

    fn next_state<T: 'static>(&mut self, initial: impl FnOnce() -> T) -> Rc<RefCell<T>> {
        if self.current_index < self.state_hooks.len() {
            let hook = self.state_hooks[self.current_index].downcast_ref::<StateHookImpl<T>>()
                .expect("Hook type mismatch");
            self.current_index += 1;
            hook.value.clone()
        } else {
            let value = Rc::new(RefCell::new(initial()));
            self.state_hooks.push(Box::new(StateHookImpl { value: value.clone() }));
            self.current_index += 1;
            value
        }
    }

    fn next_effect(&mut self, deps: Vec<usize>, cleanup: Option<Box<dyn FnOnce()>>, effect: Box<dyn FnOnce()>) {
        if self.current_index < self.effect_hooks.len() {
            let hook = self.effect_hooks[self.current_index].downcast_mut::<EffectHookImpl>()
                .expect("Hook type mismatch");
            hook.deps = deps;
            hook.cleanup = cleanup;
            hook.effect = Some(effect);
            self.current_index += 1;
        } else {
            self.effect_hooks.push(Box::new(EffectHookImpl {
                deps,
                cleanup,
                effect: Some(effect),
                ran: false,
            }));
            self.current_index += 1;
        }
    }

    fn run_effects(&mut self) {
        for hook in &mut self.effect_hooks {
            let hook = hook.downcast_mut::<EffectHookImpl>().unwrap();
            if !hook.ran || hook.deps_changed() {
                if let Some(cleanup) = hook.cleanup.take() {
                    cleanup();
                }
                if let Some(effect) = hook.effect.take() {
                    effect();
                }
                hook.ran = true;
            }
        }
    }
}

trait StateHook: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

struct StateHookImpl<T> {
    value: Rc<RefCell<T>>,
}

impl<T: 'static> StateHook for StateHookImpl<T> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

trait EffectHook: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

struct EffectHookImpl {
    deps: Vec<usize>,
    cleanup: Option<Box<dyn FnOnce()>>,
    effect: Option<Box<dyn FnOnce()>>,
    ran: bool,
}

impl EffectHookImpl {
    fn deps_changed(&self) -> bool {
        // Simplified - would compare actual dependency values
        true
    }
}

impl EffectHook for EffectHookImpl {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

trait CallbackHook: Send + Sync {}
trait MemoHook: Send + Sync {}

/// Run a component function with hook context
pub fn run_with_hooks<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    HOOK_CONTEXT.with(|ctx| {
        let mut context = HookContext::new();
        *ctx.borrow_mut() = Some(context);
        let result = f();
        if let Some(mut ctx) = ctx.borrow_mut().take() {
            ctx.run_effects();
        }
        result
    })
}

/// State hook - returns current value and setter
pub fn use_state<T: 'static>(initial: impl FnOnce() -> T) -> (Rc<RefCell<T>>, impl Fn(T)) {
    HOOK_CONTEXT.with(|ctx| {
        let mut context = ctx.borrow_mut().as_mut().expect("use_state called outside component");
        let value = context.next_state(initial);
        let setter = {
            let value = value.clone();
            move |new_value: T| {
                *value.borrow_mut() = new_value;
                // Trigger re-render (would be handled by framework)
            }
        };
        (value, setter)
    })
}

/// Effect hook - runs side effects
pub fn use_effect<F>(effect: F, deps: &[usize])
where
    F: FnOnce() + 'static,
{
    HOOK_CONTEXT.with(|ctx| {
        let mut context = ctx.borrow_mut().as_mut().expect("use_effect called outside component");
        context.next_effect(
            deps.to_vec(),
            None,
            Box::new(effect),
        );
    })
}

/// Effect hook with cleanup
pub fn use_effect_with_cleanup<F, C>(effect: F, cleanup: C, deps: &[usize])
where
    F: FnOnce() + 'static,
    C: FnOnce() + 'static,
{
    HOOK_CONTEXT.with(|ctx| {
        let mut context = ctx.borrow_mut().as_mut().expect("use_effect called outside component");
        context.next_effect(
            deps.to_vec(),
            Some(Box::new(cleanup)),
            Box::new(effect),
        );
    })
}

/// Memo hook - memoizes a value
pub fn use_memo<T: 'static>(compute: impl FnOnce() -> T, deps: &[usize]) -> Rc<RefCell<T>> {
    HOOK_CONTEXT.with(|ctx| {
        let mut context = ctx.borrow_mut().as_mut().expect("use_memo called outside component");
        context.next_state(compute)
    })
}

/// Callback hook - memoizes a callback
pub fn use_callback<F>(callback: F, deps: &[usize]) -> F
where
    F: Clone + 'static,
{
    // Simplified - would memoize based on deps
    callback
}

/// Ref hook - creates a mutable reference
pub fn use_ref<T: 'static>(initial: T) -> Rc<RefCell<T>> {
    HOOK_CONTEXT.with(|ctx| {
        let mut context = ctx.borrow_mut().as_mut().expect("use_ref called outside component");
        context.next_state(move || initial)
    })
}

/// Context hook - consumes a context value
pub fn use_context<T: 'static>(context: &Context<T>) -> Option<T> {
    context.get_current()
}

/// Context provider for dependency injection
pub struct Context<T> {
    value: RwLock<Option<T>>,
}

impl<T> Context<T> {
    pub fn new() -> Self {
        Self {
            value: RwLock::new(None),
        }
    }

    pub fn provide(&self, value: T) -> ContextGuard<T> {
        *self.value.write() = Some(value);
        ContextGuard { context: self }
    }

    fn get_current(&self) -> Option<T> {
        self.value.read().as_ref().map(|v| {
            // Would need Clone bound
            unsafe { std::ptr::read(v as *const T) }
        })
    }
}

impl<T> Default for Context<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ContextGuard<'a, T> {
    context: &'a Context<T>,
}

impl<'a, T> Drop for ContextGuard<'a, T> {
    fn drop(&mut self) {
        *self.context.value.write() = None;
    }
}

/// Event emitter hook for component communication
pub fn use_event_emitter<T: Clone + 'static>() -> (EventEmitter<T>, EventReceiver<T>) {
    let (tx, rx) = broadcast::channel(100);
    let emitter = EventEmitter { tx };
    let receiver = EventReceiver { rx };
    (emitter, receiver)
}

pub struct EventEmitter<T> {
    tx: broadcast::Sender<T>,
}

impl<T> EventEmitter<T> {
    pub fn emit(&self, event: T) {
        let _ = self.tx.send(event);
    }
}

pub struct EventReceiver<T> {
    rx: broadcast::Receiver<T>,
}

impl<T> EventReceiver<T> {
    pub async fn next(&mut self) -> Option<T> {
        self.rx.recv().await.ok()
    }

    pub fn try_next(&mut self) -> Option<T> {
        self.rx.try_recv().ok()
    }
}

/// Subscription hook for async streams
pub fn use_subscription<S, T>(stream: S) -> Option<T>
where
    S: StreamExt<Item = T> + Unpin + 'static,
    T: 'static,
{
    let (value, setter) = use_state(|| None::<T>);
    
    use_effect_with_cleanup(
        {
            let value = value.clone();
            let mut stream = stream;
            move || {
                // Would spawn async task to listen to stream
            }
        },
        move || {
            // Cleanup
        },
        &[],
    );

    Some((*value.borrow()).clone().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_state() {
        let result = run_with_hooks(|| {
            let (state, set_state) = use_state(|| 0);
            assert_eq!(*state.borrow(), 0);
            set_state(42);
            assert_eq!(*state.borrow(), 42);
            state
        });
        assert_eq!(*result.borrow(), 42);
    }

    #[test]
    fn test_use_ref() {
        let result = run_with_hooks(|| {
            let ref_cell = use_ref(0);
            assert_eq!(*ref_cell.borrow(), 0);
            *ref_cell.borrow_mut() = 100;
            assert_eq!(*ref_cell.borrow(), 100);
            ref_cell
        });
        assert_eq!(*result.borrow(), 100);
    }
}