use rand_core::{CryptoRng, RngCore};
use uuid::Uuid;

/// A simple, fast, and seedable pseudo-random number generator implementing `RngCore`
/// to provide deterministic entropy stubbing without external runtime dependencies.
#[derive(Debug, Clone)]
pub struct XorshiftRng {
    state: u64,
}

impl XorshiftRng {
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0xDEADC0DE } else { seed };
        Self { state }
    }
}

impl RngCore for XorshiftRng {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand_core::impls::fill_bytes_via_next(self, dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl CryptoRng for XorshiftRng {}

/// Generates a RFC 4122 compliant UUID v4 deterministically using the provided RNG.
pub fn deterministic_uuid(rng: &mut impl RngCore) -> Uuid {
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    // Set version to 4
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    // Set variant to RFC 4122
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}
