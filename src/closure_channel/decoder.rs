use super::diagnostics::LSPMAX_PROSE_CLOSURE_TOKEN_REFUSED;
use super::grammar::PROSE_CLOSURE_TOKENS;
use super::packet::ClosurePacket;

pub struct DecoderResult {
    pub q: u8,
    pub diagnostics: Vec<String>,
}

pub fn decode(text: &str, packet: &ClosurePacket) -> DecoderResult {
    let mut diagnostics = Vec::new();

    // Extract text outside of string boundaries
    let mut in_string = false;
    let mut escaped = false;
    let mut outside_text = String::new();

    for c in text.chars() {
        if escaped {
            escaped = false;
            if !in_string {
                outside_text.push(c);
            }
            continue;
        }
        if c == '\\' {
            escaped = true;
            if !in_string {
                outside_text.push(c);
            }
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            outside_text.push(' ');
            continue;
        }
        if !in_string {
            outside_text.push(c);
        }
    }

    let lower_outside = outside_text.to_lowercase();
    let mut refused = false;
    for token in PROSE_CLOSURE_TOKENS {
        if lower_outside.contains(&token.to_lowercase()) {
            refused = true;
            break;
        }
    }

    if refused {
        diagnostics.push(LSPMAX_PROSE_CLOSURE_TOKEN_REFUSED.to_string());
        return DecoderResult { q: 0, diagnostics };
    }

    let valid_q = packet.f_t == 0 && packet.w_t == 1 && packet.c_t == 1 && !packet.r_b.is_empty();

    DecoderResult {
        q: if valid_q { 1 } else { 0 },
        diagnostics,
    }
}
