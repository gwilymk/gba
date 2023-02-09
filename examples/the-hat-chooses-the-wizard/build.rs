const LEVELS: &[&str] = &[
    "1-1.tmx", "1-2.tmx", "1-3.tmx", "1-4.tmx", "1-5.tmx", "1-6.tmx", "1-7.tmx", "1-8.tmx",
    "2-4.tmx", "2-2.tmx", "2-1.tmx", "2-3.tmx",
];

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR environment variable must be specified");
    println!("cargo:rerun-if-changed=build.rs");

    tiled_export::export_tilemap(&out_dir).expect("Failed to export tilemap");
    for &level in LEVELS {
        tiled_export::export_level(&out_dir, level).expect("Failed to export level");
    }
}

mod tiled_export {
    use quote::ToTokens;
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufReader, BufWriter, Write};

    const COLLISION_TILE: i32 = 1;
    const KILL_TILE: i32 = 2;
    const WIN_TILE: i32 = 4;

    pub fn export_tilemap(out_dir: &str) -> std::io::Result<()> {
        let filename = "map/tilemap.json";
        println!("cargo:rerun-if-changed={filename}");
        let file = File::open(filename)?;
        let reader = BufReader::new(file);

        let tilemap: TiledTilemap = serde_json::from_reader(reader)?;

        let output_file = File::create(format!("{out_dir}/tilemap.rs"))?;
        let mut writer = BufWriter::new(output_file);

        let tile_data: HashMap<_, _> = tilemap
            .tiles
            .iter()
            .map(|tile| {
                (
                    tile.id,
                    match tile.tile_type.as_str() {
                        "Collision" => COLLISION_TILE,
                        "Kill" => KILL_TILE,
                        "Win" => WIN_TILE,
                        _ => 0,
                    },
                )
            })
            .collect();

        let tile_info = (0..tilemap.tilecount)
            .map(|id| *tile_data.get(&id).unwrap_or(&0))
            .map(|tile_type| tile_type.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        writeln!(
            &mut writer,
            "pub const COLLISION_TILE: i32 = {COLLISION_TILE};",
        )?;

        writeln!(&mut writer, "pub const KILL_TILE: i32 = {KILL_TILE};")?;
        writeln!(&mut writer, "pub const WIN_TILE: i32 = {WIN_TILE};")?;

        writeln!(&mut writer, "pub const TILE_DATA: &[u32] = &[{tile_info}];")?;

        Ok(())
    }

    pub fn export_level(out_dir: &str, level_file: &str) -> std::io::Result<()> {
        let filename = format!("map/{level_file}");
        println!("cargo:rerun-if-changed={filename}");
        let output_file = File::create(format!("{out_dir}/{level_file}.rs"))?;
        let mut writer = BufWriter::new(output_file);

        let level = load_level(&filename);

        let width = &level.width;
        let height = &level.height;
        let foreground = &level.foreground;
        let background = &level.background;
        let enemies = level
            .enemies
            .iter()
            .map(|&((x, y), enemy_type)| wrap_tuple(&(wrap_tuple(&(x, y)), enemy_type)));
        let stops = level.stops.iter().map(wrap_tuple);
        let start = wrap_tuple(&level.start);

        let encoded = quote::quote!(
            #[allow(unused_imports)]
            use crate::enemies::EnemyType::{self, *};

            const WIDTH: u32 = #width;
            const HEIGHT: u32 = #height;
            const TILEMAP: &[u16] = &[#(#foreground),*];
            const BACKGROUND: &[u16] = &[#(#background),*];

            const ENEMIES: &[((i32, i32), EnemyType)] = &[#(#enemies),*];
            const ENEMY_STOPS: &[(i32, i32)] = &[#(#stops),*];
            const START_POS: (i32, i32) = #start;

            use crate::Level;
            use agb::fixnum::Vector2D;

            pub const fn get_level() -> Level {
                Level {
                    background: TILEMAP,
                    foreground: BACKGROUND,
                    dimensions: Vector2D {x: WIDTH, y: HEIGHT},
                    collision: crate::map_tiles::tilemap::TILE_DATA,

                    enemy_stops: ENEMY_STOPS,
                    enemies: ENEMIES,
                    start_pos: START_POS,
                }
            }
        );

        let _ = write!(writer, "{encoded}");

        Ok(())
    }

    fn get_map_id(id: i32) -> i32 {
        match id {
            0 => 10,
            i => i - 1,
        }
    }

    #[derive(Clone, Copy)]
    struct WrappedTuple<T, V>(T, V);

    impl<T, V> ToTokens for WrappedTuple<T, V>
    where
        T: ToTokens,
        V: ToTokens,
    {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            let t1 = &self.0;
            let t2 = &self.1;
            quote::quote!((#t1, #t2)).to_tokens(tokens);
        }
    }

    fn wrap_tuple<T: Copy, V: Copy>(a: &(T, V)) -> WrappedTuple<T, V> {
        WrappedTuple(a.0, a.1)
    }

    fn load_level(level: &str) -> Level {
        let mut loader = tiled::Loader::new();
        let map = loader
            .load_tmx_map(level)
            .expect("should be able to load tiled level");

        println!("Loading {level}");

        let height = map.height;
        let width = map.width;

        let foreground = map.get_layer(0).expect("layer 1 (foreground) should exist");
        let background = map.get_layer(1).expect("layer 2 (background) should exist");
        let points = map.get_layer(2).expect("layer 0 (points) should exist");

        let foreground = extract_tiles(foreground);
        let background = extract_tiles(background);

        let points = extract_points(points);

        let mut enemies = vec![];
        let mut enemy_stops = Vec::new();

        let mut start_pos = (0, 0);

        for point in points {
            match point.0.as_str() {
                "Player Start" => start_pos = point.1,
                "Slime Spawn" => enemies.push((point.1, EnemyType::Slime)),
                "Snail Spawn" => enemies.push((point.1, EnemyType::Snail)),
                "Bat Spawn" => enemies.push((point.1, EnemyType::Bat)),
                "Enemy Stop" => enemy_stops.push(point.1),
                _ => panic!("unknown object {}", point.0),
            }
        }

        Level {
            width,
            height,
            enemies,
            background,
            foreground,
            stops: enemy_stops,
            start: start_pos,
        }
    }

    fn extract_points(layer: tiled::Layer) -> impl Iterator<Item = (String, (i32, i32))> + '_ {
        let layer = match layer.layer_type() {
            tiled::LayerType::ObjectLayer(layer) => layer,
            _ => panic!("expected a tile layer but got something other than a tile layer"),
        };

        layer
            .objects()
            .map(|o| (o.obj_type.clone(), (o.x as i32, o.y as i32)))
    }

    fn extract_tiles(layer: tiled::Layer) -> Vec<u16> {
        let layer = match layer.layer_type() {
            tiled::LayerType::TileLayer(layer) => layer,
            _ => panic!("expected a tile layer but got something other than a tile layer"),
        };
        let width = layer.width().unwrap();
        let height = layer.height().unwrap();

        let mut tiles = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                let tile = layer
                    .get_tile(x as i32, y as i32)
                    .map(|x| get_map_id((x.id() + 1) as i32))
                    .unwrap_or(10);
                tiles.push(tile as u16);
            }
        }
        tiles
    }

    #[derive(Debug, Clone, Copy)]
    enum EnemyType {
        Bat,
        Snail,
        Slime,
    }

    impl ToTokens for EnemyType {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            quote::format_ident!("{self:?}").to_tokens(tokens);
        }
    }

    struct Level {
        width: u32,
        height: u32,
        background: Vec<u16>,
        foreground: Vec<u16>,
        enemies: Vec<((i32, i32), EnemyType)>,
        stops: Vec<(i32, i32)>,
        start: (i32, i32),
    }

    #[derive(Deserialize)]
    struct TiledTilemap {
        tiles: Vec<TiledTile>,
        tilecount: i32,
    }

    #[derive(Deserialize)]
    struct TiledTile {
        id: i32,
        #[serde(rename = "type")]
        tile_type: String,
    }
}
