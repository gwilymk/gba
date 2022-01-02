use agb_xm::parse;

const ALGAR_NINJA_ON_SPEED: &[u8] = include_bytes!("mod_files/algar_-_ninja_on_speed.xm");

#[test]
fn can_parse_algar_ninja_on_speed_header() {
    let parsed = parse(ALGAR_NINJA_ON_SPEED).unwrap();

    assert_eq!(parsed.header.module_name, b"Ninja on speed");
    assert_eq!(parsed.header.tracker_name, b"MilkyTracker        ");

    assert_eq!(parsed.header.song_length, 0x30);
    assert_eq!(parsed.header.song_restart_pos, 0);
    assert_eq!(parsed.header.num_channels, 4);
    assert_eq!(parsed.header.num_patterns, 0x2f);
    assert_eq!(parsed.header.num_instruments, 15);
    assert_eq!(parsed.header.flags, 1);
    assert_eq!(parsed.header.default_tempo, 0x4);
}
