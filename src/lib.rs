//! A flexible state machine implementation with optional advanced features
//!
//! # Features
//!
//! - `history` - State transition history tracking
//! - `extended` - Entry/exit actions for states
//! - `metrics` - Performance metrics collection
//! - `hierarchical` - Hierarchical state support
//! - `guards` - Guard conditions with priorities
//! - `timeout` - State timeout support
//! - `parallel` - Parallel state regions
//! - `visualization` - Export to DOT/PlantUML
//! - `serde` - Serialization support
//! - `async` - Async action support
//!
//! # How to use rs-statemachine
//!
//!```rust
//! use rs_statemachine::*;
//! // Define your states
//! #[derive(Debug, Clone, Hash, Eq, PartialEq)]
//! enum MyState {
//!     Idle,
//!     Working,
//!     Done,
//! }
//! impl State for MyState {}
//!
//! // Define your events
//! #[derive(Debug, Clone, Hash, Eq, PartialEq)]
//! enum MyEvent {
//!     Start,
//!     Complete,
//! }
//! impl Event for MyEvent {}
//!
//! // Define your context
//! #[derive(Debug, Clone)]
//! struct MyContext {
//!     task_id: String,
//! }
//! impl Context for MyContext {}
//!
//! // Build the state machine
//! let mut builder = StateMachineBuilderFactory::create::<MyState, MyEvent, MyContext>();
//! builder
//!     .external_transition()
//!     .from(MyState::Idle)
//!     .to(MyState::Working)
//!     .on(MyEvent::Start)
//!     .perform(|_s, _e, ctx| {
//!         println!("Starting task {}", ctx.task_id);
//!     });
//! let state_machine = builder.build();
//!
//! let context = MyContext {
//!             task_id: "frank".to_string(),
//!         };
//! state_machine.fire_event(MyState::Idle, MyEvent::Start, context);
//! ```
//!

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

#[cfg(feature = "history")]
use std::sync::Mutex;
#[cfg(any(feature = "history", feature = "timeout", feature = "metrics"))]
use std::time::{Duration, Instant};

/// Trait for state machine states
pub trait State: Debug + Clone + Hash + Eq + PartialEq {
    #[cfg(feature = "serde")]
    fn serialize(&self) -> Result<String, Box<dyn std::error::Error>>
    where
        Self: serde::Serialize,
    {
        serde_json::to_string(self).map_err(|e| e.into())
    }
}

/// Trait for state machine events
pub trait Event: Debug + Clone + Hash + Eq + PartialEq {
    #[cfg(feature = "serde")]
    fn serialize(&self) -> Result<String, Box<dyn std::error::Error>>
    where
        Self: serde::Serialize,
    {
        serde_json::to_string(self).map_err(|e| e.into())
    }
}

/// Trait for state machine context
pub trait Context: Debug + Clone {
    #[cfg(feature = "serde")]
    fn serialize(&self) -> Result<String, Box<dyn std::error::Error>>
    where
        Self: serde::Serialize,
    {
        serde_json::to_string(self).map_err(|e| e.into())
    }
}

/// Type alias for condition functions
pub type Condition<S, E, C> = Arc<dyn Fn(&S, &E, &C) -> bool + Send + Sync>;

/// Type alias for action functions
pub type Action<S, E, C> = Arc<dyn Fn(&S, &E, &C) -> () + Send + Sync>;

/// Type alias for fail callback functions
pub type FailCallback<S, E, C> = Arc<dyn Fn(&S, &E, &C) + Send + Sync>;

/// Represents a transition in the state machine
#[derive(Clone)]
pub struct Transition<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    from: S,
    to: S,
    event: E,
    condition: Option<Condition<S, E, C>>,
    action: Option<Action<S, E, C>>,
    transition_type: TransitionType,
    #[cfg(feature = "guards")]
    priority: u32,
}

/// Type of transition
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionType {
    External,
    Internal,
}

