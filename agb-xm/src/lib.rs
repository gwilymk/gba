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
#[derive(Debug)]
pub struct Song<'a> {
    pub header: Header<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmError {
    HeaderTooShort,
    InvalidHeader,
    UnsupportedVersion(u16),
}

pub fn parse(xm: &'_ [u8]) -> Result<Song<'_>, XmError> {
    let (header, xm) = parse_header(xm)?;

    Ok(Song { header })
}

fn parse_header(xm: &'_ [u8]) -> Result<(Header<'_>, &'_ [u8]), XmError> {
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

    let xm_after_header = &xm[header_size..];

    let (song_length, xm) = read_u16(xm);
    let (song_restart_pos, xm) = read_u16(xm);
    let (num_channels, xm) = read_u16(xm);
    let (num_patterns, xm) = read_u16(xm);
    let (num_instruments, xm) = read_u16(xm);
    let (flags, xm) = read_u16(xm);
    let (default_tempo, xm) = read_u16(xm);
    let (default_bpm, xm) = read_u16(xm);

    const PATTERN_ORDER_TABLE_LENGTH: usize = 256;
    let (pattern_order_table, xm) = xm.split_at(PATTERN_ORDER_TABLE_LENGTH);

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

fn read_u16(xm: &'_ [u8]) -> (u16, &'_ [u8]) {
    let (value, xm) = xm.split_at(2);
    let value = u16::from_le_bytes(value.try_into().unwrap());
    return (value, xm);
}
