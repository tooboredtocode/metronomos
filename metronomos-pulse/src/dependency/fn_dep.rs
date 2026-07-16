use any_container::AnyCloneBox;
use metronomos_loom::dependency::DependencyItem;
use seq_macro::seq;

use crate::container::PulseContext;
use crate::dependency::PulseDependencyInfo;
use crate::dependency::util::IntoPulseResult;
use crate::error::PulseError;
use crate::value::{FromPulseValue, PulseValue, utils as value_utils};

type ErasedFnDepResult = Result<AnyCloneBox, PulseError>;

/// A type-erased synchronous dependency provider.
pub(super) trait ErasedFnDep: Send + Sync {
    /// Provide the dependency value from a [`PulseContext`].
    fn provide(&self, context: PulseContext<'_>) -> ErasedFnDepResult;
}

struct MakeErasedFnDep<F> {
    f: F,
}

impl<F> ErasedFnDep for MakeErasedFnDep<F>
where
    F: Fn(PulseContext<'_>) -> ErasedFnDepResult + Send + Sync + 'static,
{
    fn provide(&self, context: PulseContext<'_>) -> ErasedFnDepResult {
        (self.f)(context)
    }
}

pub(super) fn erase_fn_dep<F, Dep>(fn_dep: F) -> Box<dyn ErasedFnDep>
where
    F: FnDependency<Dep>,
{
    let fun =
        move |context: PulseContext<'_>| fn_dep.provide_from_context(context).map(AnyCloneBox::new);

    Box::new(MakeErasedFnDep { f: fun })
}

/// Trait for synchronous dependency providers.
///
/// Implementing this trait enables a value to be registered as a dependency in the container
/// via [`PulseContainerBuilder::provide`](crate::builder::PulseContainerBuilder::provide).
///
/// # Automatically Implemented
///
/// This trait is automatically implemented for any function `Fn(Arg1, Arg2, ..., N) -> R` where:
///
/// - Each argument `Arg1..N` implements [`FromPulseValue`](crate::value::FromPulseValue)
/// - The return type `R` is either the value directly or `Result<T, BuildDependencyError>`
///
/// The arguments are resolved from the container at build time. The function's return type
/// becomes the dependency's output value.
///
/// # Example
///
/// ```
/// use metronomos_pulse::PulseContainer;
/// use metronomos_pulse::value::{ArcValue, PulseValue};
///
/// #[derive(PulseValue, Clone)]
/// struct Greeting {
///     message: String,
/// }
///
/// # tokio::runtime::Builder::new_current_thread()
/// #     .enable_all()
/// #     .build()
/// #     .unwrap()
/// #     .block_on(async {
/// let mut builder = PulseContainer::builder();
/// builder.provide_arc_value(String::from("Hello!")).unwrap();
///
/// // The closure implements `FnDependency<(ArcValue<String>)>` automatically.
/// builder.provide(|message: ArcValue<String>| Greeting { message: message.to_string() }).unwrap();
///
/// let container = builder.build().await.unwrap();
///
/// let greeting = container.context().get_value::<Greeting>().unwrap();
/// assert_eq!(greeting.message, "Hello!");
/// # });
/// ```
#[diagnostic::on_unimplemented(
    message = "The function must implement FnDependency<Dep> to be used as a synchronous dependency provider.",
    label = "Function does not implement FnDependency<Dep>",
    note = "For asynchronous functions, use PulseContainerBuilder::provide_async instead.",
    note = "FnDependency<Dep> is implemented for functions that take parameters that implement FromPulseValue and return a Result<T, BuildDependencyError> where T is a PulseValue."
)]
pub trait FnDependency<Dep>: Send + Sync + 'static {
    /// The output value type produced by this dependency.
    type Value: PulseValue;

    /// Provide the dependency value from a [`PulseContext`].
    ///
    /// This method is called during container build time to produce the value.
    /// Arguments declared in the function signature are extracted from the context
    /// and passed to the underlying function.
    fn provide_from_context(
        &self,
        context: PulseContext<'_>,
    ) -> Result<<Self::Value as PulseValue>::StorageType, PulseError>
    where
        Self: Sized;

    /// Return an iterator of this dependency's dependencies.
    ///
    /// The items describe the other dependencies that must be resolved before this one can run.
    fn dependencies() -> impl Iterator<Item = DependencyItem<PulseDependencyInfo>>;
}

macro_rules! impl_provide_fn {
    ($($name:ident,)*) => {
        impl<F, $($name,)* R> FnDependency<($( $name, )*)> for F
        where
            F: Fn($($name,)*) -> R + Send + Sync + 'static,
            $($name: FromPulseValue,)*
            R: IntoPulseResult,
        {
            type Value = R::Value;

            #[allow(non_snake_case, unused_variables)]
            fn provide_from_context(&self, context: PulseContext<'_>) -> Result<<Self::Value as PulseValue>::StorageType, PulseError> {
                let ($($name,)*) = ($(
                    value_utils::from_context::<$name>(context)
                        .ok_or_else(|| PulseError::DependencyNotProvided(PulseDependencyInfo::new::<$name::Value>()))?,
                )*);
                let value = self($($name,)*).into_pulse_result()?;
                Ok(R::Value::map_to_storage_type(value))
            }

            fn dependencies() -> impl Iterator<Item = DependencyItem<PulseDependencyInfo>> {
                let array: [Option<_>; _] = [
                    $( value_utils::dependency_info::<$name>(), )*
                ];
                array.into_iter().flatten()
            }
        }
    }
}

// Implement for functions with up to 26 parameters
seq!(N in 0..=26 {
    #(
        seq!(M in 0..N {
            impl_provide_fn!( #(T~M,)* );
        });
    )*
});
