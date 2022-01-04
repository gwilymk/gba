#![deny(clippy::all)]
// Based on information from https://github.com/milkytracker/MilkyTracker/blob/master/resources/reference/xm-form.txt

#[non_exhaustive]
#[derive(Debug)]
pub struct Header<'a> {
    pub module_name: &'a [u8],
    pub tracker_name: &'a [u8],

    pub version_number: u16,

    pub song_length: u16,
    pub song_restart_pos: u16,
    pub num_channels: u16,
    pub num_patterns: u16,
    pub num_instruments: u16,
    pub flags: u16,
    pub default_tempo: u16,
    pub default_bpm: u16,

    pub pattern_order_table: &'a [u8],
}

#[non_exhaustive]
#[derive(Debug, Default)]
pub struct Note {
    pub note: u8,
    pub instrument: u8,
    pub volume: u8,
    pub effect: u8,
    pub parameter: u8,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Pattern {
    pub notes: Vec<Note>,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum SampleData {
    Bits8(Vec<i8>),
    Bits16(Vec<i16>),
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Sample<'a> {
    pub loop_start: u32,
    pub loop_length: u32,

    pub volume: u8,
    pub fine_tune: i8,
    pub sample_type: u8,
    pub panning: u8,

    pub relative_note: u8,
    pub name: &'a [u8],

    pub sample_data: SampleData,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct EnvelopePoint {
    pub frame_number: u16,
    pub value: u16,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Envelope {
    pub points: Vec<EnvelopePoint>,
    pub loop_start: u8,
    pub loop_end: u8,
    pub sustain: u8,
    pub envelope_type: u8,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct SampleHeader<'a> {
    pub sample_number: &'a [u8; 96],

    pub volume_envelope: Envelope,
    pub panning_envelope: Envelope,

    pub vibrato_type: u8,
    pub vibrato_sweep: u8,
    pub vibrato_depth: u8,
    pub vibrato_rate: u8,

    pub volume_fadeout: u16,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Instrument<'a> {
    pub name: &'a [u8],
    pub instrument_type: u8,
    pub samples: Vec<Sample<'a>>,

    pub sample_header: Option<SampleHeader<'a>>,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Song<'a> {
    pub header: Header<'a>,
    pub patterns: Vec<Pattern>,
    pub instruments: Vec<Instrument<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmError {
    HeaderTooShort,
    InvalidHeader,
    UnsupportedVersion(u16),
    InvalidPatternPackingType(u8),
    InvalidPacking { actual: u16, expected: u16 },
}

pub fn parse(xm: &[u8]) -> Result<Song<'_>, XmError> {
    let (header, xm) = parse_header(xm)?;

    let mut xm = xm;
    let mut patterns = vec![];
    for _ in 0..header.num_patterns {
        let (pattern, next_xm) = parse_pattern(xm, header.num_channels)?;
        xm = next_xm;

        patterns.push(pattern);
    }

    let mut instruments = vec![];
    for _ in 0..header.num_instruments {
        let (instrument, next_xm) = parse_instrument(xm)?;
        xm = next_xm;

        instruments.push(instrument);
    }

    Ok(Song {
        header,
        patterns,
        instruments,
    })
}

fn parse_header(xm: &[u8]) -> Result<(Header<'_>, &[u8]), XmError> {
    const HEADER_BASE_SIZE: usize = 64;
    if xm.len() < HEADER_BASE_SIZE {
        return Err(XmError::HeaderTooShort);
    }

    const ID_TEXT_LENGTH: usize = 17;
    let (id_text, xm) = xm.split_at(ID_TEXT_LENGTH);
    if id_text != b"Extended Module: " {
        return Err(XmError::InvalidHeader);
    }

    const MODULE_NAME_LENGTH: usize = 20;
    let (module_name, xm) = xm.split_at(MODULE_NAME_LENGTH);
    let module_name = &module_name[..module_name
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(MODULE_NAME_LENGTH)];

    let (one_a, xm) = xm.split_at(1);
    if one_a[0] != 0x1a {
        return Err(XmError::InvalidHeader);
    }

    const TRACKER_NAME_LENGTH: usize = 20;
    let (tracker_name, xm) = xm.split_at(TRACKER_NAME_LENGTH);
    let tracker_name = &tracker_name[..tracker_name
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(TRACKER_NAME_LENGTH)];

    let (version_number, xm) = read_u16(xm);

    if version_number != 0x0104 {
        return Err(XmError::UnsupportedVersion(version_number));
    }

    let (header_size, xm) = xm.split_at(4);
    let header_size = u32::from_le_bytes(header_size.try_into().unwrap()) as usize;

    let xm_after_header = &xm[header_size - 4..];

    let (song_length, xm) = read_u16(xm);
    let (song_restart_pos, xm) = read_u16(xm);
    let (num_channels, xm) = read_u16(xm);
    let (num_patterns, xm) = read_u16(xm);
    let (num_instruments, xm) = read_u16(xm);
    let (flags, xm) = read_u16(xm);
    let (default_tempo, xm) = read_u16(xm);
    let (default_bpm, xm) = read_u16(xm);

    const PATTERN_ORDER_TABLE_LENGTH: usize = 256;
    let (pattern_order_table, _xm) = xm.split_at(PATTERN_ORDER_TABLE_LENGTH);

    Ok((
        Header {
            module_name,
            tracker_name,

            version_number,

            song_length,
            song_restart_pos,
            num_channels,
            num_patterns,
            num_instruments,
            flags,
            default_tempo,
            default_bpm,

            pattern_order_table,
        },
        xm_after_header,
    ))
}

fn parse_pattern(xm: &[u8], num_channels: u16) -> Result<(Pattern, &[u8]), XmError> {
    let (pattern_header_length, xm) = read_u32(xm);

    let (packing_type, xm) = read_u8(xm);
    if packing_type != 0 {
        return Err(XmError::InvalidPatternPackingType(packing_type));
    }

    let (num_rows, xm) = read_u16(xm);

    let (packed_data_size, xm) = read_u16(xm);

    let xm = &xm[(pattern_header_length - 9) as usize..];

    let mut notes = Vec::with_capacity((num_rows * num_channels) as usize);
    let mut xm_iter = xm[..packed_data_size as usize].iter();
    while let Some(&dbyte) = xm_iter.next() {
        let mut note = Note::default();

        if dbyte & 0x80 != 0 {
            if dbyte & 0x01 != 0 {
                note.note = *xm_iter.next().unwrap();
            }

            if dbyte & 0x02 != 0 {
                note.instrument = *xm_iter.next().unwrap();
            }

            if dbyte & 0x04 != 0 {
                note.volume = *xm_iter.next().unwrap();
            }

            if dbyte & 0x08 != 0 {
                note.effect = *xm_iter.next().unwrap();
            }

            if dbyte & 0x10 != 0 {
                note.parameter = *xm_iter.next().unwrap();
            }
        } else {
            note.note = dbyte;
            note.instrument = *xm_iter.next().unwrap();
            note.volume = *xm_iter.next().unwrap();
            note.effect = *xm_iter.next().unwrap();
            note.parameter = *xm_iter.next().unwrap();
        }

        notes.push(note);
    }

    if notes.len() != (num_rows * num_channels).into() {
        return Err(XmError::InvalidPacking {
            actual: notes.len() as u16,
            expected: num_rows * num_channels,
        });
    }

    Ok((Pattern { notes }, &xm[packed_data_size as usize..]))
}

fn parse_instrument(xm: &[u8]) -> Result<(Instrument<'_>, &[u8]), XmError> {
    let (instrument_header_size, xm) = read_u32(xm);
    let after_instrument = &xm[(instrument_header_size - 4) as usize..];

    const INSTRUMENT_NAME_LENGTH: usize = 22;
    let (instrument_name, xm) = xm.split_at(INSTRUMENT_NAME_LENGTH);

    let (instrument_type, xm) = read_u8(xm);
    let (num_samples, xm) = read_u16(xm);

    let sample_header = if num_samples > 0 {
        let (_sample_header_size, xm) = read_u32(xm);
        let (sample_number, xm) = xm.split_at(96);

        let (volume_envelope_points, xm) = xm.split_at(48);
        let (panning_envelope_points, xm) = xm.split_at(48);

        let (num_volume_points, xm) = read_u8(xm);
        let (num_panning_points, xm) = read_u8(xm);

        let (volume_sustain, xm) = read_u8(xm);
        let (volume_loop_start, xm) = read_u8(xm);
        let (volume_loop_end, xm) = read_u8(xm);

        let (panning_sustain, xm) = read_u8(xm);
        let (panning_loop_start, xm) = read_u8(xm);
        let (panning_loop_end, xm) = read_u8(xm);

        let (volume_type, xm) = read_u8(xm);
        let (panning_type, xm) = read_u8(xm);

        let (vibrato_type, xm) = read_u8(xm);
        let (vibrato_sweep, xm) = read_u8(xm);
        let (vibrato_depth, xm) = read_u8(xm);
        let (vibrato_rate, xm) = read_u8(xm);

        let (volume_fadeout, xm) = read_u16(xm);
        let (_reserved, _xm) = read_u16(xm);

        Some(SampleHeader {
            sample_number: sample_number.try_into().unwrap(),
            volume_envelope: Envelope {
                points: volume_envelope_points
                    .chunks_exact(4)
                    .take(num_volume_points as usize)
                    .map(|point| {
                        let (frame_number, point) = read_u16(point);
                        let (value, _) = read_u16(point);

                        EnvelopePoint {
                            frame_number,
                            value,
                        }
                    })
                    .collect(),
                loop_start: volume_loop_start,
                loop_end: volume_loop_end,
                sustain: volume_sustain,
                envelope_type: volume_type,
            },
            panning_envelope: Envelope {
                points: panning_envelope_points
                    .chunks_exact(4)
                    .take(num_panning_points as usize)
                    .map(|point| {
                        let (frame_number, point) = read_u16(point);
                        let (value, _) = read_u16(point);

                        EnvelopePoint {
                            frame_number,
                            value,
                        }
                    })
                    .collect(),
                loop_start: panning_loop_start,
                loop_end: panning_loop_end,
                sustain: panning_sustain,
                envelope_type: panning_type,
            },

            vibrato_type,
            vibrato_sweep,
            vibrato_depth,
            vibrato_rate,

            volume_fadeout,
        })
    } else {
        None
    };

    let mut samples = vec![];
    let mut next_xm = after_instrument;

    for _ in 0..num_samples {
        let (sample_length, xm) = read_u32(next_xm);
        let (sample_loop_start, xm) = read_u32(xm);
        let (sample_loop_length, xm) = read_u32(xm);
        let (volume, xm) = read_u8(xm);
        let (fine_tune, xm) = read_u8(xm);

        let (sample_type, xm) = read_u8(xm);

        let (panning, xm) = read_u8(xm);
        let (relative_note_number, xm) = read_u8(xm);
        let (_reserved, xm) = read_u8(xm);

        const SAMPLE_NAME_LENGTH: usize = 22;
        let (sample_name, xm) = xm.split_at(SAMPLE_NAME_LENGTH);

        let (sample_data, xm) = if sample_type & (1 << 4) == 0 {
            // 8 bit
            let mut sample_data = vec![];

            let mut prev_value = 0i8;
            for &point in xm.iter().take(sample_length as usize) {
                let point = point as i8;
                prev_value = prev_value.wrapping_add(point);
                sample_data.push(prev_value);
            }

            (
                SampleData::Bits8(sample_data),
                &xm[sample_length as usize..],
            )
        } else {
            // 16 bit
            let mut sample_data = vec![];

            let mut prev_value = 0i16;
            for point in xm.chunks_exact(2).take((sample_length / 2) as usize) {
                let (point, _) = read_u16(point);
                let point = point as i16;
                prev_value = prev_value.wrapping_add(point);
                sample_data.push(prev_value);
            }

            (
                SampleData::Bits16(sample_data),
                &xm[sample_length as usize..],
            )
        };

        samples.push(Sample {
            name: sample_name,
            loop_start: sample_loop_start,
            loop_length: sample_loop_length,
            volume,
            panning,
            fine_tune: fine_tune as i8,
            sample_type,
            relative_note: relative_note_number,

            sample_data,
        });

        next_xm = xm;
    }

    Ok((
        Instrument {
            name: instrument_name,
            instrument_type,
            sample_header,
            samples,
        },
        next_xm,
    ))
}

fn read_u8(xm: &[u8]) -> (u8, &[u8]) {
    let (value, xm) = xm.split_at(1);
    (value[0], xm)
}

fn read_u16(xm: &[u8]) -> (u16, &[u8]) {
    let (value, xm) = xm.split_at(2);
    let value = u16::from_le_bytes(value.try_into().unwrap());
    (value, xm)
}

fn read_u32(xm: &[u8]) -> (u32, &[u8]) {
    let (value, xm) = xm.split_at(4);
    let value = u32::from_le_bytes(value.try_into().unwrap());
    (value, xm)
}
