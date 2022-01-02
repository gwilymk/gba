// Based on information from https://github.com/milkytracker/MilkyTracker/blob/master/resources/reference/xm-form.txt

#[non_exhaustive]
#[derive(Debug)]
pub struct Header<'a> {
    pub module_name: &'a [u8],
    pub tracker_name: &'a [u8],

    pub version_number: u16,
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

    let (version_number, xm) = xm.split_at(2);
    let version_number = u16::from_le_bytes(version_number.try_into().unwrap());

    if version_number != 0x0104 {
        return Err(XmError::UnsupportedVersion(version_number));
    }

    Ok((
        Header {
            module_name,
            tracker_name,

            version_number,
        },
        xm,
    ))
}
