//! Seeded pseudo-random number generator (xoshiro256\*\*).
//!
//! A self-contained, `no_std`-compatible PRNG so the crate has no external
//! rand dependency. Seed any `u64`; the state is then expanded with splitmix64
//! to avoid the all-zero trap.

#[derive(Clone, Debug)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    /// Create a new generator from a 64-bit seed.
    pub fn new(seed: u64) -> Self {
        // Use splitmix64 to fill the four state words.
        let mut x = seed;
        let mut s = [0u64; 4];
        for word in s.iter_mut() {
            x = x.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            *word = z ^ (z >> 31);
        }
        Self { s }
    }

    /// Return the next raw 64-bit integer.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform float in `[0, 1)`.
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        // Use the upper 53 bits.
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform integer in `[0, n)`.
    #[inline]
    pub fn next_usize(&mut self, n: usize) -> usize {
        assert!(n > 0);
        // Rejection-free: use 128-bit multiplication trick.
        let mut x = self.next_u64();
        let mut m = (x as u128).wrapping_mul(n as u128);
        let mut lo = m as u64;
        if lo < n as u64 {
            let threshold = (n as u64).wrapping_neg() % (n as u64);
            while lo < threshold {
                x = self.next_u64();
                m = (x as u128).wrapping_mul(n as u128);
                lo = m as u64;
            }
        }
        (m >> 64) as usize
    }

    /// Shuffle a slice in-place (Fisher-Yates).
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.next_usize(i + 1);
            slice.swap(i, j);
        }
    }
}