/// Error types for state machine operations
#[derive(Debug, Clone)]
pub enum TransitionError {
    NoValidTransition {
        from: String,
        event: String,
    },
    ConditionFailed,
    #[cfg(feature = "timeout")]
    Timeout,
    #[cfg(feature = "async")]
    AsyncError(String),
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionError::NoValidTransition { from, event } => {
                write!(
                    f,
                    "No valid transition from state {} with event {}",
                    from, event
                )
            }
            TransitionError::ConditionFailed => write!(f, "Transition condition failed"),
            #[cfg(feature = "timeout")]
            TransitionError::Timeout => write!(f, "State timeout occurred"),
            #[cfg(feature = "async")]
            TransitionError::AsyncError(msg) => write!(f, "Async error: {}", msg),
        }
    }
}

impl std::error::Error for TransitionError {}

// History tracking feature
#[cfg(feature = "history")]
#[derive(Debug, Clone)]
pub struct TransitionRecord<S, E>
where
    S: State,
    E: Event,
{
    pub from: S,
    pub to: S,
    pub event: E,
    pub timestamp: Instant,
    pub success: bool,
}

// Metrics feature
#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub struct StateMachineMetrics {
    pub total_transitions: u64,
    pub successful_transitions: u64,
    pub failed_transitions: u64,
    pub transition_durations: Vec<Duration>,
    pub state_visit_counts: HashMap<String, u64>,
}

#[cfg(feature = "metrics")]
impl StateMachineMetrics {
    pub fn new() -> Self {
        StateMachineMetrics {
            total_transitions: 0,
            successful_transitions: 0,
            failed_transitions: 0,
            transition_durations: Vec::new(),
            state_visit_counts: HashMap::new(),
        }
    }

    pub fn average_transition_time(&self) -> Option<Duration> {
        if self.transition_durations.is_empty() {
            None
        } else {
            let total: Duration = self.transition_durations.iter().sum();
            Some(total / self.transition_durations.len() as u32)
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_transitions == 0 {
            0.0
        } else {
            self.successful_transitions as f64 / self.total_transitions as f64
        }
    }
}

// Extended state machine features
#[cfg(feature = "extended")]
pub struct StateActions<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    pub on_entry: Option<Arc<dyn Fn(&S, &C) + Send + Sync>>,
    pub on_exit: Option<Arc<dyn Fn(&S, &C) + Send + Sync>>,
    _phantom: std::marker::PhantomData<E>,
}

// Hierarchical state support
#[cfg(feature = "hierarchical")]
pub trait HierarchicalState: State {
    fn parent(&self) -> Option<Self>;
    fn children(&self) -> Vec<Self>;
    fn is_substate_of(&self, other: &Self) -> bool;
}

// Async support
#[cfg(feature = "async")]
use async_trait::async_trait;

#[cfg(feature = "async")]
#[async_trait]
pub trait AsyncAction<S, E, C>: Send + Sync
where
    S: State + Send,
    E: Event + Send,
    C: Context + Send,
{
    async fn execute(&self, from: &S, event: &E, context: &C);
}

/// The main state machine struct
pub struct StateMachine<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    id: String,
    transitions: HashMap<(S, E), Vec<Transition<S, E, C>>>,
    fail_callback: Option<FailCallback<S, E, C>>,

    #[cfg(feature = "history")]
    history: Arc<Mutex<Vec<TransitionRecord<S, E>>>>,

    #[cfg(feature = "metrics")]
    metrics: Arc<Mutex<StateMachineMetrics>>,

    #[cfg(feature = "extended")]
    state_actions: HashMap<S, StateActions<S, E, C>>,

    #[cfg(feature = "timeout")]
    state_timeouts: HashMap<S, Duration>,
    #[cfg(feature = "timeout")]
    timeout_transitions: HashMap<S, (S, E)>,

    #[cfg(feature = "async")]
    async_actions: HashMap<(S, E), Box<dyn AsyncAction<S, E, C>>>,
}

