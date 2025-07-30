//! Examples demonstrating different features of the state machine

use rs_statemachine::*;
use std::sync::Arc;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum OrderState {
    New,
    PaymentPending,
    PaymentReceived,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
}

impl State for OrderState {}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum OrderEvent {
    Pay,
    ConfirmPayment,
    Process,
    Ship,
    Deliver,
    Cancel,
    Refund,
}

impl Event for OrderEvent {}

#[derive(Debug, Clone)]
struct OrderContext {
    order_id: String,
    amount: f64,
    customer_id: String,
}

impl Context for OrderContext {}

/// Example 1: Basic state machine (no features required)
fn basic_example() {
    println!("=== Basic State Machine Example ===");

    let mut builder = StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();

    builder
        .external_transition()
        .from(OrderState::New)
        .to(OrderState::PaymentPending)
        .on(OrderEvent::Pay)
        .perform(|_s, _e, ctx| {
            println!("Order {} payment initiated", ctx.order_id);
        });

    builder
        .external_transition()
        .from(OrderState::PaymentPending)
        .to(OrderState::PaymentReceived)
        .on(OrderEvent::ConfirmPayment)
        .perform(|_s, _e, ctx| {
            println!("Payment confirmed for order {}", ctx.order_id);
        });

    let state_machine = builder.id("BasicOrderMachine").build();

    let context = OrderContext {
        order_id: "ORD-001".to_string(),
        amount: 99.99,
        customer_id: "CUST-123".to_string(),
    };

    let result = state_machine.fire_event(OrderState::New, OrderEvent::Pay, context.clone());
    println!("Transition result: {:?}", result);
}

/// Example 2: With history tracking (requires 'history' feature)
#[cfg(feature = "history")]
fn history_example() {
    println!("\n=== History Tracking Example ===");

    let mut builder = StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();

    builder
        .external_transition()
        .from(OrderState::New)
        .to(OrderState::PaymentPending)
        .on(OrderEvent::Pay)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::PaymentPending)
        .to(OrderState::PaymentReceived)
        .on(OrderEvent::ConfirmPayment)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::PaymentReceived)
        .to(OrderState::Processing)
        .on(OrderEvent::Process)
        .perform(|_s, _e, _c| {});

    let state_machine = builder.id("HistoryOrderMachine").build();

    let context = OrderContext {
        order_id: "ORD-002".to_string(),
        amount: 149.99,
        customer_id: "CUST-456".to_string(),
    };

    // Execute multiple transitions
    let _ = state_machine.fire_event(OrderState::New, OrderEvent::Pay, context.clone());
    let _ = state_machine.fire_event(
        OrderState::PaymentPending,
        OrderEvent::ConfirmPayment,
        context.clone(),
    );
    let _ = state_machine.fire_event(OrderState::PaymentReceived, OrderEvent::Process, context);

    // Check history
    let history = state_machine.get_history();
    println!("Transition history:");
    for (i, record) in history.iter().enumerate() {
        println!(
            "  {}. {:?} -> {:?} via {:?} (success: {})",
            i + 1,
            record.from,
            record.to,
            record.event,
            record.success
        );
    }
}

/// Example 3: With entry/exit actions (requires 'extended' feature)
#[cfg(feature = "extended")]
fn extended_example() {
    println!("\n=== Extended State Machine Example ===");

    let mut builder = StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();

    builder
        .with_entry_action(OrderState::Processing, |state, ctx| {
            println!(
                "ENTRY: Starting to process order {} in state {:?}",
                ctx.order_id, state
            );
        })
        .with_exit_action(OrderState::Processing, |state, ctx| {
            println!(
                "EXIT: Finished processing order {} from state {:?}",
                ctx.order_id, state
            );
        })
        .with_entry_action(OrderState::Shipped, |_state, ctx| {
            println!("ENTRY: Order {} has been shipped!", ctx.order_id);
        });

    builder
        .external_transition()
        .from(OrderState::PaymentReceived)
        .to(OrderState::Processing)
        .on(OrderEvent::Process)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::Processing)
        .to(OrderState::Shipped)
        .on(OrderEvent::Ship)
        .perform(|_s, _e, _c| {});

    let state_machine = builder.id("ExtendedOrderMachine").build();

    let context = OrderContext {
        order_id: "ORD-003".to_string(),
        amount: 299.99,
        customer_id: "CUST-789".to_string(),
    };

    let _ = state_machine.fire_event(
        OrderState::PaymentReceived,
        OrderEvent::Process,
        context.clone(),
    );
    let _ = state_machine.fire_event(OrderState::Processing, OrderEvent::Ship, context);
}

