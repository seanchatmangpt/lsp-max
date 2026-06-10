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
    /// The autonomic mesh manager for monitoring the workspace state.
    pub mesh: Mutex<crate::max_runtime::AutonomicMesh>,
    parent_pid: Mutex<Option<u32>>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerState {
    /// Creates a new `ServerState` initialized to `State::Uninitialized`.
    pub fn new() -> Self {
        let mut mesh = crate::max_runtime::AutonomicMesh::new();
        let mut instance = crate::max_runtime::LspInstance::new("LSP_1");
        instance.phase = crate::max_runtime::LspPhase::Uninitialized;
        let r0 = crate::max_protocol::Receipt {
            receipt_id: "rcpt-uninitialized".to_string(),
            hash: crate::max_runtime::sha256(b"rcpt-uninitialized"),
            prev_receipt_hash: None,
        };
        instance.receipts.push(r0.clone());
        mesh.add_instance(instance);

        if let Ok(mut reg) = crate::get_registry().lock() {
            reg.receipts.insert(r0.receipt_id.clone(), r0);
        }

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
            mesh: Mutex::new(mesh),
            parent_pid: Mutex::new(None),
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
        if let Some(instance) = self.mesh.lock().unwrap().instances.get_mut("LSP_1") {
            instance.phase = match state {
                crate::service::state::State::Uninitialized => {
                    crate::max_runtime::LspPhase::Uninitialized
                }
                crate::service::state::State::Initializing => {
                    crate::max_runtime::LspPhase::Initializing
                }
                crate::service::state::State::Initialized => {
                    crate::max_runtime::LspPhase::Initialized
                }
                crate::service::state::State::ShutDown => crate::max_runtime::LspPhase::ShutDown,
                crate::service::state::State::Exited => crate::max_runtime::LspPhase::Exited,
            };
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

    /// Sets the parent process ID.
    pub fn set_parent_pid(&self, pid: u32) {
        *self.parent_pid.lock().unwrap() = Some(pid);
    }

    /// Gets the parent process ID.
    pub fn get_parent_pid(&self) -> Option<u32> {
        *self.parent_pid.lock().unwrap()
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
                    let next = m.admit_initialize(client_caps);
                    let receipt = crate::max_runtime::TypestateKernel::receipt(&next);
                    *lock = StateMachine::Initializing(next);
                    if let Ok(mut reg) = crate::get_registry().lock() {
                        reg.current_state = State::Initializing;
                        reg.receipts
                            .insert(receipt.receipt_id.clone(), receipt.clone());
                    }
                    if let Some(instance) = self.mesh.lock().unwrap().instances.get_mut("LSP_1") {
                        instance.phase = crate::max_runtime::LspPhase::Initializing;
                        instance.receipts.push(receipt.clone());
                    }

                    let mut mesh = self.mesh.lock().unwrap();
                    mesh.dispatch_event(crate::max_protocol::HookEvent::StateTransition {
                        instance_id: tower_lsp_max_runtime::InstanceId::from("LSP_1"),
                        from_phase: "Uninitialized".to_string(),
                        to_phase: "Initializing".to_string(),
                    });
                    mesh.dispatch_event(crate::max_protocol::HookEvent::ReceiptEmitted {
                        instance_id: tower_lsp_max_runtime::InstanceId::from("LSP_1"),
                        receipt,
                    });
                    let _ = mesh.verify_instance_ledger("LSP_1");
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
        println!("--- transition_to_initialized entered");
        let mut lock = self.machine.lock().unwrap();
        println!(
            "--- transition_to_initialized state machine before: {:?}",
            lock.get_state()
        );
        let (success, receipt) = match &*lock {
            StateMachine::Initializing(_) => {
                let old = std::mem::replace(
                    &mut *lock,
                    StateMachine::Exited(Machine::new(Exited, EmptyData::default())),
                );
                if let StateMachine::Initializing(m) = old {
                    let next = m.admit_initialized(server_caps);
                    let receipt = crate::max_runtime::TypestateKernel::receipt(&next);
                    *lock = StateMachine::Initialized(next);
                    if let Some(instance) = self.mesh.lock().unwrap().instances.get_mut("LSP_1") {
                        instance.phase = crate::max_runtime::LspPhase::Initialized;
                        instance.receipts.push(receipt.clone());
                    }
                    (true, Some(receipt))
                } else {
                    unreachable!()
                }
            }
            _ => (false, None),
        };
        println!("--- transition_to_initialized success: {}", success);
        if success {
            if let Ok(mut reg) = crate::get_registry().lock() {
                reg.current_state = State::Initialized;
                if let Some(ref r) = receipt {
                    reg.receipts.insert(r.receipt_id.clone(), r.clone());
                }
            }
            if let Some(ref r) = receipt {
                let mut mesh = self.mesh.lock().unwrap();
                mesh.dispatch_event(crate::max_protocol::HookEvent::StateTransition {
                    instance_id: tower_lsp_max_runtime::InstanceId::from("LSP_1"),
                    from_phase: "Initializing".to_string(),
                    to_phase: "Initialized".to_string(),
                });
                mesh.dispatch_event(crate::max_protocol::HookEvent::ReceiptEmitted {
                    instance_id: tower_lsp_max_runtime::InstanceId::from("LSP_1"),
                    receipt: r.clone(),
                });
                let _ = mesh.verify_instance_ledger("LSP_1");
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
                if let Some(instance) = self.mesh.lock().unwrap().instances.get_mut("LSP_1") {
                    instance.phase = crate::max_runtime::LspPhase::Uninitialized;
                    instance.receipts.truncate(1);
                }
                true
            }
            _ => false,
        };
        if success {
            if let Ok(mut reg) = crate::get_registry().lock() {
                reg.current_state = State::Uninitialized;
                reg.receipts.retain(|k, _| k == "rcpt-uninitialized");
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
        let (success, receipt) = match &*lock {
            StateMachine::Initialized(_) => {
                let old = std::mem::replace(
                    &mut *lock,
                    StateMachine::Exited(Machine::new(Exited, EmptyData::default())),
                );
                if let StateMachine::Initialized(m) = old {
                    let next = m.admit_shutdown();
                    let receipt = crate::max_runtime::TypestateKernel::receipt(&next);
                    *lock = StateMachine::ShutDown(next);
                    if let Some(instance) = self.mesh.lock().unwrap().instances.get_mut("LSP_1") {
                        instance.phase = crate::max_runtime::LspPhase::ShutDown;
                        instance.receipts.push(receipt.clone());
                    }
                    (true, Some(receipt))
                } else {
                    unreachable!()
                }
            }
            _ => (false, None),
        };
        if success {
            if let Ok(mut reg) = crate::get_registry().lock() {
                reg.current_state = State::ShutDown;
                if let Some(ref r) = receipt {
                    reg.receipts.insert(r.receipt_id.clone(), r.clone());
                }
            }
            if let Some(ref r) = receipt {
                let mut mesh = self.mesh.lock().unwrap();
                mesh.dispatch_event(crate::max_protocol::HookEvent::StateTransition {
                    instance_id: tower_lsp_max_runtime::InstanceId::from("LSP_1"),
                    from_phase: "Initialized".to_string(),
                    to_phase: "ShutDown".to_string(),
                });
                mesh.dispatch_event(crate::max_protocol::HookEvent::ReceiptEmitted {
                    instance_id: tower_lsp_max_runtime::InstanceId::from("LSP_1"),
                    receipt: r.clone(),
                });
                let _ = mesh.verify_instance_ledger("LSP_1");
            }
        }
        success
    }

    /// Transitions to `Exited` phase.
    pub fn transition_to_exited(&self) -> bool {
        let mut lock = self.machine.lock().unwrap();
        let (res, receipt) = match &*lock {
            StateMachine::ShutDown(_m) => {
                let old = std::mem::replace(
                    &mut *lock,
                    StateMachine::Exited(Machine::new(Exited, EmptyData::default())),
                );
                if let StateMachine::ShutDown(m) = old {
                    let next = m.admit_exit();
                    let receipt = crate::max_runtime::TypestateKernel::receipt(&next);
                    *lock = StateMachine::Exited(next);
                    self.set_exit_code(0);
                    if let Some(instance) = self.mesh.lock().unwrap().instances.get_mut("LSP_1") {
                        instance.phase = crate::max_runtime::LspPhase::Exited;
                        instance.receipts.push(receipt.clone());
                    }
                    (true, Some(receipt))
                } else {
                    unreachable!()
                }
            }
            _ => {
                *lock = StateMachine::Exited(Machine::new(Exited, EmptyData::default()));
                self.set_exit_code(1);
                if let Some(instance) = self.mesh.lock().unwrap().instances.get_mut("LSP_1") {
                    instance.phase = crate::max_runtime::LspPhase::Exited;
                }
                (true, None)
            }
        };
        if let Ok(mut reg) = crate::get_registry().lock() {
            reg.current_state = State::Exited;
            if let Some(ref r) = receipt {
                reg.receipts.insert(r.receipt_id.clone(), r.clone());
            }
        }
        if let Some(ref r) = receipt {
            let mut mesh = self.mesh.lock().unwrap();
            mesh.dispatch_event(crate::max_protocol::HookEvent::StateTransition {
                instance_id: tower_lsp_max_runtime::InstanceId::from("LSP_1"),
                from_phase: "ShutDown".to_string(),
                to_phase: "Exited".to_string(),
            });
            mesh.dispatch_event(crate::max_protocol::HookEvent::ReceiptEmitted {
                instance_id: tower_lsp_max_runtime::InstanceId::from("LSP_1"),
                receipt: r.clone(),
            });
            let _ = mesh.verify_instance_ledger("LSP_1");
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
