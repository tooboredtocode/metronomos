use std::any::type_name;
use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use metronomos_loom::dependency::DependencyKeyKind;

use crate::value::{PulseValue, PulseValueSealed};

/// A wrapper around `Arc<T>` that implements `PulseValue`.
///
/// `ArcValue` wraps a value in an `Arc`, allowing cheap cloning of the stored value across
/// multiple container locations. It is useful when you want to share expensive-to-construct
/// values throughout the dependency graph without recreating them.
#[repr(transparent)]
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArcValue<T: ?Sized + Send + Sync + 'static> {
    inner: Arc<T>,
}

impl<T: ?Sized + Send + Sync + 'static> ArcValue<T> {
    /// Creates a new `ArcValue` wrapping the given value in an `Arc`.
    ///
    /// The wrapped value will be shared among all clones of this `ArcValue`.
    pub fn new(value: impl Into<Arc<T>>) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

impl<T: ?Sized + Send + Sync + 'static> AsRef<T> for ArcValue<T> {
    fn as_ref(&self) -> &T {
        self.inner.as_ref()
    }
}

impl<T: ?Sized + Send + Sync + 'static> Borrow<T> for ArcValue<T> {
    fn borrow(&self) -> &T {
        self.inner.borrow()
    }
}

impl<T: ?Sized + Send + Sync + 'static> Deref for ArcValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: ?Sized + Send + Sync + 'static> Clone for ArcValue<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> fmt::Debug for ArcValue<T>
where
    T: fmt::Debug + ?Sized + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArcValue")
            .field(&self.inner.deref())
            .finish()
    }
}

impl<T> fmt::Display for ArcValue<T>
where
    T: fmt::Display + ?Sized + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl<T: ?Sized + Send + Sync + 'static> PulseValueSealed for ArcValue<T> {}
impl<T: ?Sized + Send + Sync + 'static> PulseValue for ArcValue<T> {
    const KIND: DependencyKeyKind = DependencyKeyKind::Unique;
    type StorageType = Self;

    type GetValueType<'a> = Option<&'a Self>;

    fn name() -> &'static str {
        let name = type_name::<Self>();
        if let Some(stripped) = name.strip_prefix("metronomos_pulse::value::arc_value::") {
            stripped
        } else {
            name
        }
    }

    fn map_to_storage_type(value: Self) -> Self::StorageType {
        value
    }

    fn get_value_type(context: crate::container::PulseContext<'_>) -> Self::GetValueType<'_> {
        context.inner_get_value::<Self>()
    }
}
