use super::MuTable;
use std::ops::{Add, Sub};

#[derive(Clone, Copy)]
pub struct Sample(i8, i8);

impl Add<Sample> for Sample {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1)
    }
}

impl Sub<Sample> for Sample {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0, self.1 - other.1)
    }
}

impl Sample {
    pub fn new(l: i8, r: i8) -> Self {
        Self(l, r)
    }

    pub fn mu(&self, mu_table: &MuTable) -> USample {
        USample(mu_table.mu(self.0), mu_table.mu(self.1))
    }

    pub fn hypot(&self) -> u64 {
        let x = self.0.unsigned_abs() as u64;
        let y = self.1.unsigned_abs() as u64;

        x * x + y * y
    }

    pub fn l(&self) -> i8 {
        self.0
    }

    pub fn r(&self) -> i8 {
        self.1
    }
}

#[derive(Clone, Copy)]
pub struct USample(u8, u8);

impl Add<USample> for USample {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1)
    }
}

impl Sub<USample> for USample {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0, self.1 - other.1)
    }
}

impl USample {
    pub fn unmu(&self, mu_table: &MuTable) -> Sample {
        Sample(mu_table.unmu(self.0), mu_table.unmu(self.1))
    }

    pub fn l(&self) -> usize {
        self.0 as usize
    }

    pub fn r(&self) -> usize {
        self.1 as usize
    }
}
