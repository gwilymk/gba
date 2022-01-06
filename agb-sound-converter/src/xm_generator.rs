use quote::quote;

use agb_xm::{parse, SampleData};

pub fn generate(xm_data: &[u8]) -> proc_macro2::TokenStream {
    let song = parse(xm_data).expect("Failed to load XM song data");

    let samples = song.instruments.iter().flat_map(|instrument| {
        instrument.samples.iter().map(|sample| {
            let sample_data = match &sample.sample_data {
                SampleData::Bits8(bits) => bits.iter().map(|&bit| bit as u8).collect::<Vec<_>>(),
                _ => unimplemented!("Currently only support 8 bit samples"),
            };

            let should_loop = sample.sample_type & 1 != 0;

            quote! {
                Sample::new(&[#(#sample_data),*], #should_loop)
            }
        })
    });

    let num_channels = song.header.num_channels as u8;
    let initial_speed = song.header.default_bpm as usize;

    quote! {
        {
            use agb::sound::mixer::tracker::{TrackerMusic, Sample, Note};
            use agb::number::Num;

            const SAMPLES: &[Sample] = &[#(#samples),*];
            const NOTES: &[&[Note]] = &[&[Note::new(3, 3)]];

            TrackerMusic::new(
                SAMPLES,
                NOTES,
                #num_channels,
                #initial_speed,
            )
        }
    }
}
