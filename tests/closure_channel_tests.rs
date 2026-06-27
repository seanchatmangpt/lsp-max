use lsp_max::closure_channel::decoder::decode;
use lsp_max::closure_channel::diagnostics::LSPMAX_PROSE_CLOSURE_TOKEN_REFUSED;
use lsp_max::closure_channel::packet::ClosurePacket;

fn valid_packet() -> ClosurePacket {
    ClosurePacket {
        status: "ADMITTED".to_string(),
        f_t: 0,
        w_t: 1,
        c_t: 1,
        r_b: vec!["receipts/v26.6.28-closure-channel.receipt.json".to_string()],
    }
}

#[test]
fn prose_status_admitted_decodes_zero() {
    let packet = valid_packet();
    let text = "The status is ADMITTED.";
    let res = decode(text, &packet);
    assert_eq!(res.q, 0);
    assert!(res
        .diagnostics
        .contains(&LSPMAX_PROSE_CLOSURE_TOKEN_REFUSED.to_string()));
}

#[test]
fn boxed_admitted_text_decodes_zero() {
    let packet = valid_packet();
    let text = "[ADMITTED]";
    let res = decode(text, &packet);
    assert_eq!(res.q, 0);
    assert!(res
        .diagnostics
        .contains(&LSPMAX_PROSE_CLOSURE_TOKEN_REFUSED.to_string()));
}

#[test]
fn noiseless_claim_decodes_zero() {
    let packet = valid_packet();
    let text = "This is a noiseless output.";
    let res = decode(text, &packet);
    assert_eq!(res.q, 0);
    assert!(res
        .diagnostics
        .contains(&LSPMAX_PROSE_CLOSURE_TOKEN_REFUSED.to_string()));
}

#[test]
fn valid_packet_decodes_q() {
    let packet = valid_packet();
    let text = r#"{"status": "ADMITTED", "f_t": 0, "w_t": 1, "c_t": 1, "r_b": ["x"]}"#;
    let res = decode(text, &packet);
    assert_eq!(res.q, 1);
    assert!(res.diagnostics.is_empty());
}

#[test]
fn q_one_requires_zero_failset() {
    let mut packet = valid_packet();
    packet.f_t = 1;
    let text = "{}";
    let res = decode(text, &packet);
    assert_eq!(res.q, 0);
}

#[test]
fn q_one_requires_receipts() {
    let mut packet = valid_packet();
    packet.r_b = vec![];
    let text = "{}";
    let res = decode(text, &packet);
    assert_eq!(res.q, 0);
}

#[test]
fn q_one_requires_witness_vector() {
    let mut packet = valid_packet();
    packet.w_t = 0;
    let text = "{}";
    let res = decode(text, &packet);
    assert_eq!(res.q, 0);
}

#[test]
fn q_one_requires_counterfactual_vector() {
    let mut packet = valid_packet();
    packet.c_t = 0;
    let text = "{}";
    let res = decode(text, &packet);
    assert_eq!(res.q, 0);
}

#[test]
fn mixed_math_plus_prose_refused() {
    let packet = valid_packet();
    let text = r#"Here is the mathematically closed result: {"status": "ADMITTED", "f_t": 0, "w_t": 1, "c_t": 1, "r_b": ["x"]}"#;
    let res = decode(text, &packet);
    assert_eq!(res.q, 0);
    assert!(res
        .diagnostics
        .contains(&LSPMAX_PROSE_CLOSURE_TOKEN_REFUSED.to_string()));
}