impl<S, E, C> StateMachine<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    /// Fire an event and perform state transition
    pub fn fire_event(&self, from: S, event: E, context: C) -> Result<S, TransitionError> {
        #[cfg(feature = "metrics")]
        let start_time = Instant::now();

        #[cfg(feature = "extended")]
        {
            // Execute exit action for current state
            if let Some(actions) = self.state_actions.get(&from) {
                if let Some(on_exit) = &actions.on_exit {
                    on_exit(&from, &context);
                }
            }
        }

        let key = (from.clone(), event.clone());
        let result = if let Some(transitions) = self.transitions.get(&key) {
            let mut valid_transitions = transitions.clone();

            #[cfg(feature = "guards")]
            {
                // Sort by priority if guards feature is enabled
                valid_transitions.sort_by_key(|t| std::cmp::Reverse(t.priority));
            }

            let mut transition_result = None;
            for transition in valid_transitions {
                if let Some(condition) = &transition.condition {
                    if !condition(&from, &event, &context) {
                        continue;
                    }
                }

                // Execute action if present
                if let Some(action) = &transition.action {
                    action(&from, &event, &context);
                }

                transition_result = Some(Ok(transition.to.clone()));
                break;
            }

            transition_result.unwrap_or_else(|| {
                if let Some(fail_callback) = &self.fail_callback {
                    fail_callback(&from, &event, &context);
                }
                Err(TransitionError::NoValidTransition {
                    from: format!("{:?}", from),
                    event: format!("{:?}", event),
                })
            })
        } else {
            if let Some(fail_callback) = &self.fail_callback {
                fail_callback(&from, &event, &context);
            }
            Err(TransitionError::NoValidTransition {
                from: format!("{:?}", from),
                event: format!("{:?}", event),
            })
        };

        #[cfg(feature = "extended")]
        {
            // Execute entry action for new state
            if let Ok(new_state) = &result {
                if let Some(actions) = self.state_actions.get(new_state) {
                    if let Some(on_entry) = &actions.on_entry {
                        on_entry(new_state, &context);
                    }
                }
            }
        }

        #[cfg(feature = "history")]
        {
            let record = match &result {
                Ok(to_state) => TransitionRecord {
                    from: from.clone(),
                    to: to_state.clone(),
                    event: event.clone(),
                    timestamp: Instant::now(),
                    success: true,
                },
                Err(_) => TransitionRecord {
                    from: from.clone(),
                    to: from.clone(),
                    event: event.clone(),
                    timestamp: Instant::now(),
                    success: false,
                },
            };

            if let Ok(mut history) = self.history.lock() {
                history.push(record);
            }
        }

        #[cfg(feature = "metrics")]
        {
            let duration = start_time.elapsed();
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.total_transitions += 1;
                metrics.transition_durations.push(duration);

                match &result {
                    Ok(to_state) => {
                        metrics.successful_transitions += 1;
                        let state_name = format!("{:?}", to_state);
                        *metrics.state_visit_counts.entry(state_name).or_insert(0) += 1;
                    }
                    Err(_) => {
                        metrics.failed_transitions += 1;
                    }
                }
            }
        }

        result
    }

    /// Verify if a transition is possible
    pub fn verify(&self, from: S, event: E) -> bool {
        let key = (from, event);
        self.transitions.contains_key(&key)
    }

    /// Get the ID of the state machine
    pub fn id(&self) -> &str {
        &self.id
    }

    #[cfg(feature = "history")]
    /// Get transition history
    pub fn get_history(&self) -> Vec<TransitionRecord<S, E>> {
        self.history.lock().unwrap().clone()
    }

    #[cfg(feature = "history")]
    /// Clear transition history
    pub fn clear_history(&self) {
        self.history.lock().unwrap().clear();
    }

    #[cfg(feature = "metrics")]
    /// Get metrics
    pub fn get_metrics(&self) -> StateMachineMetrics {
        self.metrics.lock().unwrap().clone()
    }

    #[cfg(feature = "extended")]
    /// Add entry action for a state
    pub fn add_entry_action<F>(&mut self, state: S, action: F)
    where
        F: Fn(&S, &C) + Send + Sync + 'static,
    {
        let actions = self.state_actions.entry(state).or_insert(StateActions {
            on_entry: None,
            on_exit: None,
            _phantom: Default::default(),
        });
        actions.on_entry = Some(Arc::new(action));
    }

    #[cfg(feature = "extended")]
    /// Add exit action for a state
    pub fn add_exit_action<F>(&mut self, state: S, action: F)
    where
        F: Fn(&S, &C) + Send + Sync + 'static,
    {
        let actions = self.state_actions.entry(state).or_insert(StateActions {
            on_entry: None,
            on_exit: None,
            _phantom: Default::default(),
        });
        actions.on_exit = Some(Arc::new(action));
    }

    #[cfg(feature = "timeout")]
    /// Set timeout for a state
    pub fn set_state_timeout(
        &mut self,
        state: S,
        duration: Duration,
        target_state: S,
        timeout_event: E,
    ) {
        self.state_timeouts.insert(state.clone(), duration);
        self.timeout_transitions
            .insert(state, (target_state, timeout_event));
    }

    #[cfg(feature = "visualization")]
    /// Export to DOT format
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph StateMachine {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n\n");

        for ((from, event), transitions) in &self.transitions {
            for transition in transitions {
                dot.push_str(&format!(
                    "  \"{:?}\" -> \"{:?}\" [label=\"{:?}\"];\n",
                    from, transition.to, event
                ));
            }
        }

        dot.push_str("}\n");
        dot
    }

    #[cfg(feature = "visualization")]
    /// Export to PlantUML format
    pub fn to_plantuml(&self) -> String {
        let mut uml = String::from("@startuml\n");

        for ((from, event), transitions) in &self.transitions {
            for transition in transitions {
                uml.push_str(&format!(
                    "{:?} --> {:?} : {:?}\n",
                    from, transition.to, event
                ));
            }
        }

        uml.push_str("@enduml\n");
        uml
    }
}

