use sha3::{Digest, Keccak256};

pub struct SeededRng {
    state: [u8; 32],
    position: usize,
}

#[allow(dead_code)]
impl SeededRng {
    pub fn new(seed: &str) -> Self {
        let mut hasher = Keccak256::new();
        hasher.update(seed.as_bytes());
        let state: [u8; 32] = hasher.finalize().into();
        Self { state, position: 0 }
    }

    pub fn update(&self, seed: &str) -> Self {
        let mut hasher = Keccak256::new();
        hasher.update(self.state);
        hasher.update(seed.as_bytes());
        let new_state: [u8; 32] = hasher.finalize().into();
        Self {
            state: new_state,
            position: 0,
        }
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            state: bytes,
            position: 0,
        }
    }

    fn rehash(&mut self) {
        let mut hasher = Keccak256::new();
        hasher.update(self.state);
        self.state = hasher.finalize().into();
        self.position = 0;
    }

    fn next_u8(&mut self) -> u8 {
        if self.position >= 32 {
            self.rehash();
        }
        let val = self.state[self.position];
        self.position += 1;
        val
    }

    fn next_u32(&mut self) -> u32 {
        let b0 = self.next_u8() as u32;
        let b1 = self.next_u8() as u32;
        let b2 = self.next_u8() as u32;
        let b3 = self.next_u8() as u32;
        (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
    }

    fn next_u64(&mut self) -> u64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (hi << 32) | lo
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    pub fn random_range_u32(&mut self, range: std::ops::Range<u32>) -> u32 {
        let len = range.end - range.start;
        if len == 0 {
            return range.start;
        }
        range.start + (self.next_u32() % len)
    }

    pub fn random_range_u8(&mut self, range: std::ops::Range<u8>) -> u8 {
        let len = range.end - range.start;
        if len == 0 {
            return range.start;
        }
        range.start + (self.next_u8() % len)
    }

    pub fn random_range_i32(&mut self, range: std::ops::Range<i32>) -> i32 {
        let len = (range.end - range.start) as u32;
        if len == 0 {
            return range.start;
        }
        range.start + (self.next_u32() % len) as i32
    }

    pub fn random_range_usize(&mut self, range: std::ops::Range<usize>) -> usize {
        let len = range.end - range.start;
        if len == 0 {
            return range.start;
        }
        range.start + (self.next_u64() as usize % len)
    }

    pub fn random_range_f64(&mut self, range: std::ops::Range<f64>) -> f64 {
        let len = range.end - range.start;
        range.start + self.next_f64() * len
    }

    pub fn random_range_f64_inclusive(&mut self, range: std::ops::RangeInclusive<f64>) -> f64 {
        let start = *range.start();
        let end = *range.end();
        let len = end - start;
        start + self.next_f64() * len
    }

    pub fn random_bool(&mut self, probability: f64) -> bool {
        self.next_f64() < probability
    }

    pub fn state(&self) -> [u8; 32] {
        self.state
    }
}

pub fn keccak256(seed: &str) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(seed.as_bytes());
    hasher.finalize().into()
}

pub fn derive_seed(base: &[u8; 32], context: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(base);
    hasher.update(context);
    hasher.finalize().into()
}
