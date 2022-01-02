use agb_xm::parse;

const ALGAR_NINJA_ON_SPEED: &[u8] = include_bytes!("mod_files/algar_-_ninja_on_speed.xm");

#[test]
fn can_parse_algar_ninja_on_speed() {
    let parsed = parse(ALGAR_NINJA_ON_SPEED).unwrap();

    assert_eq!(parsed.module_name, b"Ninja on speed");
    assert_eq!(parsed.tracker_name, b"MilkyTracker        ");
}
