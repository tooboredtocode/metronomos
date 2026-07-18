use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use pin_project::pin_project;

use crate::lifecycle::LifecycleContext;
use crate::lifecycle::context::ShutdownOwned;

impl LifecycleContext {
    /// Wraps a stream with the lifecycle context, so that it will terminate on shutdown.
    ///
    /// ### Note
    /// If your event loop is based on a stream, you should wrap it with this method to ensure
    /// that on shutdown, the stream will terminate and the event loop will exit gracefully.
    pub fn wrap_stream<S>(&self, stream: S) -> LifecycleStream<S>
    where
        S: Stream,
    {
        LifecycleStream {
            shutdown: self.wait_for_shutdown_owned(),
            inner: stream,
        }
    }
}

/// A [`futures::Stream`] that terminates when the lifecycle context is shut down.
///
/// Wraps any stream and automatically terminates it when a shutdown signal is received
/// (via [`notify_error`][crate::lifecycle::LifecycleContext::notify_error]). This is useful
/// for event-loop-based applications where the stream drives the main event loop.
#[pin_project]
pub struct LifecycleStream<S> {
    #[pin]
    shutdown: ShutdownOwned,
    #[pin]
    inner: S,
}

impl<S> LifecycleStream<S>
where
    S: Stream,
{
    /// Returns a reference to the inner stream.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Returns a mutable reference to the inner stream.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Returns a pinned reference to the inner stream.
    pub fn inner_pin(self: Pin<&Self>) -> Pin<&S> {
        self.project_ref().inner
    }

    /// Returns a pinned mutable reference to the inner stream.
    pub fn inner_pin_mut(self: Pin<&mut Self>) -> Pin<&mut S> {
        self.project().inner
    }
}

impl<S> Stream for LifecycleStream<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().project();

        if this.shutdown.poll(cx).is_ready() {
            return Poll::Ready(None);
        }

        this.inner.poll_next(cx)
    }
}