#[cfg(feature = "async")]
impl<S, E, C> StateMachine<S, E, C>
where
    S: State + Send + Sync,
    E: Event + Send + Sync,
    C: Context + Send + Sync,
{
    /// Fire an event asynchronously
    pub async fn fire_event_async(
        &self,
        from: S,
        event: E,
        context: C,
    ) -> Result<S, TransitionError> {
        let key = (from.clone(), event.clone());

        if let Some(async_action) = self.async_actions.get(&key) {
            async_action.execute(&from, &event, &context).await;
        }

        self.fire_event(from, event, context)
    }
}

/// Builder for creating state machines with fluent API
pub struct StateMachineBuilder<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    id: Option<String>,
    transitions: Vec<Transition<S, E, C>>,
    fail_callback: Option<FailCallback<S, E, C>>,
    #[cfg(feature = "extended")]
    state_actions: HashMap<S, StateActions<S, E, C>>,
    #[cfg(feature = "timeout")]
    state_timeouts: HashMap<S, Duration>,
    #[cfg(feature = "timeout")]
    timeout_transitions: HashMap<S, (S, E)>,
    #[cfg(feature = "async")]
    async_actions: HashMap<(S, E), Box<dyn AsyncAction<S, E, C>>>,
}

impl<S, E, C> StateMachineBuilder<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    /// Create a new state machine builder
    pub fn new() -> Self {
        StateMachineBuilder {
            id: None,
            transitions: Vec::new(),
            fail_callback: None,
            #[cfg(feature = "extended")]
            state_actions: HashMap::new(),
            #[cfg(feature = "timeout")]
            state_timeouts: HashMap::new(),
            #[cfg(feature = "timeout")]
            timeout_transitions: HashMap::new(),
            #[cfg(feature = "async")]
            async_actions: HashMap::new(),
        }
    }

    /// Set the ID of the state machine
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Start building an external transition
    pub fn external_transition(&mut self) -> ExternalTransitionBuilder<S, E, C> {
        ExternalTransitionBuilder::new(self)
    }

    /// Start building an internal transition
    pub fn internal_transition(&mut self) -> InternalTransitionBuilder<S, E, C> {
        InternalTransitionBuilder::new(self)
    }

    /// Start building external transitions from multiple states
    pub fn external_transitions(&mut self) -> ExternalTransitionsBuilder<S, E, C> {
        ExternalTransitionsBuilder::new(self)
    }

    /// Set fail callback
    pub fn set_fail_callback(&mut self, callback: FailCallback<S, E, C>) -> &mut Self {
        self.fail_callback = Some(callback);
        self
    }

    #[cfg(feature = "extended")]
    /// Add entry action for a state
    pub fn with_entry_action<F>(&mut self, state: S, action: F) -> &mut Self
    where
        F: Fn(&S, &C) + Send + Sync + 'static,
    {
        let actions = self.state_actions.entry(state).or_insert(StateActions {
            on_entry: None,
            on_exit: None,
            _phantom: Default::default(),
        });
        actions.on_entry = Some(Arc::new(action));
        self
    }

    #[cfg(feature = "extended")]
    /// Add exit action for a state
    pub fn with_exit_action<F>(&mut self, state: S, action: F) -> &mut Self
    where
        F: Fn(&S, &C) + Send + Sync + 'static,
    {
        let actions = self.state_actions.entry(state).or_insert(StateActions {
            on_entry: None,
            on_exit: None,
            _phantom: Default::default(),
        });
        actions.on_exit = Some(Arc::new(action));
        self
    }

    #[cfg(feature = "timeout")]
    /// Set timeout for a state
    pub fn with_state_timeout(
        &mut self,
        state: S,
        duration: Duration,
        target_state: S,
        timeout_event: E,
    ) -> &mut Self {
        self.state_timeouts.insert(state.clone(), duration);
        self.timeout_transitions
            .insert(state, (target_state, timeout_event));
        self
    }

    /// Build the state machine
    pub fn build(self) -> StateMachine<S, E, C> {
        let id = self.id.unwrap_or_else(|| "StateMachine".to_string());
        let mut transitions_map = HashMap::new();

        for transition in self.transitions {
            let key = (transition.from.clone(), transition.event.clone());
            transitions_map
                .entry(key)
                .or_insert_with(Vec::new)
                .push(transition);
        }

        StateMachine {
            id,
            transitions: transitions_map,
            fail_callback: self.fail_callback,
            #[cfg(feature = "history")]
            history: Arc::new(Mutex::new(Vec::new())),
            #[cfg(feature = "metrics")]
            metrics: Arc::new(Mutex::new(StateMachineMetrics::new())),
            #[cfg(feature = "extended")]
            state_actions: self.state_actions,
            #[cfg(feature = "timeout")]
            state_timeouts: self.state_timeouts,
            #[cfg(feature = "timeout")]
            timeout_transitions: self.timeout_transitions,
            #[cfg(feature = "async")]
            async_actions: self.async_actions,
        }
    }

    fn add_transition(&mut self, transition: Transition<S, E, C>) {
        self.transitions.push(transition);
    }
}