/// Example 4: With metrics (requires 'metrics' feature)
#[cfg(feature = "metrics")]
fn metrics_example() {
    println!("\n=== Metrics Collection Example ===");

    let mut builder = StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();

    // Add multiple transitions
    builder
        .external_transition()
        .from(OrderState::New)
        .to(OrderState::PaymentPending)
        .on(OrderEvent::Pay)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::PaymentPending)
        .to(OrderState::PaymentReceived)
        .on(OrderEvent::ConfirmPayment)
        .when(|_s, _e, ctx| ctx.amount > 0.0)
        .perform(|_s, _e, _c| {});

    builder
        .external_transitions()
        .from_among(vec![
            OrderState::New,
            OrderState::PaymentPending,
            OrderState::Processing,
        ])
        .to(OrderState::Cancelled)
        .on(OrderEvent::Cancel)
        .perform(|_s, _e, _c| {});

    let state_machine = builder.id("MetricsOrderMachine").build();

    // Simulate multiple orders
    for i in 0..10 {
        let context = OrderContext {
            order_id: format!("ORD-{:03}", i),
            amount: (i as f64) * 10.0,
            customer_id: format!("CUST-{:03}", i),
        };

        let _ = state_machine.fire_event(OrderState::New, OrderEvent::Pay, context.clone());

        if i % 3 == 0 {
            // Some orders get cancelled
            let _ =
                state_machine.fire_event(OrderState::PaymentPending, OrderEvent::Cancel, context);
        } else {
            // Others proceed normally
            let _ = state_machine.fire_event(
                OrderState::PaymentPending,
                OrderEvent::ConfirmPayment,
                context,
            );
        }
    }

    // Get and display metrics
    let metrics = state_machine.get_metrics();
    println!("State Machine Metrics:");
    println!("  Total transitions: {}", metrics.total_transitions);
    println!("  Successful: {}", metrics.successful_transitions);
    println!("  Failed: {}", metrics.failed_transitions);
    println!("  Success rate: {:.2}%", metrics.success_rate() * 100.0);

    if let Some(avg_time) = metrics.average_transition_time() {
        println!("  Average transition time: {:?}", avg_time);
    }

    println!("  State visit counts:");
    for (state, count) in &metrics.state_visit_counts {
        println!("    {}: {}", state, count);
    }
}

/// Example 5: With guard priorities (requires 'guards' feature)
#[cfg(feature = "guards")]
fn guards_example() {
    println!("\n=== Guard Priorities Example ===");

    let mut builder = StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();

    // Multiple transitions with different priorities
    builder
        .external_transition()
        .from(OrderState::PaymentReceived)
        .to(OrderState::Processing)
        .on(OrderEvent::Process)
        .when(|_s, _e, ctx| ctx.amount < 100.0)
        .with_priority(10)
        .perform(|_s, _e, ctx| {
            println!(
                "Processing small order {} (amount: {})",
                ctx.order_id, ctx.amount
            );
        });

    builder
        .external_transition()
        .from(OrderState::PaymentReceived)
        .to(OrderState::Processing)
        .on(OrderEvent::Process)
        .when(|_s, _e, ctx| ctx.amount >= 100.0 && ctx.amount < 1000.0)
        .with_priority(20)
        .perform(|_s, _e, ctx| {
            println!(
                "Processing medium order {} (amount: {})",
                ctx.order_id, ctx.amount
            );
        });

    builder
        .external_transition()
        .from(OrderState::PaymentReceived)
        .to(OrderState::Processing)
        .on(OrderEvent::Process)
        .when(|_s, _e, ctx| ctx.amount >= 1000.0)
        .with_priority(30)
        .perform(|_s, _e, ctx| {
            println!(
                "Processing large order {} (amount: {}) - Priority handling!",
                ctx.order_id, ctx.amount
            );
        });

    let state_machine = builder.id("GuardsOrderMachine").build();

    // Test with different order amounts
    let contexts = vec![
        OrderContext {
            order_id: "ORD-SMALL".to_string(),
            amount: 50.0,
            customer_id: "C1".to_string(),
        },
        OrderContext {
            order_id: "ORD-MEDIUM".to_string(),
            amount: 500.0,
            customer_id: "C2".to_string(),
        },
        OrderContext {
            order_id: "ORD-LARGE".to_string(),
            amount: 5000.0,
            customer_id: "C3".to_string(),
        },
    ];

    for context in contexts {
        let _ = state_machine.fire_event(OrderState::PaymentReceived, OrderEvent::Process, context);
    }
}

