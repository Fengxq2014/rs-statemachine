//! Real-world example: Traffic Light Control System
//!
//! This example demonstrates how to use different feature combinations
//! for a practical application.

use rs_statemachine::*;
use std::sync::Arc;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum TrafficLightState {
    Red,
    Yellow,
    Green,
    FlashingYellow, // For maintenance or low-traffic periods
    Emergency,      // For emergency vehicle passage
}

impl State for TrafficLightState {}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum TrafficLightEvent {
    Timer,
    EmergencyVehicleDetected,
    EmergencyCleared,
    MaintenanceMode,
    NormalMode,
    PedestrianRequest,
}

impl Event for TrafficLightEvent {}

#[derive(Debug, Clone)]
struct TrafficContext {
    intersection_id: String,
    traffic_density: f32, // 0.0 to 1.0
    pedestrian_waiting: bool,
    emergency_active: bool,
    time_in_state: std::time::Duration,
}

impl Context for TrafficContext {}

/// Build a traffic light system with configurable features
pub fn build_traffic_light_system(
) -> StateMachine<TrafficLightState, TrafficLightEvent, TrafficContext> {
    let mut builder =
        StateMachineBuilderFactory::create::<TrafficLightState, TrafficLightEvent, TrafficContext>(
        );

    // Configure the state machine based on available features
    configure_basic_transitions(&mut builder);

    #[cfg(feature = "extended")]
    configure_entry_exit_actions(&mut builder);

    #[cfg(feature = "guards")]
    configure_priority_transitions(&mut builder);

    #[cfg(feature = "timeout")]
    configure_timeouts(&mut builder);

    builder.set_fail_callback(Arc::new(|state, event, ctx| {
        eprintln!(
            "WARNING: Invalid transition from {:?} with {:?} at intersection {}",
            state, event, ctx.intersection_id
        );
    }));
    builder.id("TrafficLightController").build()
}

/// Configure basic state transitions
fn configure_basic_transitions<'a>(
    builder: &'a mut StateMachineBuilder<TrafficLightState, TrafficLightEvent, TrafficContext>,
) -> &'a mut StateMachineBuilder<TrafficLightState, TrafficLightEvent, TrafficContext> {
    // Normal traffic light cycle
    builder
        .external_transition()
        .from(TrafficLightState::Green)
        .to(TrafficLightState::Yellow)
        .on(TrafficLightEvent::Timer)
        .perform(|_s, _e, ctx| {
            println!("[{}] Changing to YELLOW", ctx.intersection_id);
        });

    builder
        .external_transition()
        .from(TrafficLightState::Yellow)
        .to(TrafficLightState::Red)
        .on(TrafficLightEvent::Timer)
        .perform(|_s, _e, ctx| {
            println!("[{}] Changing to RED", ctx.intersection_id);
        });

    builder
        .external_transition()
        .from(TrafficLightState::Red)
        .to(TrafficLightState::Green)
        .on(TrafficLightEvent::Timer)
        .when(|_s, _e, ctx| !ctx.emergency_active)
        .perform(|_s, _e, ctx| {
            println!("[{}] Changing to GREEN", ctx.intersection_id);
        });

    // Emergency vehicle handling
    builder
        .external_transitions()
        .from_among(vec![
            TrafficLightState::Green,
            TrafficLightState::Yellow,
            TrafficLightState::Red,
        ])
        .to(TrafficLightState::Emergency)
        .on(TrafficLightEvent::EmergencyVehicleDetected)
        .perform(|from, _e, ctx| {
            println!(
                "[{}] EMERGENCY MODE! Was in {:?}",
                ctx.intersection_id, from
            );
        });

    builder
        .external_transition()
        .from(TrafficLightState::Emergency)
        .to(TrafficLightState::Red)
        .on(TrafficLightEvent::EmergencyCleared)
        .perform(|_s, _e, ctx| {
            println!(
                "[{}] Emergency cleared, returning to RED",
                ctx.intersection_id
            );
        });

    // Maintenance mode
    builder
        .external_transitions()
        .from_among(vec![
            TrafficLightState::Green,
            TrafficLightState::Yellow,
            TrafficLightState::Red,
        ])
        .to(TrafficLightState::FlashingYellow)
        .on(TrafficLightEvent::MaintenanceMode)
        .perform(|_s, _e, ctx| {
            println!(
                "[{}] Entering maintenance mode - FLASHING YELLOW",
                ctx.intersection_id
            );
        });

    builder
        .external_transition()
        .from(TrafficLightState::FlashingYellow)
        .to(TrafficLightState::Red)
        .on(TrafficLightEvent::NormalMode)
        .perform(|_s, _e, ctx| {
            println!("[{}] Exiting maintenance mode", ctx.intersection_id);
        });

    builder
}