impl<S, E, C> Default for StateMachineBuilder<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for external transitions
pub struct ExternalTransitionBuilder<'a, S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    builder: &'a mut StateMachineBuilder<S, E, C>,
    from: Option<S>,
    to: Option<S>,
    event: Option<E>,
    condition: Option<Condition<S, E, C>>,
    action: Option<Action<S, E, C>>,
    #[cfg(feature = "guards")]
    priority: u32,
}

impl<'a, S, E, C> ExternalTransitionBuilder<'a, S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    fn new(builder: &'a mut StateMachineBuilder<S, E, C>) -> Self {
        ExternalTransitionBuilder {
            builder,
            from: None,
            to: None,
            event: None,
            condition: None,
            action: None,
            #[cfg(feature = "guards")]
            priority: 0,
        }
    }

    pub fn from(mut self, state: S) -> Self {
        self.from = Some(state);
        self
    }

    pub fn to(mut self, state: S) -> Self {
        self.to = Some(state);
        self
    }

    pub fn on(mut self, event: E) -> Self {
        self.event = Some(event);
        self
    }

    pub fn when<F>(mut self, condition: F) -> Self
    where
        F: Fn(&S, &E, &C) -> bool + Send + Sync + 'static,
    {
        self.condition = Some(Arc::new(condition));
        self
    }

    #[cfg(feature = "guards")]
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn perform<F>(mut self, action: F) -> &'a mut StateMachineBuilder<S, E, C>
    where
        F: Fn(&S, &E, &C) -> () + Send + Sync + 'static,
    {
        self.action = Some(Arc::new(action));
        self.build()
    }

    fn build(self) -> &'a mut StateMachineBuilder<S, E, C> {
        let transition = Transition {
            from: self.from.expect("from state is required"),
            to: self.to.expect("to state is required"),
            event: self.event.expect("event is required"),
            condition: self.condition,
            action: self.action,
            transition_type: TransitionType::External,
            #[cfg(feature = "guards")]
            priority: self.priority,
        };

        self.builder.add_transition(transition);
        self.builder
    }
}

