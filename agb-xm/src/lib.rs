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
    if xm.len() < 60 {
        return Err(XmError::HeaderTooShort);
    }

    const ID_TEXT_LENGTH: usize = 17;
    let id_text = &xm[..ID_TEXT_LENGTH];
    if id_text != b"Extended Module: " {
        return Err(XmError::InvalidHeader);
    }

    let xm = &xm[ID_TEXT_LENGTH..];

    const MODULE_NAME_LENGTH: usize = 20;
    let module_name = &xm[..MODULE_NAME_LENGTH];
    let module_name = &module_name[..module_name
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(MODULE_NAME_LENGTH)];

    let xm = &xm[MODULE_NAME_LENGTH..];

    if xm[0] != 0x1a {
        return Err(XmError::InvalidHeader);
    }

    let xm = &xm[1..];

    const TRACKER_NAME_LENGTH: usize = 20;
    let tracker_name = &xm[..TRACKER_NAME_LENGTH];
    let tracker_name = &tracker_name[..tracker_name
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(TRACKER_NAME_LENGTH)];

    let xm = &xm[TRACKER_NAME_LENGTH..];

    let version_number = (xm[1] as u16) << 8 | (xm[0] as u16);

    if version_number != 0x0104 {
        return Err(XmError::UnsupportedVersion(version_number));
    }

    Ok(Song {
        header: Header {
            module_name,
            tracker_name,

            version_number,
        },
    })
}
