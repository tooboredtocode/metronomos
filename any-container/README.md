## Any Container

A Rust crate providing type-erased container types for storing heterogeneous values behind a common interface.

### Features

- `AnyMap` - A map from types to a single value of each type
- `AnyMultiMap` - A map from types to multiple values of each type
- `AnyVec` - A type-erased vector storing values of a single type
- `AnyCloneBox` - A type-erased box that allows cloning the boxed value

### Usage

```toml
[dependencies]
any-container = "0.1"
```

### License

ISC Licensed. See [LICENSE](../LICENSE) for details.
