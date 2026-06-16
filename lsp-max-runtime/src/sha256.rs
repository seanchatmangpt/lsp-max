pub fn sha256(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut padded = data.to_vec();
    let original_len_bits = (data.len() as u64) * 8;
    padded.push(0x80);
    // Keep padding until length + 8 is a multiple of 64
    while (padded.len() + 8) % 64 != 0 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&original_len_bits.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }

        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h_val
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut result = String::with_capacity(64);
    for val in h.iter() {
        result.push_str(&format!("{:08x}", val));
    }
    result
}

pub fn validate_and_reconstruct_chain_checked(
    history: &[lsp_max_protocol::Receipt],
) -> Result<(serde_json::Value, serde_json::Value), String> {
    if history.is_empty() {
        return Err("History must not be empty".to_string());
    }

    let r0 = &history[0];
    if r0.receipt_id != "rcpt-uninitialized" {
        return Err(format!(
            "Expected receipt_id 'rcpt-uninitialized' at index 0, got '{}'",
            r0.receipt_id
        ));
    }
    let mut expected_hash = sha256(r0.receipt_id.as_bytes());
    if r0.hash != expected_hash {
        return Err(format!(
            "Hash mismatch at index 0: expected '{}', got '{}'",
            expected_hash, r0.hash
        ));
    }

    let mut client_caps = serde_json::Value::Null;
    let mut server_caps = serde_json::Value::Null;

    if history.len() > 1 {
        let r1 = &history[1];
        if !r1
            .receipt_id
            .starts_with("rcpt-uninitialized-to-initializing:")
        {
            return Err(format!(
                "Invalid receipt ID at index 1: expected prefix 'rcpt-uninitialized-to-initializing:', got '{}'",
                r1.receipt_id
            ));
        }
        let prefix_len = "rcpt-uninitialized-to-initializing:".len();
        let json_str = &r1.receipt_id[prefix_len..];
        client_caps = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse client capabilities at index 1: {}", e))?;

        expected_hash = sha256(format!("{}:{}", expected_hash, r1.receipt_id).as_bytes());
        if r1.hash != expected_hash {
            return Err(format!(
                "Hash mismatch at index 1: expected '{}', got '{}'",
                expected_hash, r1.hash
            ));
        }
    }

    if history.len() > 2 {
        let r2 = &history[2];
        if !r2
            .receipt_id
            .starts_with("rcpt-initializing-to-initialized:")
        {
            return Err(format!(
                "Invalid receipt ID at index 2: expected prefix 'rcpt-initializing-to-initialized:', got '{}'",
                r2.receipt_id
            ));
        }
        let prefix_len = "rcpt-initializing-to-initialized:".len();
        let json_str = &r2.receipt_id[prefix_len..];
        server_caps = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse server capabilities at index 2: {}", e))?;

        expected_hash = sha256(format!("{}:{}", expected_hash, r2.receipt_id).as_bytes());
        if r2.hash != expected_hash {
            return Err(format!(
                "Hash mismatch at index 2: expected '{}', got '{}'",
                expected_hash, r2.hash
            ));
        }
    }

    if history.len() > 3 {
        let r3 = &history[3];
        if r3.receipt_id != "rcpt-initialized-to-shutdown" {
            return Err(format!(
                "Expected receipt_id 'rcpt-initialized-to-shutdown' at index 3, got '{}'",
                r3.receipt_id
            ));
        }
        expected_hash = sha256(format!("{}:{}", expected_hash, r3.receipt_id).as_bytes());
        if r3.hash != expected_hash {
            return Err(format!(
                "Hash mismatch at index 3: expected '{}', got '{}'",
                expected_hash, r3.hash
            ));
        }
    }

    if history.len() > 4 {
        let r4 = &history[4];
        if r4.receipt_id != "rcpt-shutdown-to-exited" {
            return Err(format!(
                "Expected receipt_id 'rcpt-shutdown-to-exited' at index 4, got '{}'",
                r4.receipt_id
            ));
        }
        expected_hash = sha256(format!("{}:{}", expected_hash, r4.receipt_id).as_bytes());
        if r4.hash != expected_hash {
            return Err(format!(
                "Hash mismatch at index 4: expected '{}', got '{}'",
                expected_hash, r4.hash
            ));
        }
    }

    if history.len() > 5 {
        return Err(format!(
            "History contains {} unexpected items beyond the Exited state (max 5)",
            history.len() - 5
        ));
    }

    Ok((client_caps, server_caps))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- sha256 core ----

    #[test]
    fn sha256_empty_known_vector() {
        // SHA-256 of zero bytes is a fixed standard value.
        let h = sha256(b"");
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_output_is_64_hex_chars() {
        let h = sha256(b"");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sha256_hello_known_vector() {
        let h = sha256(b"hello");
        assert_eq!(
            h,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_deterministic() {
        assert_eq!(sha256(b"hello"), sha256(b"hello"));
    }

    #[test]
    fn sha256_differs_on_different_input() {
        assert_ne!(sha256(b"hello"), sha256(b"world"));
    }

    // ---- validate_and_reconstruct_chain_checked ----

    fn make_receipt(id: &str, hash: &str) -> lsp_max_protocol::Receipt {
        lsp_max_protocol::Receipt {
            receipt_id: id.to_string(),
            hash: hash.to_string(),
            prev_receipt_hash: None,
        }
    }

    #[test]
    fn chain_empty_history_is_err() {
        let result = validate_and_reconstruct_chain_checked(&[]);
        assert!(result.is_err(), "empty history must return Err");
        assert!(result.unwrap_err().contains("must not be empty"));
    }

    #[test]
    fn chain_single_valid_uninitialized_receipt() {
        // A chain with only the genesis receipt should verify.
        let h0 = sha256(b"rcpt-uninitialized");
        let r0 = make_receipt("rcpt-uninitialized", &h0);
        let result = validate_and_reconstruct_chain_checked(&[r0]);
        assert!(result.is_ok(), "single-entry genesis chain must be CANDIDATE");
    }

    #[test]
    fn chain_single_wrong_id_is_err() {
        let h = sha256(b"rcpt-wrong");
        let r = make_receipt("rcpt-wrong", &h);
        let result = validate_and_reconstruct_chain_checked(&[r]);
        assert!(result.is_err(), "wrong genesis receipt_id must return Err");
    }

    #[test]
    fn chain_hash_mismatch_is_err() {
        // Correct receipt_id but tampered hash.
        let r = make_receipt(
            "rcpt-uninitialized",
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        let result = validate_and_reconstruct_chain_checked(&[r]);
        assert!(result.is_err(), "hash mismatch must return Err");
        assert!(result.unwrap_err().contains("Hash mismatch"));
    }

    #[test]
    fn chain_two_entry_valid() {
        // Genesis + Initializing transition with empty client caps.
        let h0 = sha256(b"rcpt-uninitialized");
        let r0 = make_receipt("rcpt-uninitialized", &h0);
        let rcpt1_id = "rcpt-uninitialized-to-initializing:{}";
        let h1 = sha256(format!("{}:{}", h0, rcpt1_id).as_bytes());
        let r1 = make_receipt(rcpt1_id, &h1);
        let result = validate_and_reconstruct_chain_checked(&[r0, r1]);
        assert!(result.is_ok(), "two-entry valid chain must be CANDIDATE");
        let (client_caps, _) = result.unwrap();
        // Empty JSON object was embedded in the receipt_id.
        assert_eq!(client_caps, serde_json::json!({}));
    }

    #[test]
    fn chain_excess_entries_is_err() {
        // Build a valid 5-entry chain then add a 6th.
        let h0 = sha256(b"rcpt-uninitialized");
        let r0 = make_receipt("rcpt-uninitialized", &h0);
        let rcpt1_id = "rcpt-uninitialized-to-initializing:{}";
        let h1 = sha256(format!("{}:{}", h0, rcpt1_id).as_bytes());
        let r1 = make_receipt(rcpt1_id, &h1);
        let rcpt2_id = "rcpt-initializing-to-initialized:{}";
        let h2 = sha256(format!("{}:{}", h1, rcpt2_id).as_bytes());
        let r2 = make_receipt(rcpt2_id, &h2);
        let rcpt3_id = "rcpt-initialized-to-shutdown";
        let h3 = sha256(format!("{}:{}", h2, rcpt3_id).as_bytes());
        let r3 = make_receipt(rcpt3_id, &h3);
        let rcpt4_id = "rcpt-shutdown-to-exited";
        let h4 = sha256(format!("{}:{}", h3, rcpt4_id).as_bytes());
        let r4 = make_receipt(rcpt4_id, &h4);
        // Extra receipt beyond the protocol maximum.
        let r_extra = make_receipt("rcpt-unknown-extra", "aaaa");
        let result = validate_and_reconstruct_chain_checked(&[r0, r1, r2, r3, r4, r_extra]);
        assert!(result.is_err(), "6-entry chain must return Err");
    }
}
