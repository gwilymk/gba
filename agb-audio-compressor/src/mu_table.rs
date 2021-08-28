pub struct MuTable {
    mu_table: [u8; 256],
    unmu_table: [i8; 16],
}

fn mu(x: f64, mu: f64) -> f64 {
    x.signum() * ((mu * x.abs()).ln_1p() / mu.ln_1p())
}

fn muinv(x: f64, mu: f64) -> f64 {
    x.signum() * ((mu + 1.0).powf(x.abs()) - 1.0) / mu
}

impl MuTable {
    pub fn new(m: f64) -> Self {
        let mut mu_table = [0; 256];
        let mut unmu_table = [0; 16];

        for i in 0..256 {
            mu_table[i as usize] = ((mu(((i - 128) as f64) / 128.0, m) * 8.0).floor() + 8.0) as u8;
        }

        for i in 0..16 {
            unmu_table[i] = (muinv((i as f64 - 8.0) / 8.0, m) * 128.0).ceil() as i8;
        }

        Self {
            mu_table,
            unmu_table,
        }
    }

    pub fn mu(&self, x: i8) -> u8 {
        self.mu_table[(x as i32 + 128) as usize]
    }

    pub fn unmu(&self, x: u8) -> i8 {
        self.unmu_table[x as usize]
    }

    pub fn unmu_table(&self) -> [i8; 16] {
        self.unmu_table
    }
}
