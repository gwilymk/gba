use quote::quote;

use agb_xm::{parse, SampleData};

const DEFAULT_FREQUENCY: i32 = 8363;

pub fn generate(xm_data: &[u8], file_path: &str, gba_frequency: u32) -> proc_macro2::TokenStream {
    let song = parse(xm_data).expect("Failed to load XM song data");

    let samples = song.instruments.iter().flat_map(|instrument| {
        instrument.samples.iter().map(|sample| {
            let mut sample_data = match &sample.sample_data {
                SampleData::Bits8(bits) => bits.iter().map(|&bit| bit as u8).collect::<Vec<_>>(),
                _ => unimplemented!("Currently only support 8 bit samples"),
            };

            let should_loop = sample.sample_type & 1 != 0;

            if should_loop && sample_data.len() < 304 * 5 {
                sample_data = sample_data.repeat(304 * 5 / sample_data.len() + 1);
            }

            quote! {
                Sample::new(&[#(#sample_data),*], #should_loop)
            }
        })
    });

    let num_channels = song.header.num_channels as u8;
    let pattern_hz = (song.header.default_bpm as f32) * 2.0 / 5.0;
    let lines_per_second = pattern_hz / (song.header.default_tempo as f32);

    let frames_per_line = 60f32 / lines_per_second;
    let frames_per_line = frames_per_line as u8;

    let notes = song.patterns.iter().map(|pattern| {
        let notes = pattern.notes.iter().map(|note| {
            let pattern_note = note.note as i16;

            if note.note == 97 {
                return quote! {
                    Note::new(1, 0)
                };
            } else if note.instrument == 0 || pattern_note == 0 {
                return quote! {
                    Note::new(0, 0)
                };
            }

            let instrument = &song.instruments[(note.instrument - 1) as usize];
            let sample = instrument.sample_header.as_ref().unwrap().sample_number
                [(pattern_note - 1) as usize];

            let sample_offset: u8 = song
                .instruments
                .iter()
                .take(note.instrument as usize - 1)
                .map(|i| i.samples.len() as u8)
                .sum();

            let sample_id = sample + sample_offset + 1;
            let sample = &instrument.samples[sample as usize];

            let real_note = pattern_note + (sample.relative_note as i8) as i16;

            let period = 7680 - ((real_note - 1) * 64) - ((sample.fine_tune / 2) as i16);
            let frequency =
                (DEFAULT_FREQUENCY as f32) * 2f32.powf((4608f32 - period as f32) / 768.0);

            let target_frequency = frequency / (gba_frequency as f32);

            let integer = target_frequency.trunc();
            let fractional = target_frequency.fract() * (1u32 << 8) as f32;

            let playback_speed = ((integer as u16) << 8) | fractional as u16;

            quote! {
                Note::new(#sample_id, #playback_speed)
            }
        });

        quote! {
            &[#(#notes),*]
        }
    });

    quote! {
        {
            const _: &[u8] = include_bytes!(#file_path);

            use agb::sound::mixer::tracker::{TrackerMusic, Sample, Note};
            use agb::number::Num;

            const SAMPLES: &[Sample] = &[#(#samples),*];
            const NOTES: &[&[Note]] = &[#(#notes),*];

            TrackerMusic::new(
                SAMPLES,
                NOTES,
                #num_channels,
                #frames_per_line,
            )
        }
    }
}

#[test]
fn should_not_explode() {
    let test_file = include_bytes!("../../agb/examples/algar_-_ninja_on_speed.xm");

    generate(test_file, "some path", 10512);
}
