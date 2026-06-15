//! A tiny, deterministic, dependency-free PRNG (SplitMix64).
//!
//! The whole world is driven from a single seed, so a given seed always
//! produces the same chronicle. That makes surprising histories reproducible:
//! find a seed you like and share it.

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid the degenerate all-zero state.
        Rng { state: seed ^ 0x9E37_79B9_7F4A_7C15 }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        // SplitMix64.
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform f32 in [0.0, 1.0).
    #[inline]
    pub fn f32(&mut self) -> f32 {
        // 24 bits of randomness mapped into the unit interval.
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// True with probability `p`.
    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    /// Uniform integer in [0, n).
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
}