/// Builder for internal transitions
pub struct InternalTransitionBuilder<'a, S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    builder: &'a mut StateMachineBuilder<S, E, C>,
    within: Option<S>,
    event: Option<E>,
    condition: Option<Condition<S, E, C>>,
    action: Option<Action<S, E, C>>,
    #[cfg(feature = "guards")]
    priority: u32,
}

impl<'a, S, E, C> InternalTransitionBuilder<'a, S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    fn new(builder: &'a mut StateMachineBuilder<S, E, C>) -> Self {
        InternalTransitionBuilder {
            builder,
            within: None,
            event: None,
            condition: None,
            action: None,
            #[cfg(feature = "guards")]
            priority: 0,
        }
    }

    pub fn within(mut self, state: S) -> Self {
        self.within = Some(state);
        self
    }

    pub fn on(mut self, event: E) -> Self {
        self.event = Some(event);
        self
    }

    pub fn when<F>(mut self, condition: F) -> Self
    where
        F: Fn(&S, &E, &C) -> bool + Send + Sync + 'static,
    {
        self.condition = Some(Arc::new(condition));
        self
    }

    #[cfg(feature = "guards")]
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn perform<F>(mut self, action: F) -> &'a mut StateMachineBuilder<S, E, C>
    where
        F: Fn(&S, &E, &C) -> () + Send + Sync + 'static,
    {
        self.action = Some(Arc::new(action));
        self.build()
    }

    fn build(self) -> &'a mut StateMachineBuilder<S, E, C> {
        let state = self.within.expect("within state is required");
        let transition = Transition {
            from: state.clone(),
            to: state,
            event: self.event.expect("event is required"),
            condition: self.condition,
            action: self.action,
            transition_type: TransitionType::Internal,
            #[cfg(feature = "guards")]
            priority: self.priority,
        };

        self.builder.add_transition(transition);
        self.builder
    }
}

/// Builder for external transitions from multiple states
pub struct ExternalTransitionsBuilder<'a, S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    builder: &'a mut StateMachineBuilder<S, E, C>,
    from_states: Vec<S>,
    to: Option<S>,
    event: Option<E>,
    condition: Option<Condition<S, E, C>>,
    action: Option<Action<S, E, C>>,
    #[cfg(feature = "guards")]
    priority: u32,
}

impl<'a, S, E, C> ExternalTransitionsBuilder<'a, S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    fn new(builder: &'a mut StateMachineBuilder<S, E, C>) -> Self {
        ExternalTransitionsBuilder {
            builder,
            from_states: Vec::new(),
            to: None,
            event: None,
            condition: None,
            action: None,
            #[cfg(feature = "guards")]
            priority: 0,
        }
    }

    pub fn from_among(mut self, states: Vec<S>) -> Self {
        self.from_states = states;
        self
    }

    pub fn to(mut self, state: S) -> Self {
        self.to = Some(state);
        self
    }

    pub fn on(mut self, event: E) -> Self {
        self.event = Some(event);
        self
    }

    pub fn when<F>(mut self, condition: F) -> Self
    where
        F: Fn(&S, &E, &C) -> bool + Send + Sync + 'static,
    {
        self.condition = Some(Arc::new(condition));
        self
    }

    #[cfg(feature = "guards")]
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn perform<F>(mut self, action: F) -> &'a mut StateMachineBuilder<S, E, C>
    where
        F: Fn(&S, &E, &C) -> () + Send + Sync + 'static,
    {
        self.action = Some(Arc::new(action));
        self.build()
    }

    fn build(self) -> &'a mut StateMachineBuilder<S, E, C> {
        let to = self.to.expect("to state is required");
        let event = self.event.expect("event is required");
        let condition = self.condition.clone();
        let action = self.action.clone();

        for from in self.from_states {
            let transition = Transition {
                from,
                to: to.clone(),
                event: event.clone(),
                condition: condition.clone(),
                action: action.clone(),
                transition_type: TransitionType::External,
                #[cfg(feature = "guards")]
                priority: self.priority,
            };

            self.builder.add_transition(transition);
        }

        self.builder
    }
}

