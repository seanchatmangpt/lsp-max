use super::*;

#[test]
fn test_typestate_lifecycle() {
    let machine = Machine::new(Uninitialized, EmptyData::default());
    let rcpt_uninit = machine.receipt();
    assert_eq!(rcpt_uninit.receipt_id, "rcpt-uninitialized");
    assert_eq!(rcpt_uninit.hash, sha256(b"rcpt-uninitialized"));

    let client_caps = serde_json::json!({"textDocument": {"hover": {}}});
    let machine = machine.admit_initialize(client_caps.clone());
    assert_eq!(machine.data.client_capabilities, client_caps);
    let rcpt_initializing = machine.receipt();
    let expected_init_id = format!(
        "rcpt-uninitialized-to-initializing:{}",
        serde_json::to_string(&client_caps).unwrap_or_else(|_| "<unserializable>".to_string())
    );
    assert_eq!(rcpt_initializing.receipt_id, expected_init_id);
    let expected_init_hash =
        sha256(format!("{}:{}", rcpt_uninit.hash, expected_init_id).as_bytes());
    assert_eq!(rcpt_initializing.hash, expected_init_hash);

    let server_caps = serde_json::json!({"hoverProvider": true});
    let machine = machine.admit_initialized(server_caps.clone());
    assert_eq!(machine.data.client_capabilities, client_caps);
    assert_eq!(machine.data.server_capabilities, server_caps);
    let rcpt_initialized = machine.receipt();
    let expected_initialized_id = format!(
        "rcpt-initializing-to-initialized:{}",
        serde_json::to_string(&server_caps).unwrap_or_else(|_| "<unserializable>".to_string())
    );
    assert_eq!(rcpt_initialized.receipt_id, expected_initialized_id);
    let expected_initialized_hash =
        sha256(format!("{}:{}", rcpt_initializing.hash, expected_initialized_id).as_bytes());
    assert_eq!(rcpt_initialized.hash, expected_initialized_hash);

    let machine = machine.admit_shutdown();
    let rcpt_shutdown = machine.receipt();
    let expected_shutdown_id = "rcpt-initialized-to-shutdown";
    assert_eq!(rcpt_shutdown.receipt_id, expected_shutdown_id);
    let expected_shutdown_hash =
        sha256(format!("{}:{}", rcpt_initialized.hash, expected_shutdown_id).as_bytes());
    assert_eq!(rcpt_shutdown.hash, expected_shutdown_hash);

    let machine = machine.admit_exit();
    let rcpt_exited = machine.receipt();
    let expected_exited_id = "rcpt-shutdown-to-exited";
    assert_eq!(rcpt_exited.receipt_id, expected_exited_id);
    let expected_exited_hash =
        sha256(format!("{}:{}", rcpt_shutdown.hash, expected_exited_id).as_bytes());
    assert_eq!(rcpt_exited.hash, expected_exited_hash);
}

#[test]
fn test_kernel_admit_replay() {
    let machine_uninit = Machine::new(Uninitialized, EmptyData::default());
    let rcpt_0 = machine_uninit.receipt();

    let client_caps = serde_json::json!({"textDocument": {"completion": {}}});
    let machine_initializing =
        <Machine<AccessAdmissionLaw, Uninitialized, EmptyData> as TypestateKernel<_, _, _>>::admit(
            machine_uninit,
            client_caps.clone(),
        )
        .unwrap();
    let rcpt_1 = machine_initializing.receipt();

    let server_caps = serde_json::json!({"completionProvider": {}});
    let machine_initialized =
        <Machine<AccessAdmissionLaw, Initializing, InitializingData> as TypestateKernel<
            _,
            _,
            _,
        >>::admit(machine_initializing, server_caps.clone())
        .unwrap();
    let rcpt_2 = machine_initialized.receipt();

    let machine_shutdown =
        <Machine<AccessAdmissionLaw, Initialized, InitializedData> as TypestateKernel<
            _,
            _,
            _,
        >>::admit(machine_initialized, ())
        .unwrap();
    let rcpt_3 = machine_shutdown.receipt();

    let machine_exited = <Machine<AccessAdmissionLaw, ShutDown, EmptyData> as TypestateKernel<
        _,
        _,
        _,
    >>::admit(machine_shutdown, ())
    .unwrap();
    let rcpt_4 = machine_exited.receipt();

    let history = vec![
        rcpt_0.clone(),
        rcpt_1.clone(),
        rcpt_2.clone(),
        rcpt_3.clone(),
        rcpt_4.clone(),
    ];

    let replayed_exited =
        <Machine<AccessAdmissionLaw, Exited, EmptyData> as TypestateKernel<_, _, _>>::replay(
            history.clone(),
        );
    let replayed_exited_ok = replayed_exited.expect("replay must succeed");
    assert_eq!(
        replayed_exited_ok
            .data
            .client_capabilities
            .as_ref()
            .unwrap(),
        &client_caps
    );
    assert_eq!(
        replayed_exited_ok
            .data
            .server_capabilities
            .as_ref()
            .unwrap(),
        &server_caps
    );

    let mut tampered_history = history.clone();
    tampered_history[1].receipt_id = "rcpt-uninitialized-to-initializing:{}".to_string();
    let tampered_result =
        <Machine<AccessAdmissionLaw, Exited, EmptyData> as TypestateKernel<_, _, _>>::replay(
            tampered_history,
        );
    assert!(
        tampered_result.is_err(),
        "Replaying a tampered receipt history must return Err"
    );

    let mut tampered_history_hash = history.clone();
    tampered_history_hash[2].hash = "wrong_hash".to_string();
    let tampered_hash_result =
        <Machine<AccessAdmissionLaw, Exited, EmptyData> as TypestateKernel<_, _, _>>::replay(
            tampered_history_hash,
        );
    assert!(
        tampered_hash_result.is_err(),
        "Replaying a tampered hash must return Err"
    );
}
