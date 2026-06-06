use blake3::Hasher;
use oxigraph::sparql::QueryResults;
use rand_core::RngCore;
use uuid::Uuid;

use super::rng::{deterministic_uuid, XorshiftRng};

/// Stubbed entropy source for deterministic replay.
#[derive(Debug, Clone)]
pub struct ReplayEntropy {
    rng: XorshiftRng,
}

impl ReplayEntropy {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: XorshiftRng::new(seed),
        }
    }

    pub fn next_uuid(&mut self) -> Uuid {
        deterministic_uuid(&mut self.rng)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    pub fn next_f64(&mut self) -> f64 {
        // Generate a float in [0.0, 1.0)
        let val = self.rng.next_u64();
        (val as f64) / (u64::MAX as f64)
    }
}

/// Stubbed clock source for deterministic replay.
#[derive(Debug, Clone, Copy)]
pub struct ReplayClock {
    now_ms: u64,
}

impl ReplayClock {
    pub const fn new(now_ms: u64) -> Self {
        Self { now_ms }
    }

    /// Formats the milliseconds timestamp into a deterministic UTC ISO 8601 string.
    pub fn now_iso(&self) -> String {
        let seconds = self.now_ms / 1000;
        let days = seconds / 86400;
        let rem_seconds = seconds % 86400;
        let hours = rem_seconds / 3600;
        let rem_minutes = rem_seconds % 3600;
        let minutes = rem_minutes / 60;
        let secs = rem_minutes % 60;

        let mut year = 1970;
        let mut day_of_year = days;
        loop {
            let leap = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                1
            } else {
                0
            };
            let days_in_year = 365 + leap;
            if day_of_year < days_in_year {
                break;
            }
            day_of_year -= days_in_year;
            year += 1;
        }

        let leap = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
            1
        } else {
            0
        };
        let month_days = [31, 28 + leap, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut month = 1;
        for &m_days in month_days.iter() {
            if day_of_year < m_days {
                break;
            }
            day_of_year -= m_days;
            month += 1;
        }
        let day = day_of_year + 1;
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, secs
        )
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '?' || c == '$' || c == ':' || c == '-'
}

