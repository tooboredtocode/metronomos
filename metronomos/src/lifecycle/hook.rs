use crate::lifecycle::context::{LifecycleContext, LifecycleContextManager};

type BoxedStartFn = Box<dyn Fn(&mut LifecycleContextManager) + Send + Sync + 'static>;

pub(super) struct LifeCycleHook {
    inner: BoxedStartFn,
}

impl LifeCycleHook {
    pub(super) fn new<F, Fut>(run: F) -> Self
    where
        F: Fn(LifecycleContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self {
            inner: Box::new(move |ctx: &mut LifecycleContextManager| ctx.spawn(&run)),
        }
    }

    #[inline]
    pub(super) fn start(&self, ctx: &mut LifecycleContextManager) {
        (self.inner)(ctx);
    }
}