/// Example 6: With visualization (requires 'visualization' feature)
#[cfg(feature = "visualization")]
fn visualization_example() {
    println!("\n=== Visualization Example ===");

    let mut builder = StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();

    // Build a complete order flow
    builder
        .external_transition()
        .from(OrderState::New)
        .to(OrderState::PaymentPending)
        .on(OrderEvent::Pay)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::PaymentPending)
        .to(OrderState::PaymentReceived)
        .on(OrderEvent::ConfirmPayment)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::PaymentReceived)
        .to(OrderState::Processing)
        .on(OrderEvent::Process)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::Processing)
        .to(OrderState::Shipped)
        .on(OrderEvent::Ship)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::Shipped)
        .to(OrderState::Delivered)
        .on(OrderEvent::Deliver)
        .perform(|_s, _e, _c| {});

    builder
        .external_transitions()
        .from_among(vec![
            OrderState::New,
            OrderState::PaymentPending,
            OrderState::Processing,
        ])
        .to(OrderState::Cancelled)
        .on(OrderEvent::Cancel)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::Cancelled)
        .to(OrderState::Refunded)
        .on(OrderEvent::Refund)
        .perform(|_s, _e, _c| {});

    let state_machine = builder.id("VisualOrderMachine").build();

    println!("DOT Format:");
    println!("{}", state_machine.to_dot());

    println!("\nPlantUML Format:");
    println!("{}", state_machine.to_plantuml());
}

/// Example 7: With parallel regions (requires 'parallel' feature)
#[cfg(feature = "parallel")]
fn parallel_example() {
    println!("\n=== Parallel Regions Example ===");

    // Order processing region
    let mut order_builder =
        StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();
    order_builder
        .external_transition()
        .from(OrderState::New)
        .to(OrderState::Processing)
        .on(OrderEvent::Process)
        .perform(|_s, _e, ctx| {
            println!("Order region: Processing order {}", ctx.order_id);
        });

    // Payment processing region (using same states/events for simplicity)
    let mut payment_builder =
        StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();
    payment_builder
        .external_transition()
        .from(OrderState::PaymentPending)
        .to(OrderState::PaymentReceived)
        .on(OrderEvent::ConfirmPayment)
        .perform(|_s, _e, ctx| {
            println!(
                "Payment region: Payment confirmed for order {}",
                ctx.order_id
            );
        });

    let mut parallel_machine = ParallelStateMachine::new();
    parallel_machine.add_region(order_builder.id("OrderRegion").build());
    parallel_machine.add_region(payment_builder.id("PaymentRegion").build());

    let context = OrderContext {
        order_id: "ORD-PARALLEL".to_string(),
        amount: 199.99,
        customer_id: "CUST-P1".to_string(),
    };

    // Fire events in parallel regions
    println!("Firing Process event in parallel regions:");
    let results = parallel_machine.fire_event(
        vec![OrderState::New, OrderState::PaymentPending],
        OrderEvent::Process,
        context.clone(),
    );

    for (i, result) in results.iter().enumerate() {
        println!("  Region {}: {:?}", i, result);
    }

    println!("Firing ConfirmPayment event in parallel regions:");
    let results = parallel_machine.fire_event(
        vec![OrderState::Processing, OrderState::PaymentPending],
        OrderEvent::ConfirmPayment,
        context,
    );

    for (i, result) in results.iter().enumerate() {
        println!("  Region {}: {:?}", i, result);
    }
}

