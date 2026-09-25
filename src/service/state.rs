//! Types representing the current state of the language server.

use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

use crate::max_runtime::{
    AccessAdmissionLaw, EmptyData, Exited, Initialized, InitializedData, Initializing,
    InitializingData, Machine, ShutDown, Uninitialized,
};

/// A list of possible states the language server can be in.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum State {
    /// Server has not received an `initialize` request.
    Uninitialized = 0,
    /// Server received an `initialize` request, but has not yet responded.
    Initializing = 1,
    /// Server received and responded success to an `initialize` request.
    Initialized = 2,
    /// Server received a `shutdown` request.
    ShutDown = 3,
    /// Server received an `exit` notification.
    Exited = 4,
}

/// An enum representing the active typestate machine at runtime.
#[derive(Debug)]
pub enum StateMachine {
    /// Uninitialized.
    Uninitialized(Machine<AccessAdmissionLaw, Uninitialized, EmptyData>),
    /// Initializing.
    Initializing(Machine<AccessAdmissionLaw, Initializing, InitializingData>),
    /// Initialized.
    Initialized(Machine<AccessAdmissionLaw, Initialized, InitializedData>),
    /// ShutDown.
    ShutDown(Machine<AccessAdmissionLaw, ShutDown, EmptyData>),
    /// Exited.
    #[allow(dead_code)]
    Exited(Machine<AccessAdmissionLaw, Exited, EmptyData>),
}

impl StateMachine {
    /// Returns the corresponding State enum variant.
    pub fn get_state(&self) -> State {
        match self {
            StateMachine::Uninitialized(_) => State::Uninitialized,
            StateMachine::Initializing(_) => State::Initializing,
            StateMachine::Initialized(_) => State::Initialized,
            StateMachine::ShutDown(_) => State::ShutDown,
            StateMachine::Exited(_) => State::Exited,
        }
    }
}

/// Thread-safe server state wrapper using a mutex.
pub struct ServerState {
    machine: Mutex<StateMachine>,
    wakers: Mutex<Vec<Waker>>,
    exit_code: AtomicI32,
}

impl ServerState {
    /// Creates a new `ServerState` initialized to `State::Uninitialized`.
    pub const fn new() -> Self {
        ServerState {
            machine: Mutex::new(StateMachine::Uninitialized(Machine {
                _law: std::marker::PhantomData,
                phase: Uninitialized,
                data: EmptyData {
                    client_capabilities: None,
                    server_capabilities: None,
                },
            })),
            wakers: Mutex::new(Vec::new()),
            exit_code: AtomicI32::new(1),
        }
    }

    /// Set the server state explicitly, discarding previous typestate data.
    /// This is provided for compatibility and direct overrides.
    #[allow(dead_code)]
    pub fn set(&self, state: State) {
        let mut lock = self.machine.lock().unwrap();
        let next_machine = match state {
            State::Uninitialized => {
                StateMachine::Uninitialized(Machine::new(Uninitialized, EmptyData::default()))
            }
            State::Initializing => StateMachine::Initializing(Machine::new(
                Initializing,
                InitializingData {
                    client_capabilities: serde_json::Value::Null,
                },
            )),
            State::Initialized => StateMachine::Initialized(Machine::new(
                Initialized,
                InitializedData {
                    client_capabilities: serde_json::Value::Null,
                    server_capabilities: serde_json::Value::Null,
                },
            )),
            State::ShutDown => StateMachine::ShutDown(Machine::new(ShutDown, EmptyData::default())),
            State::Exited => StateMachine::Exited(Machine::new(Exited, EmptyData::default())),
        };
        *lock = next_machine;
        if let Ok(mut reg) = crate::get_registry().lock() {
            reg.current_state = state;
        }

        if state != State::Initializing {
            let mut wakers = self.wakers.lock().unwrap();
            let wakers_to_wake = std::mem::take(&mut *wakers);
            for waker in wakers_to_wake {
                waker.wake();
            }
        }
    }

    /// Returns the current state of the server.
    pub fn get(&self) -> State {
        self.machine.lock().unwrap().get_state()
    }

    /// Sets the exit code.
    #[allow(dead_code)]
    pub fn set_exit_code(&self, code: i32) {
        self.exit_code.store(code, Ordering::SeqCst);
    }

    /// Gets the exit code.
    pub fn get_exit_code(&self) -> i32 {
        self.exit_code.load(Ordering::SeqCst)
    }

