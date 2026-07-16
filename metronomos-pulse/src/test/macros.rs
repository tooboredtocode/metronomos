macro_rules! make_test_value {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PulseValue)]
        #[allow(unused)]
        struct $name;
    };
}

macro_rules! make_sync_test_type {
    (($($deps:ty),*) => $name:ident) => {
        $crate::test::macros::make_test_value!($name);

        impl $name {
            #[allow(non_snake_case, unused)]
            fn new($( _: $deps ),*) -> Result<Self, $crate::error::BuildDependencyError> {
                Ok(Self)
            }
        }
    };
    (($( $field:ident: $ty:ty ),*) => $name:ident) => {
        #[derive(Debug, Clone, PulseValue)]
        #[allow(unused)]
        struct $name {
            $( $field: $ty ),*
        }

        impl $name {
            #[allow(non_snake_case, unused)]
            fn new($( $field: $ty ),*) -> Result<Self, $crate::error::BuildDependencyError> {
                Ok(Self { $( $field ),* })
            }
        }
    };
}

macro_rules! make_async_test_type {
    (($($deps:ty),*) => $name:ident) => {
        $crate::test::macros::make_test_value!($name);

        impl $name {
            #[allow(non_snake_case, unused)]
            async fn new($( _: $deps ),*) -> Result<Self, $crate::error::BuildDependencyError> {
                Ok(Self)
            }
        }
    };
    (($( $field:ident: $ty:ty ),*) => $name:ident) => {
        #[derive(Debug, Clone, PulseValue)]
        #[allow(unused)]
        struct $name {
            $( $field: $ty ),*
        }

        impl $name {
            #[allow(non_snake_case, unused)]
            async fn new($( $field: $ty ),*) -> Result<Self, $crate::error::BuildDependencyError> {
                Ok(Self { $( $field ),* })
            }
        }
    };
}

macro_rules! assert_provide_value {
    ($container:expr, $value:expr) => {
        match $container.provide_value($value) {
            Ok(_) => {}
            Err(e) => panic!("Failed to provide value for {}: {}", stringify!($value), e),
        }
    };
}

macro_rules! assert_provide_sync {
    ($container:expr, $type:ty) => {
        match $container.provide(<$type>::new) {
            Ok(_) => {}
            Err(e) => panic!("Failed to provide {}: {}", stringify!($type), e),
        }
    };
}

macro_rules! assert_provide_async {
    ($container:expr, $type:ty) => {
        match $container.provide_async(<$type>::new) {
            Ok(_) => {}
            Err(e) => panic!("Failed to provide {}: {}", stringify!($type), e),
        }
    };
}

macro_rules! assert_builds {
    ($container:expr) => {
        $container
            .build()
            .await
            .unwrap_or_else(|e| panic!("Failed to build container: {}", e))
    };
}

macro_rules! assert_not_builds {
    ($container:expr) => {
        $container
            .build()
            .await
            .map_or_else(|e| e, |_| panic!("Container should not build successfully"))
    };
}

macro_rules! assert_contains {
    ($container:expr, $type:ty) => {
        match $container.context().get_value::<$type>() {
            Some(val) => val,
            None => panic!("Container does not contain {}", stringify!($type)),
        }
    };
}

macro_rules! assert_not_contains {
    ($container:expr, $type:ty) => {
        if $container.context().get_value::<$type>().is_some() {
            panic!("Container should not contain {}", stringify!($type));
        }
    };
}

pub(super) use assert_builds;
pub(super) use assert_contains;
pub(super) use assert_not_builds;
pub(super) use assert_not_contains;
pub(super) use assert_provide_async;
pub(super) use assert_provide_sync;
pub(super) use assert_provide_value;
pub(super) use make_async_test_type;
pub(super) use make_sync_test_type;
pub(super) use make_test_value;

pub(super) use crate as metronomos_pulse;
pub(super) use crate::value::PulseValue;
