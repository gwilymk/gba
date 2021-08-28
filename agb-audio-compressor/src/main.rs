mod sample;
use sample::{Sample, USample};

mod mu_table;
use mu_table::MuTable;

use std::env;

fn compress<'a>(samples: impl Iterator<Item = &'a Sample>, m: f64) -> (Vec<USample>, f64) {
    let mut compressed = vec![];

    let mut current_error = 0;
    let mut sample_count = 0;

    let mut previous_sample = Sample::new(0, 0);

    let mu_table = MuTable::new(m);

    for &sample in samples {
        let difference = sample - previous_sample;

        let mued_difference = difference.mu(&mu_table);
        compressed.push(mued_difference);

        previous_sample = previous_sample + mued_difference.unmu(&mu_table);

        current_error += (previous_sample - sample).hypot();
        sample_count += 2;
    }

    (
        compressed,
        ((current_error as f64) / (sample_count as f64)).sqrt(),
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let input = &args[1];
    let output = &args[2];

    let mut reader = hound::WavReader::open(input).unwrap();
    let mut writer = hound::WavWriter::create(output, reader.spec()).unwrap();

    let samples: Vec<_> = reader
        .samples::<i8>()
        .map(|sample| sample.unwrap())
        .collect::<Vec<_>>()
        .chunks_exact(2)
        .map(|samples| Sample::new(samples[0], samples[1]))
        .collect();

    let mut best_mu = 0f64;
    let mut best_rms = f64::MAX;

    let mut best_compressed = vec![];

    for m in 1..128 {
        let (compressed, rms) = compress(samples.iter(), m as f64);

        println!("mu = {}, rms = {}", m, rms);

        if rms < best_rms {
            best_rms = rms;
            best_compressed = compressed;
            best_mu = m as f64;
        }
    }

    println!("Best mu: {}, best rms: {}", best_mu, best_rms);
    let mu_table = MuTable::new(best_mu);

    println!("Mu table: {:?}", mu_table.unmu_table());

    let mut statistics = [0; 16];
    for compressed_sample in best_compressed.iter() {
        statistics[compressed_sample.l()] += 1;
        statistics[compressed_sample.r()] += 1;
    }
    println!("Compressed statistics: {:?}", statistics);

    let mut current_sample = Sample::new(0, 0);
    for compressed_sample in best_compressed.iter() {
        current_sample = current_sample + compressed_sample.unmu(&mu_table);

        writer.write_sample(current_sample.l()).unwrap();
        writer.write_sample(current_sample.r()).unwrap();
    }
}
