//! Minimal timing utilities vendored from `dioxus-sdk-time` (0.7.0).
//!
//! `dioxus-sdk-time` has no Dioxus 0.8 release yet, and its 0.7 dependency
//! pulls a second, incompatible copy of `dioxus-core` into any 0.8 dependency
//! graph. Only [`sleep`] and [`use_timeout`] are used by these primitives, so
//! they are vendored here (both crates are MIT OR Apache-2.0) to keep the tree
//! on a single Dioxus version. See the upstream crate for `use_debounce` /
//! `use_interval`.

use dioxus::{
    core::Task,
    dioxus_core::SpawnIfAsync,
    prelude::{Callback, spawn, use_hook},
    signals::Signal,
};
use futures::{SinkExt, StreamExt, channel::mpsc};
use std::time::Duration;

/// Pause the current task for the specified duration.
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(duration).await;

    #[cfg(target_family = "wasm")]
    gloo_timers::future::sleep(duration).await;
}

/// The interface to a timeout, used to trigger it with [`UseTimeout::action`].
pub struct UseTimeout<Args: 'static> {
    duration: Duration,
    sender: Signal<mpsc::UnboundedSender<Args>>,
}

impl<Args> UseTimeout<Args> {
    /// Trigger the timeout. If no arguments are desired, use the unit type.
    pub fn action(&self, args: Args) -> TimeoutHandle {
        let mut sender = (self.sender)();
        let duration = self.duration;

        let handle = spawn(async move {
            #[cfg(not(target_family = "wasm"))]
            tokio::time::sleep(duration).await;

            #[cfg(target_family = "wasm")]
            gloo_timers::future::sleep(duration).await;

            // If this errors then the timeout was likely dropped.
            let _ = sender.send(args).await;
        });

        TimeoutHandle { handle }
    }
}

impl<Args> Clone for UseTimeout<Args> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Args> Copy for UseTimeout<Args> {}
impl<Args> PartialEq for UseTimeout<Args> {
    fn eq(&self, other: &Self) -> bool {
        self.duration == other.duration && self.sender == other.sender
    }
}

/// A handle to a pending timeout, allowing it to be cancelled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeoutHandle {
    handle: Task,
}

impl TimeoutHandle {
    /// Cancel the timeout associated with this handle.
    #[allow(dead_code)] // part of the vendored public API; not used by these primitives yet
    pub fn cancel(self) {
        self.handle.cancel();
    }
}

/// A hook to run a callback after a period of time. Unlike a debounce, calling
/// the timeout again while it is running starts another instance rather than
/// resetting the timer.
pub fn use_timeout<Args: 'static, MaybeAsync: SpawnIfAsync<Marker>, Marker>(
    duration: Duration,
    callback: impl FnMut(Args) -> MaybeAsync + 'static,
) -> UseTimeout<Args> {
    use_hook(|| {
        let callback = Callback::new(callback);
        let (sender, mut receiver) = mpsc::unbounded();

        spawn(async move {
            loop {
                if let Some(args) = receiver.next().await {
                    callback.call(args);
                }
            }
        });

        UseTimeout {
            duration,
            sender: Signal::new(sender),
        }
    })
}