/// Configure entry and exit actions for states
#[cfg(feature = "extended")]
fn configure_entry_exit_actions(
    builder: &mut StateMachineBuilder<TrafficLightState, TrafficLightEvent, TrafficContext>,
) {
    // Entry actions
    builder.with_entry_action(TrafficLightState::Green, |_state, ctx| {
        println!(
            "[{}] GREEN light ON - Vehicles may proceed",
            ctx.intersection_id
        );
        // In a real system, this would control the actual light hardware
    });

    builder.with_entry_action(TrafficLightState::Yellow, |_state, ctx| {
        println!(
            "[{}] YELLOW light ON - Prepare to stop",
            ctx.intersection_id
        );
    });

    builder.with_entry_action(TrafficLightState::Red, |_state, ctx| {
        println!(
            "[{}] RED light ON - Vehicles must stop",
            ctx.intersection_id
        );
        if ctx.pedestrian_waiting {
            println!(
                "[{}] Pedestrian crossing signal activated",
                ctx.intersection_id
            );
        }
    });

    builder.with_entry_action(TrafficLightState::Emergency, |_state, ctx| {
        println!(
            "[{}] EMERGENCY MODE - All lights RED except emergency route",
            ctx.intersection_id
        );
        // Would trigger emergency protocols in real system
    });

    // Exit actions
    builder.with_exit_action(TrafficLightState::Green, |_state, ctx| {
        println!("[{}] GREEN light OFF", ctx.intersection_id);
    });

    builder.with_exit_action(TrafficLightState::Emergency, |_state, ctx| {
        println!("[{}] Exiting emergency mode", ctx.intersection_id);
    });
}

/// Configure priority-based transitions for complex scenarios
#[cfg(feature = "guards")]
fn configure_priority_transitions(
    builder: &mut StateMachineBuilder<TrafficLightState, TrafficLightEvent, TrafficContext>,
) {
    // High-priority pedestrian crossing during low traffic
    builder
        .external_transition()
        .from(TrafficLightState::Green)
        .to(TrafficLightState::Yellow)
        .on(TrafficLightEvent::PedestrianRequest)
        .when(|_s, _e, ctx| ctx.traffic_density < 0.3 && ctx.pedestrian_waiting)
        .with_priority(100)
        .perform(|_s, _e, ctx| {
            println!(
                "[{}] Pedestrian priority - changing to yellow",
                ctx.intersection_id
            );
        });

    // Normal pedestrian request
    builder
        .external_transition()
        .from(TrafficLightState::Green)
        .to(TrafficLightState::Yellow)
        .on(TrafficLightEvent::PedestrianRequest)
        .when(|_s, _e, ctx| {
            ctx.pedestrian_waiting && ctx.time_in_state > std::time::Duration::from_secs(10)
        })
        .with_priority(50)
        .perform(|_s, _e, ctx| {
            println!("[{}] Pedestrian request accepted", ctx.intersection_id);
        });

    // Rush hour handling - extend green time
    builder
        .internal_transition()
        .within(TrafficLightState::Green)
        .on(TrafficLightEvent::Timer)
        .when(|_s, _e, ctx| {
            ctx.traffic_density > 0.8 && ctx.time_in_state < std::time::Duration::from_secs(90)
        })
        .with_priority(200)
        .perform(|_s, _e, ctx| {
            println!(
                "[{}] High traffic - extending green phase",
                ctx.intersection_id
            );
        });
}

/// Configure timeouts for safety
#[cfg(feature = "timeout")]
fn configure_timeouts(
    builder: &mut StateMachineBuilder<TrafficLightState, TrafficLightEvent, TrafficContext>,
) {
    use std::time::Duration;

    // Safety timeout - Yellow shouldn't last too long
    builder.with_state_timeout(
        TrafficLightState::Yellow,
        Duration::from_secs(5),
        TrafficLightState::Red,
        TrafficLightEvent::Timer,
    );

    // Emergency timeout - Auto-clear if no manual clear
    builder.with_state_timeout(
        TrafficLightState::Emergency,
        Duration::from_secs(300), // 5 minutes
        TrafficLightState::Red,
        TrafficLightEvent::EmergencyCleared,
    );
}

