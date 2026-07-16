use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::sync::futures::{Notified, OwnedNotified};

use crate::lifecycle::LifecycleContext;

impl LifecycleContext {
    /// Waits for the lifecycle context to be notified of shutdown.
    pub fn wait_for_shutdown(&self) -> Shutdown<'_> {
        Shutdown::new(self)
    }

    /// Waits for the lifecycle context to be notified of shutdown, consuming the context.
    pub fn wait_for_shutdown_owned(&self) -> ShutdownOwned {
        ShutdownOwned::new(self)
    }
}

/// A future that resolves when the [`LifecycleContext`] is shut down (via
/// [`notify_error`][crate::lifecycle::LifecycleContext::notify_error]).
///
/// Created via [`LifecycleContext::wait_for_shutdown`]. This future borrows the context,
/// so the context must outlive this value.
pub struct Shutdown<'a> {
    notified: Option<Notified<'a>>,
}

/// A future that resolves when the [`LifecycleContext`] is shut down (via
/// [`notify_error`][crate::lifecycle::LifecycleContext::notify_error]), consuming the context.
///
/// Created via [`LifecycleContext::wait_for_shutdown_owned`]. Unlike [`Shutdown`], this future
/// does not borrow the context — it owns a copy of the underlying notification handle, allowing
/// it to be sent across task boundaries.
pub struct ShutdownOwned {
    owned_notified: Option<OwnedNotified>,
}

impl<'a> Shutdown<'a> {
    fn new(context: &'a LifecycleContext) -> Self {
        let notified = match context.is_shutdown() {
            true => None,
            false => Some(context.notify.notified()),
        };

        Self { notified }
    }

    fn project<'pin>(self: Pin<&'pin mut Self>) -> Option<Pin<&'pin mut Notified<'a>>> {
        unsafe {
            // SAFETY: We are not moving any fields out of the struct, just creating a pinned
            // reference within the struct, which keeps the pin invariants intact.
            match self.get_unchecked_mut().notified {
                Some(ref mut notified) => Some(Pin::new_unchecked(notified)),
                None => None,
            }
        }
    }
}

impl ShutdownOwned {
    fn new(context: &LifecycleContext) -> Self {
        let owned_notified = match context.is_shutdown() {
            true => None,
            false => Some(context.notify.clone().notified_owned()),
        };

        Self { owned_notified }
    }

    fn project(self: Pin<&'_ mut Self>) -> Option<Pin<&'_ mut OwnedNotified>> {
        unsafe {
            // SAFETY: We are not moving any fields out of the struct, just creating a pinned
            // reference within the struct, which keeps the pin invariants intact.
            match self.get_unchecked_mut().owned_notified {
                Some(ref mut owned_notified) => Some(Pin::new_unchecked(owned_notified)),
                None => None,
            }
        }
    }
}

macro_rules! impl_shutdown_future {
    ($shutdown_type:ty) => {
        impl Future for $shutdown_type {
            type Output = ();

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                // Poll the inner Notified or OwnedNotified future, if it exists.
                // If it doesn't exist, that means the context has already been notified of shutdown,
                // so we can return Poll::Ready immediately.
                self.project()
                    .map(|notified| notified.poll(cx))
                    .unwrap_or(Poll::Ready(()))
            }
        }
    };
}

impl_shutdown_future!(Shutdown<'_>);
impl_shutdown_future!(ShutdownOwned);
