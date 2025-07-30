# rs-statemachine with Feature Flags

A flexible and extensible state machine implementation for Rust with optional advanced features that can be enabled through Cargo feature flags.

## Features

The library provides a core state machine implementation with the following optional features:

| Feature | Description | Default |
|---------|-------------|---------|
| `history` | Track state transition history | ✓ |
| `extended` | Entry/exit actions for states | ✓ |
| `metrics` | Performance metrics collection | ✓ |
| `hierarchical` | Hierarchical state support | |
| `guards` | Guard conditions with priorities | |
| `timeout` | State timeout support | |
| `parallel` | Parallel state regions | |
| `visualization` | Export to DOT/PlantUML formats | |
| `serde` | Serialization support | |
| `async` | Async action support | |
| `full` | Enable all features | |

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# Basic installation with default features
rs-statemachine = "0.1"

# Or with specific features
rs-statemachine = { version = "0.1", features = ["history", "metrics", "visualization"] }

# Or with all features
rs-statemachine = { version = "0.1", features = ["full"] }

# Minimal installation (no features)
rs-statemachine = { version = "0.1", default-features = false }
```

## Basic Usage

```rust
use rs_statemachine::*;

// Define your states
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum MyState {
    Idle,
    Working,
    Done,
}
impl State for MyState {}

// Define your events
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum MyEvent {
    Start,
    Complete,
}
impl Event for MyEvent {}

// Define your context
#[derive(Debug, Clone)]
struct MyContext {
    task_id: String,
}
impl Context for MyContext {}

// Build the state machine
let mut builder = StateMachineBuilderFactory::create::<MyState, MyEvent, MyContext>();

builder
    .external_transition()
    .from(MyState::Idle)
    .to(MyState::Working)
    .on(MyEvent::Start)
    .perform(|_s, _e, ctx| {
        println!("Starting task {}", ctx.task_id);
    });

let state_machine = builder.build();
```

## Feature Examples

### History Tracking (`history` feature)

```rust
#[cfg(feature = "history")]
{
    // Execute transitions
    let _ = state_machine.fire_event(MyState::Idle, MyEvent::Start, context);
    
    // Get transition history
    let history = state_machine.get_history();
    for record in history {
        println!("{:?} -> {:?} at {:?}", record.from, record.to, record.timestamp);
    }
    
    // Clear history
    state_machine.clear_history();
}
```

### Entry/Exit Actions (`extended` feature)

```rust
#[cfg(feature = "extended")]
{
    let mut builder = StateMachineBuilderFactory::create();
    
    builder
        .with_entry_action(MyState::Working, |state, ctx| {
            println!("Entering {:?} for task {}", state, ctx.task_id);
        })
        .with_exit_action(MyState::Working, |state, ctx| {
            println!("Exiting {:?} for task {}", state, ctx.task_id);
        });
}
```

### Metrics Collection (`metrics` feature)

```rust
#[cfg(feature = "metrics")]
{
    // Execute multiple transitions
    // ...
    
    let metrics = state_machine.get_metrics();
    println!("Success rate: {:.2}%", metrics.success_rate() * 100.0);
    println!("Average transition time: {:?}", metrics.average_transition_time());
}
```

### Guard Priorities (`guards` feature)

```rust
#[cfg(feature = "guards")]
{
    builder
        .external_transition()
        .from(MyState::Idle)
        .to(MyState::Working)
        .on(MyEvent::Start)
        .when(|_s, _e, ctx| ctx.priority > 10)
        .with_priority(100)  // Higher priority transitions are evaluated first
        .perform(|_s, _e, _c| {
            println!("High priority task started");
        });
}
```

### Visualization (`visualization` feature)

```rust
#[cfg(feature = "visualization")]
{
    // Export to GraphViz DOT format
    let dot = state_machine.to_dot();
    std::fs::write("state_machine.dot", dot)?;
    
    // Export to PlantUML format
    let plantuml = state_machine.to_plantuml();
    std::fs::write("state_machine.puml", plantuml)?;
}
```

### Parallel Regions (`parallel` feature)

```rust
#[cfg(feature = "parallel")]
{
    let mut parallel_machine = ParallelStateMachine::new();
    parallel_machine.add_region(region1_machine);
    parallel_machine.add_region(region2_machine);
    
    // Fire event in all regions
    let results = parallel_machine.fire_event(
        vec![Region1State::Initial, Region2State::Initial],
        SharedEvent::Start,
        context,
    );
}
```

### Async Support (`async` feature)

```rust
#[cfg(feature = "async")]
{
    #[async_trait]
    impl AsyncAction<MyState, MyEvent, MyContext> for MyAsyncAction {
        async fn execute(&self, from: &MyState, event: &MyEvent, context: &MyContext) {
            // Perform async operations
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Async action completed");
        }
    }
    
    // Fire event asynchronously
    let result = state_machine.fire_event_async(
        MyState::Idle,
        MyEvent::Start,
        context
    ).await?;
}
```

## Performance Considerations

- **Minimal Core**: The core state machine has minimal overhead when features are disabled
- **Feature Cost**: Each feature adds some memory and computational overhead:
    - `history`: Stores transition records in memory
    - `metrics`: Tracks timing and counters
    - `extended`: Adds hashmap lookups for entry/exit actions
    - `guards`: Adds sorting step for priority evaluation

## Building Without Default Features

To use only the core state machine without any additional features:

```toml
[dependencies]
state-machine = { version = "0.1", default-features = false }
```

Then selectively enable only the features you need:

```toml
[dependencies]
state-machine = { version = "0.1", default-features = false, features = ["visualization"] }
```

## Feature Combinations

Some features work well together:

- `history` + `metrics`: Complete audit trail with performance data
- `extended` + `guards`: Complex state logic with entry/exit actions
- `visualization` + any: Visualize your state machine configuration
- `async` + `timeout`: Handle long-running operations with timeouts

## Migration Guide

If you're migrating from a version without feature flags:

1. The core API remains unchanged
2. Advanced features that were previously in separate modules are now behind feature flags
3. Default features (`history`, `extended`, `metrics`) provide common functionality
4. Use `features = ["full"]` to enable everything

## Contributing

When adding new features:

1. Add a new feature flag in `Cargo.toml`
2. Gate the implementation with `#[cfg(feature = "your_feature")]`
3. Update documentation and examples
4. Ensure the core functionality works without your feature

## License

This project is licensed under the MIT License - see the LICENSE file for details.