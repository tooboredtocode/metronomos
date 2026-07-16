use std::marker::PhantomData;
use std::pin::Pin;

use any_container::AnyCloneBox;
use metronomos_loom::dependency::DependencyItem;
use seq_macro::seq;

use crate::container::PulseContext;
use crate::dependency::PulseDependencyInfo;
use crate::dependency::util::IntoPulseResult;
use crate::error::PulseError;
use crate::value::{FromPulseValue, PulseValue, utils as value_utils};

type ErasedAsyncFnDepResult = Result<AnyCloneBox, PulseError>;
type ErasedAsyncFnDepFuture<'a> = Pin<Box<dyn Future<Output = ErasedAsyncFnDepResult> + 'a>>;

pub(super) trait ErasedAsyncFnDep: Send + Sync {
    fn provide<'a>(&'a self, context: PulseContext<'a>) -> ErasedAsyncFnDepFuture<'a>;
}

struct MakeErasedAsyncFnDep<F, Dep> {
    f: F,
    phantom: PhantomData<fn(Dep)>,
}

impl<F, Dep> ErasedAsyncFnDep for MakeErasedAsyncFnDep<F, Dep>
where
    F: AsyncFnDependency<Dep>,
    Dep: Send + Sync + 'static,
{
    fn provide<'a>(&'a self, context: PulseContext<'a>) -> ErasedAsyncFnDepFuture<'a> {
        Box::pin(async move {
            self.f
                .provide_from_context(context)
                .await
                .map(AnyCloneBox::new)
        })
    }
}

pub(super) fn erase_async_fn_dep<F, Dep>(fn_dep: F) -> Box<dyn ErasedAsyncFnDep>
where
    F: AsyncFnDependency<Dep>,
    Dep: Send + Sync + 'static,
{
    Box::new(MakeErasedAsyncFnDep {
        f: fn_dep,
        phantom: PhantomData,
    })
}

/// Trait for asynchronous dependency providers.
///
/// Implementing this trait enables an async function to be registered as a dependency
/// via [`PulseContainerBuilder::provide_async`](crate::builder::PulseContainerBuilder::provide_async).
///
/// # Automatically Implemented
///
/// This trait is automatically implemented for any async closure `|Arg1, Arg2, ..., N| -> Fut` where:
///
/// - Each argument `Arg1..N` implements [`FromPulseValue`](crate::value::FromPulseValue)
/// - The returned future resolves to a type that either is the value directly or `Result<T, BuildDependencyError>`
///
/// Arguments are resolved from the container at build time and passed to the closure.
/// The future's resolved value becomes the dependency's output.
///
/// # Example
///
/// ```
/// use metronomos_pulse::PulseContainer;
/// use metronomos_pulse::value::{ArcValue, PulseValue};
///
/// #[derive(PulseValue, Clone, Debug)]
/// struct Config {
///     host: String,
///     port: u16,
/// }
///
/// # tokio::runtime::Builder::new_current_thread()
/// #     .enable_all()
/// #     .build()
/// #     .unwrap()
/// #     .block_on(async {
/// let mut builder = PulseContainer::builder();
/// builder.provide_arc_value(String::from("localhost")).unwrap();
/// builder.provide_arc_value(8080u16).unwrap();
///
/// // The async closure implements `AsyncFnDependency<(ArcValue<String>, ArcValue<u16>)>` automatically.
/// builder.provide_async(async |host: ArcValue<String>, port: ArcValue<u16>| Config { host: host.to_string(), port: *port }).unwrap();
///
/// let container = builder.build().await.unwrap();
///
/// let config = container.context().get_value::<Config>().unwrap();
/// assert_eq!(config.host, "localhost");
/// assert_eq!(config.port, 8080);
/// # });
/// ```
#[diagnostic::on_unimplemented(
    message = "The function must implement AsyncFnDependency<Dep> to be used an asynchronous dependency provider.",
    label = "Function does not implement AsyncFnDependency<Dep>",
    note = "For synchronous functions, use PulseContainerBuilder::provide instead.",
    note = "AsyncFnDependency<Dep> is implemented for functions that take parameters that implement FromPulseValue and return a Result<T, BuildDependencyError> where T is a PulseValue."
)]
pub trait AsyncFnDependency<Dep>: Send + Sync + 'static {
    /// The output value type produced by this dependency.
    type Value: PulseValue;

    /// Provide the dependency value from a [`PulseContext`] asynchronously.
    ///
    /// This method is called during container build time to produce the value.
    /// Arguments declared in the function signature are extracted from the context
    /// and passed to the underlying closure.
    fn provide_from_context(
        &self,
        context: PulseContext<'_>,
    ) -> impl Future<Output = Result<<Self::Value as PulseValue>::StorageType, PulseError>> + Send
    where
        Self: Sized;

    /// Return an iterator of this dependency's dependencies.
    ///
    /// The items describe the other dependencies that must be resolved before this one can run.
    fn dependencies() -> impl Iterator<Item = DependencyItem<PulseDependencyInfo>>;
}

macro_rules! impl_provide_async_fn {
    ($($name:ident,)*) => {
        impl<F, Fut, $($name,)* R> AsyncFnDependency<($( $name, )*)> for F
        where
            F: Fn($($name,)*) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = R> + Send,
            $($name: FromPulseValue,)*
            R: IntoPulseResult,
        {
            type Value = R::Value;

            #[allow(non_snake_case, unused_variables)]
            async fn provide_from_context(
                &self,
                context: PulseContext<'_>,
            ) -> Result<<Self::Value as PulseValue>::StorageType, PulseError> {
                let ($($name,)*) = ($(
                    value_utils::from_context::<$name>(context)
                        .ok_or_else(|| PulseError::DependencyNotProvided(PulseDependencyInfo::new::<$name::Value>()))?,
                )*);
                let value = self($($name,)*).await.into_pulse_result()?;
                Ok(R::Value::map_to_storage_type(value))
            }

            fn dependencies() -> impl Iterator<Item = DependencyItem<PulseDependencyInfo>> {
                let array: [Option<_>; _] = [
                    $( value_utils::dependency_info::<$name>(), )*
                ];
                array.into_iter().flatten()
            }
        }
    };
}

// Implement for functions with up to 26 parameters
seq!(N in 0..=26 {
    #(
        seq!(M in 0..N {
            impl_provide_async_fn!( #(T~M,)* );
        });
    )*
});