/// Example 8: Complete example with multiple features
#[cfg(all(feature = "history", feature = "metrics", feature = "extended"))]
fn complete_example() {
    println!("\n=== Complete Example with Multiple Features ===");

    let mut builder = StateMachineBuilderFactory::create::<OrderState, OrderEvent, OrderContext>();

    // Configure entry/exit actions
    builder
        .with_entry_action(OrderState::Processing, |_s, ctx| {
            println!("[ENTRY] Starting to process order {}", ctx.order_id);
        })
        .with_exit_action(OrderState::Processing, |_s, ctx| {
            println!("[EXIT] Finished processing order {}", ctx.order_id);
        });

    // Build transitions
    builder
        .external_transition()
        .from(OrderState::New)
        .to(OrderState::PaymentPending)
        .on(OrderEvent::Pay)
        .perform(|_s, _e, ctx| {
            println!("Payment initiated for ${}", ctx.amount);
        });

    builder
        .external_transition()
        .from(OrderState::PaymentPending)
        .to(OrderState::PaymentReceived)
        .on(OrderEvent::ConfirmPayment)
        .when(|_s, _e, ctx| ctx.amount > 0.0)
        .perform(|_s, _e, ctx| {
            println!("Payment confirmed: ${}", ctx.amount);
        });

    builder
        .external_transition()
        .from(OrderState::PaymentReceived)
        .to(OrderState::Processing)
        .on(OrderEvent::Process)
        .perform(|_s, _e, _c| {});

    builder
        .external_transition()
        .from(OrderState::Processing)
        .to(OrderState::Shipped)
        .on(OrderEvent::Ship)
        .perform(|_s, _e, ctx| {
            println!("Order {} shipped", ctx.order_id);
        });

    builder.set_fail_callback(Arc::new(|state, event, ctx| {
        println!(
            "FAILED: Cannot handle {:?} in state {:?} for order {}",
            event, state, ctx.order_id
        );
    }));

    let state_machine = builder.id("CompleteOrderMachine").build();

    // Process an order through the complete flow
    let context = OrderContext {
        order_id: "ORD-COMPLETE".to_string(),
        amount: 999.99,
        customer_id: "CUST-VIP".to_string(),
    };

    println!("\nProcessing order through complete flow:");
    let states = [
        (OrderState::New, OrderEvent::Pay),
        (OrderState::PaymentPending, OrderEvent::ConfirmPayment),
        (OrderState::PaymentReceived, OrderEvent::Process),
        (OrderState::Processing, OrderEvent::Ship),
    ];

    for (state, event) in &states {
        let result = state_machine.fire_event(state.clone(), event.clone(), context.clone());
        println!("  {:?} + {:?} = {:?}", state, event, result);
    }

    // Try an invalid transition
    println!("\nTrying invalid transition:");
    let _ = state_machine.fire_event(OrderState::Shipped, OrderEvent::Pay, context);

    // Display collected data
    println!("\nHistory:");
    for record in state_machine.get_history() {
        println!(
            "  {:?} -> {:?} (success: {})",
            record.from, record.to, record.success
        );
    }

    println!("\nMetrics:");
    let metrics = state_machine.get_metrics();
    println!(
        "  Total: {}, Success: {}, Failed: {}",
        metrics.total_transitions, metrics.successful_transitions, metrics.failed_transitions
    );
}

fn main() {
    basic_example();

    #[cfg(feature = "history")]
    history_example();

    #[cfg(feature = "extended")]
    extended_example();

    #[cfg(feature = "metrics")]
    metrics_example();

    #[cfg(feature = "guards")]
    guards_example();

    #[cfg(feature = "visualization")]
    visualization_example();

    #[cfg(feature = "parallel")]
    parallel_example();

    #[cfg(all(feature = "history", feature = "metrics", feature = "extended"))]
    complete_example();
}