    /// Attempts to transition from `Uninitialized` to `Initializing` phase.
    pub fn try_initialize(&self, client_caps: serde_json::Value) -> bool {
        let mut lock = self.machine.lock().unwrap();
        match &*lock {
            StateMachine::Uninitialized(_) => {
                let old = std::mem::replace(
                    &mut *lock,
                    StateMachine::Exited(Machine::new(Exited, EmptyData::default())),
                );
                if let StateMachine::Uninitialized(m) = old {
                    *lock = StateMachine::Initializing(m.admit_initialize(client_caps));
                    if let Ok(mut reg) = crate::get_registry().lock() {
                        reg.current_state = State::Initializing;
                    }
                    true
                } else {
                    unreachable!()
                }
            }
            _ => false,
        }
    }

    /// Transitions from `Initializing` to `Initialized` phase.
    pub fn transition_to_initialized(&self, server_caps: serde_json::Value) -> bool {
        let mut lock = self.machine.lock().unwrap();
        let success = match &*lock {
            StateMachine::Initializing(_) => {
                let old = std::mem::replace(
                    &mut *lock,
                    StateMachine::Exited(Machine::new(Exited, EmptyData::default())),
                );
                if let StateMachine::Initializing(m) = old {
                    *lock = StateMachine::Initialized(m.admit_initialized(server_caps));
                    true
                } else {
                    unreachable!()
                }
            }
            _ => false,
        };
        if success {
            if let Ok(mut reg) = crate::get_registry().lock() {
                reg.current_state = State::Initialized;
            }
            let mut wakers = self.wakers.lock().unwrap();
            let wakers_to_wake = std::mem::take(&mut *wakers);
            for waker in wakers_to_wake {
                waker.wake();
            }
        }
        success
    }

    /// Reverts transition back to `Uninitialized` from `Initializing` phase.
    pub fn transition_to_uninitialized(&self) -> bool {
        let mut lock = self.machine.lock().unwrap();
        let success = match &*lock {
            StateMachine::Initializing(_) => {
                *lock =
                    StateMachine::Uninitialized(Machine::new(Uninitialized, EmptyData::default()));
                true
            }
            _ => false,
        };
        if success {
            if let Ok(mut reg) = crate::get_registry().lock() {
                reg.current_state = State::Uninitialized;
            }
            let mut wakers = self.wakers.lock().unwrap();
            let wakers_to_wake = std::mem::take(&mut *wakers);
            for waker in wakers_to_wake {
                waker.wake();
            }
        }
        success
    }

    /// Transitions from `Initialized` to `ShutDown` phase.
    pub fn transition_to_shutdown(&self) -> bool {
        let mut lock = self.machine.lock().unwrap();
        match &*lock {
            StateMachine::Initialized(_) => {
                let old = std::mem::replace(
                    &mut *lock,
                    StateMachine::Exited(Machine::new(Exited, EmptyData::default())),
                );
                if let StateMachine::Initialized(m) = old {
                    *lock = StateMachine::ShutDown(m.admit_shutdown());
                    if let Ok(mut reg) = crate::get_registry().lock() {
                        reg.current_state = State::ShutDown;
                    }
                    true
                } else {
                    unreachable!()
                }
            }
            _ => false,
        }
    }

    /// Transitions to `Exited` phase.
    pub fn transition_to_exited(&self) -> bool {
        let mut lock = self.machine.lock().unwrap();
        let res = match &*lock {
            StateMachine::ShutDown(_) => {
                let old = std::mem::replace(
                    &mut *lock,
                    StateMachine::Exited(Machine::new(Exited, EmptyData::default())),
                );
                if let StateMachine::ShutDown(m) = old {
                    *lock = StateMachine::Exited(m.admit_exit());
                    self.set_exit_code(0);
                    true
                } else {
                    unreachable!()
                }
            }
            _ => {
                *lock = StateMachine::Exited(Machine::new(Exited, EmptyData::default()));
                self.set_exit_code(1);
                true
            }
        };
        if let Ok(mut reg) = crate::get_registry().lock() {
            reg.current_state = State::Exited;
        }
        res
    }

    /// Polls whether the server state is currently `Initializing`.
    /// If yes, registers the waker to be notified when state changes.
    #[allow(dead_code)]
    pub fn poll_initializing(&self, cx: &mut Context<'_>) -> Poll<()> {
        let lock = self.machine.lock().unwrap();
        match lock.get_state() {
            State::Initializing => {
                self.wakers.lock().unwrap().push(cx.waker().clone());
                Poll::Pending
            }
            _ => Poll::Ready(()),
        }
    }
}

impl Debug for ServerState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.get().fmt(f)
    }
}