/// Simulate the traffic light system
pub fn simulate_traffic_light_system() {
    println!("=== Traffic Light Control System Demo ===\n");

    let state_machine = build_traffic_light_system();

    let mut context = TrafficContext {
        intersection_id: "Main-St-First-Ave".to_string(),
        traffic_density: 0.5,
        pedestrian_waiting: false,
        emergency_active: false,
        time_in_state: std::time::Duration::from_secs(0),
    };

    // Normal cycle
    println!("--- Normal Traffic Cycle ---");
    let states_and_events = vec![
        (TrafficLightState::Green, TrafficLightEvent::Timer),
        (TrafficLightState::Yellow, TrafficLightEvent::Timer),
        (TrafficLightState::Red, TrafficLightEvent::Timer),
    ];

    for (state, event) in states_and_events {
        match state_machine.fire_event(state, event, context.clone()) {
            Ok(new_state) => {
                println!("  -> Now in {:?} state\n", new_state);
            }
            Err(e) => {
                eprintln!("  ERROR: {}\n", e);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // Emergency vehicle scenario
    println!("--- Emergency Vehicle Detected ---");
    context.emergency_active = true;

    match state_machine.fire_event(
        TrafficLightState::Green,
        TrafficLightEvent::EmergencyVehicleDetected,
        context.clone(),
    ) {
        Ok(new_state) => {
            println!("  -> Now in {:?} state\n", new_state);

            // Clear emergency after some time
            std::thread::sleep(std::time::Duration::from_secs(2));
            context.emergency_active = false;

            match state_machine.fire_event(
                new_state,
                TrafficLightEvent::EmergencyCleared,
                context.clone(),
            ) {
                Ok(cleared_state) => {
                    println!("  -> Emergency cleared, now in {:?} state\n", cleared_state);
                }
                Err(e) => eprintln!("  ERROR clearing emergency: {}\n", e),
            }
        }
        Err(e) => eprintln!("  ERROR handling emergency: {}\n", e),
    }

    // Feature-specific demonstrations
    #[cfg(feature = "history")]
    demonstrate_history(&state_machine);

    #[cfg(feature = "metrics")]
    demonstrate_metrics(&state_machine);

    #[cfg(feature = "visualization")]
    demonstrate_visualization(&state_machine);
}

#[cfg(feature = "history")]
fn demonstrate_history(
    state_machine: &StateMachine<TrafficLightState, TrafficLightEvent, TrafficContext>,
) {
    println!("--- Transition History ---");
    let history = state_machine.get_history();
    for (i, record) in history.iter().enumerate() {
        println!(
            "  {}. {:?} -> {:?} via {:?} ({})",
            i + 1,
            record.from,
            record.to,
            record.event,
            if record.success { "✓" } else { "✗" }
        );
    }
    println!();
}

#[cfg(feature = "metrics")]
fn demonstrate_metrics(
    state_machine: &StateMachine<TrafficLightState, TrafficLightEvent, TrafficContext>,
) {
    println!("--- Performance Metrics ---");
    let metrics = state_machine.get_metrics();
    println!("  Total transitions: {}", metrics.total_transitions);
    println!("  Success rate: {:.1}%", metrics.success_rate() * 100.0);
    if let Some(avg_time) = metrics.average_transition_time() {
        println!("  Average transition time: {:?}", avg_time);
    }
    println!("  State visits:");
    for (state, count) in &metrics.state_visit_counts {
        println!("    {}: {} times", state, count);
    }
    println!();
}

#[cfg(feature = "visualization")]
fn demonstrate_visualization(
    state_machine: &StateMachine<TrafficLightState, TrafficLightEvent, TrafficContext>,
) {
    println!("--- State Machine Visualization ---");
    println!("Saving to 'traffic_light.dot' and 'traffic_light.puml'");

    let dot = state_machine.to_dot();
    let plantuml = state_machine.to_plantuml();

    // In a real application, you would write these to files
    if let Err(e) = std::fs::write("traffic_light.dot", dot) {
        eprintln!("Failed to write DOT file: {}", e);
    }

    if let Err(e) = std::fs::write("traffic_light.puml", plantuml) {
        eprintln!("Failed to write PlantUML file: {}", e);
    }

    println!("  ✓ Visualization files created\n");
}

/// Example of using the traffic light system with different feature sets
fn main() {
    // Check which features are enabled and inform the user
    println!("Enabled features:");
    #[cfg(feature = "history")]
    println!("  ✓ history");
    #[cfg(feature = "extended")]
    println!("  ✓ extended");
    #[cfg(feature = "metrics")]
    println!("  ✓ metrics");
    #[cfg(feature = "guards")]
    println!("  ✓ guards");
    #[cfg(feature = "timeout")]
    println!("  ✓ timeout");
    #[cfg(feature = "visualization")]
    println!("  ✓ visualization");
    println!();

    // Run the simulation
    simulate_traffic_light_system();

    // Show how to use different feature combinations
    println!("--- Feature Combination Examples ---");

    #[cfg(all(feature = "history", feature = "metrics"))]
    {
        println!("With history + metrics: Full audit trail with performance analysis");
    }

    #[cfg(all(feature = "extended", feature = "guards"))]
    {
        println!("With extended + guards: Complex state logic with prioritized transitions");
    }

    #[cfg(not(any(feature = "history", feature = "extended", feature = "metrics")))]
    {
        println!("Running with minimal features - core functionality only");
    }
}