/// Factory for creating state machine builders
pub struct StateMachineBuilderFactory;

impl StateMachineBuilderFactory {
    pub fn create<S, E, C>() -> StateMachineBuilder<S, E, C>
    where
        S: State,
        E: Event,
        C: Context,
    {
        StateMachineBuilder::new()
    }
}

/// Factory for managing multiple state machines
pub struct StateMachineFactory<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    machines: HashMap<String, StateMachine<S, E, C>>,
}

impl<S, E, C> StateMachineFactory<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    pub fn new() -> Self {
        StateMachineFactory {
            machines: HashMap::new(),
        }
    }

    pub fn register(&mut self, machine: StateMachine<S, E, C>) {
        self.machines.insert(machine.id.clone(), machine);
    }

    pub fn get(&self, id: &str) -> Option<&StateMachine<S, E, C>> {
        self.machines.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut StateMachine<S, E, C>> {
        self.machines.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<StateMachine<S, E, C>> {
        self.machines.remove(id)
    }

    pub fn list_ids(&self) -> Vec<&str> {
        self.machines.keys().map(|s| s.as_str()).collect()
    }
}

impl<S, E, C> Default for StateMachineFactory<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    fn default() -> Self {
        Self::new()
    }
}

// Parallel state machine support (requires parallel feature)
#[cfg(feature = "parallel")]
pub struct ParallelStateMachine<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    regions: Vec<StateMachine<S, E, C>>,
}

