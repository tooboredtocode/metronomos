use std::time::Duration;

use crate::lifecycle::context::{LifecycleContext, LifecycleContextManager};

/// Represents the timeout configuration for lifecycle hooks in the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LifecycleTimeout {
    Default,
    None,
    Set(Duration),
}

type BoxedStartFn =
    Box<dyn Fn(&mut LifecycleContextManager, Option<Duration>) + Send + Sync + 'static>;

pub(super) struct LifecycleHook {
    inner: BoxedStartFn,
    timeout: LifecycleTimeout,
}

impl LifecycleTimeout {
    /// Returns the effective timeout based on the variant and a provided default.
    fn effective_timeout(&self, default: Duration) -> Option<Duration> {
        match self {
            LifecycleTimeout::Default => Some(default),
            LifecycleTimeout::None => None,
            LifecycleTimeout::Set(duration) => Some(*duration),
        }
    }
}

impl LifecycleHook {
    pub(super) fn new<F, Fut>(run: F) -> Self
    where
        F: Fn(LifecycleContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            inner: Box::new(
                move |ctx: &mut LifecycleContextManager, timeout: Option<Duration>| {
                    ctx.spawn(&run, timeout)
                },
            ),
            timeout: LifecycleTimeout::Default,
        }
    }

    pub(super) fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = LifecycleTimeout::Set(timeout);
    }

    pub(super) fn disable_timeout(&mut self) {
        self.timeout = LifecycleTimeout::None;
    }

    #[inline]
    pub(super) fn start_on(&self, ctx: &mut LifecycleContextManager, default_timeout: Duration) {
        let timeout = self.timeout.effective_timeout(default_timeout);
        (self.inner)(ctx, timeout);
    }
}