/// Preprocesses a SPARQL query to replace non-deterministic function calls
/// (like NOW(), RAND(), UUID(), STRUUID()) with deterministic values.
#[allow(clippy::manual_ignore_case_cmp)]
pub fn preprocess_query(query: &str, clock: &ReplayClock, entropy: &mut ReplayEntropy) -> String {
    let mut res = String::new();
    let mut chars = query.chars().peekable();
    let mut last_char: Option<char> = None;

    while let Some(c) = chars.next() {
        let is_word_boundary = match last_char {
            None => true,
            Some(lc) => !is_ident_char(lc),
        };

        if is_word_boundary && c.to_ascii_lowercase() == 'n' {
            let mut temp = String::new();
            temp.push(c);
            let mut matches = true;
            for expected in ['o', 'w', '(', ')'] {
                if let Some(&next_c) = chars.peek() {
                    if next_c.to_ascii_lowercase() == expected {
                        temp.push(chars.next().unwrap());
                    } else {
                        matches = false;
                        break;
                    }
                } else {
                    matches = false;
                    break;
                }
            }

            let followed_by_boundary = match chars.peek() {
                None => true,
                Some(&next_c) => !is_ident_char(next_c),
            };

            if matches && followed_by_boundary {
                let iso = clock.now_iso();
                let replacement =
                    format!("\"{}\"^^<http://www.w3.org/2001/XMLSchema#dateTime>", iso);
                res.push_str(&replacement);
                last_char = replacement.chars().last();
                continue;
            } else {
                res.push_str(&temp);
                last_char = temp.chars().last();
                continue;
            }
        } else if is_word_boundary && c.to_ascii_lowercase() == 'r' {
            let mut temp = String::new();
            temp.push(c);
            let mut matches = true;
            for expected in ['a', 'n', 'd', '(', ')'] {
                if let Some(&next_c) = chars.peek() {
                    if next_c.to_ascii_lowercase() == expected {
                        temp.push(chars.next().unwrap());
                    } else {
                        matches = false;
                        break;
                    }
                } else {
                    matches = false;
                    break;
                }
            }

            let followed_by_boundary = match chars.peek() {
                None => true,
                Some(&next_c) => !is_ident_char(next_c),
            };

            if matches && followed_by_boundary {
                let val = entropy.next_f64();
                let replacement = format!("{:.8}", val);
                res.push_str(&replacement);
                last_char = replacement.chars().last();
                continue;
            } else {
                res.push_str(&temp);
                last_char = temp.chars().last();
                continue;
            }
        } else if is_word_boundary && c.to_ascii_lowercase() == 'u' {
            let mut temp = String::new();
            temp.push(c);
            let mut matches = true;
            for expected in ['u', 'i', 'd', '(', ')'] {
                if let Some(&next_c) = chars.peek() {
                    if next_c.to_ascii_lowercase() == expected {
                        temp.push(chars.next().unwrap());
                    } else {
                        matches = false;
                        break;
                    }
                } else {
                    matches = false;
                    break;
                }
            }

            let followed_by_boundary = match chars.peek() {
                None => true,
                Some(&next_c) => !is_ident_char(next_c),
            };

            if matches && followed_by_boundary {
                let uuid_str = entropy.next_uuid().to_string();
                let replacement = format!("<urn:uuid:{}>", uuid_str);
                res.push_str(&replacement);
                last_char = replacement.chars().last();
                continue;
            } else {
                res.push_str(&temp);
                last_char = temp.chars().last();
                continue;
            }
        } else if is_word_boundary && c.to_ascii_lowercase() == 's' {
            let mut temp = String::new();
            temp.push(c);
            let mut matches = true;
            for expected in ['t', 'r', 'u', 'u', 'i', 'd', '(', ')'] {
                if let Some(&next_c) = chars.peek() {
                    if next_c.to_ascii_lowercase() == expected {
                        temp.push(chars.next().unwrap());
                    } else {
                        matches = false;
                        break;
                    }
                } else {
                    matches = false;
                    break;
                }
            }

            let followed_by_boundary = match chars.peek() {
                None => true,
                Some(&next_c) => !is_ident_char(next_c),
            };

            if matches && followed_by_boundary {
                let uuid_str = entropy.next_uuid().to_string();
                let replacement = format!("\"{}\"", uuid_str);
                res.push_str(&replacement);
                last_char = replacement.chars().last();
                continue;
            } else {
                res.push_str(&temp);
                last_char = temp.chars().last();
                continue;
            }
        }

        res.push(c);
        last_char = Some(c);
    }
    res
}

/// Serializes and hashes Oxigraph SPARQL QueryResults in a canonical/deterministic order.
pub fn hash_query_results(results: QueryResults) -> Result<String, String> {
    let mut hasher = Hasher::new();
    match results {
        QueryResults::Boolean(b) => {
            hasher.update(b"boolean:");
            hasher.update(if b { b"true" } else { b"false" });
        }
        QueryResults::Solutions(solutions) => {
            hasher.update(b"solutions:");
            let mut serialized_solutions = Vec::new();
            for solution_res in solutions {
                let solution = solution_res.map_err(|e| e.to_string())?;
                let mut bindings = Vec::new();
                for (var, term) in solution.iter() {
                    bindings.push((var.as_str().to_string(), term.to_string()));
                }
                bindings.sort_by(|a, b| a.0.cmp(&b.0));

                let mut sol_str = String::new();
                for (var_name, term_str) in bindings {
                    sol_str.push_str(&var_name);
                    sol_str.push('=');
                    sol_str.push_str(&term_str);
                    sol_str.push(';');
                }
                serialized_solutions.push(sol_str);
            }
            serialized_solutions.sort();
            for sol_str in serialized_solutions {
                hasher.update(sol_str.as_bytes());
            }
        }
        QueryResults::Graph(quads) => {
            hasher.update(b"graph:");
            let mut serialized_quads = Vec::new();
            for quad_res in quads {
                let quad = quad_res.map_err(|e| e.to_string())?;
                serialized_quads.push(quad.to_string());
            }
            serialized_quads.sort();
            for quad_str in serialized_quads {
                hasher.update(quad_str.as_bytes());
            }
        }
    }
    Ok(format!("{}", hasher.finalize().to_hex()))
}