#[cfg(feature = "parallel")]
impl<S, E, C> ParallelStateMachine<S, E, C>
where
    S: State,
    E: Event,
    C: Context,
{
    pub fn new() -> Self {
        ParallelStateMachine {
            regions: Vec::new(),
        }
    }

    pub fn add_region(&mut self, machine: StateMachine<S, E, C>) {
        self.regions.push(machine);
    }

    pub fn fire_event(
        &self,
        states: Vec<S>,
        event: E,
        context: C,
    ) -> Vec<Result<S, TransitionError>> {
        self.regions
            .iter()
            .zip(states.iter())
            .map(|(machine, state)| {
                machine.fire_event(state.clone(), event.clone(), context.clone())
            })
            .collect()
    }

    pub fn get_region(&self, index: usize) -> Option<&StateMachine<S, E, C>> {
        self.regions.get(index)
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    enum States {
        State1,
        State2,
        State3,
        State4,
    }

    impl State for States {}

    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    enum Events {
        Event1,
        Event2,
        Event3,
        Event4,
        InternalEvent,
    }

    impl Event for Events {}

    #[derive(Debug, Clone)]
    struct TestContext {
        operator: String,
        entity_id: String,
    }

    impl Context for TestContext {}

    #[test]
    fn test_basic_transition() {
        let mut builder = StateMachineBuilderFactory::create::<States, Events, TestContext>();

        builder
            .external_transition()
            .from(States::State1)
            .to(States::State2)
            .on(Events::Event1)
            .when(|_s, _e, c| c.operator == "frank")
            .perform(|_s, _e, c| {
                println!("Performing action for operator: {}", c.operator);
            });

        let state_machine = builder.build();

        let context = TestContext {
            operator: "frank".to_string(),
            entity_id: "123456".to_string(),
        };

        let result = state_machine.fire_event(States::State1, Events::Event1, context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), States::State2);
    }

    #[test]
    #[cfg(feature = "history")]
    fn test_history_tracking() {
        let mut builder = StateMachineBuilderFactory::create::<States, Events, TestContext>();

        builder
            .external_transition()
            .from(States::State1)
            .to(States::State2)
            .on(Events::Event1)
            .perform(|_s, _e, _c| {});

        let state_machine = builder.build();
        let context = TestContext {
            operator: "test".to_string(),
            entity_id: "789".to_string(),
        };

        let _ = state_machine.fire_event(States::State1, Events::Event1, context);
        let history = state_machine.get_history();
        assert_eq!(history.len(), 1);
        assert!(history[0].success);
    }

    #[test]
    #[cfg(feature = "extended")]
    fn test_entry_exit_actions() {
        let mut builder = StateMachineBuilderFactory::create::<States, Events, TestContext>();

        builder
            .with_entry_action(States::State2, |_s, _c| {
                println!("Entering State2");
            })
            .with_exit_action(States::State1, |_s, _c| {
                println!("Exiting State1");
            })
            .external_transition()
            .from(States::State1)
            .to(States::State2)
            .on(Events::Event1)
            .perform(|_s, _e, _c| {});

        let state_machine = builder.build();
        let context = TestContext {
            operator: "test".to_string(),
            entity_id: "789".to_string(),
        };

        let result = state_machine.fire_event(States::State1, Events::Event1, context);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "metrics")]
    fn test_metrics_collection() {
        let mut builder = StateMachineBuilderFactory::create::<States, Events, TestContext>();

        builder
            .external_transition()
            .from(States::State1)
            .to(States::State2)
            .on(Events::Event1)
            .perform(|_s, _e, _c| {});

        let state_machine = builder.build();
        let context = TestContext {
            operator: "test".to_string(),
            entity_id: "789".to_string(),
        };

        let _ = state_machine.fire_event(States::State1, Events::Event1, context.clone());
        let _ = state_machine.fire_event(States::State1, Events::Event2, context); // Should fail

        let metrics = state_machine.get_metrics();
        assert_eq!(metrics.total_transitions, 2);
        assert_eq!(metrics.successful_transitions, 1);
        assert_eq!(metrics.failed_transitions, 1);
        assert_eq!(metrics.success_rate(), 0.5);
    }

    #[test]
    #[cfg(feature = "visualization")]
    fn test_visualization() {
        let mut builder = StateMachineBuilderFactory::create::<States, Events, TestContext>();

        builder
            .external_transition()
            .from(States::State1)
            .to(States::State2)
            .on(Events::Event1)
            .perform(|_s, _e, _c| {});

        let state_machine = builder.build();

        let dot = state_machine.to_dot();
        assert!(dot.contains("digraph StateMachine"));
        assert!(dot.contains("State1"));
        assert!(dot.contains("State2"));

        let plantuml = state_machine.to_plantuml();
        assert!(plantuml.contains("@startuml"));
        assert!(plantuml.contains("State1"));
        assert!(plantuml.contains("State2"));
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn test_parallel_regions() {
        let mut builder1 = StateMachineBuilderFactory::create::<States, Events, TestContext>();
        builder1
            .external_transition()
            .from(States::State1)
            .to(States::State2)
            .on(Events::Event1)
            .perform(|_s, _e, _c| {});

        let mut builder2 = StateMachineBuilderFactory::create::<States, Events, TestContext>();
        builder2
            .external_transition()
            .from(States::State3)
            .to(States::State4)
            .on(Events::Event1)
            .perform(|_s, _e, _c| {});

        let mut parallel_machine = ParallelStateMachine::new();
        parallel_machine.add_region(builder1.build());
        parallel_machine.add_region(builder2.build());

        let context = TestContext {
            operator: "test".to_string(),
            entity_id: "789".to_string(),
        };

        let results = parallel_machine.fire_event(
            vec![States::State1, States::State3],
            Events::Event1,
            context,
        );

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        assert_eq!(results[0].as_ref().unwrap(), &States::State2);
        assert_eq!(results[1].as_ref().unwrap(), &States::State4);
    }
}
