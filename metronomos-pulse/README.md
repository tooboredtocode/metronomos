## Metronomos Pulse

A type-safe, compile-free dependency injection (DI) container for Rust inspired by [Uber's `dig`][dig] Go library.

`metronomos-pulse` provides a DI container that dynamically resolves dependencies during setup using compile-time optimizable function-based constructors. It is designed to be ergonomic, type-safe, and flexible.

### Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
metronomos-pulse = "0.1"
```

#### Example

```rust
use metronomos_pulse::{PulseContainer, value::PulseValue};

#[derive(PulseValue, Clone, Debug, PartialEq)]
struct Config(String);

#[derive(PulseValue, Clone, Debug, PartialEq)]
struct Database { config: Config }

impl Database {
    fn init(config: Config) -> Self {
        Database { config }
    }
}

let mut builder = PulseContainer::builder();
builder.provide_value(Config("postgres://localhost".into())).unwrap();
builder.provide(Database::init).unwrap();

let container = builder.build().await.unwrap();
let db = container.context().get_value::<Database>();
```

[dig]: https://pkg.go.dev/go.uber.org/dig

### License

ISC Licensed. See [LICENSE](../LICENSE) for details.
