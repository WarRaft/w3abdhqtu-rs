use super::*;
use parser_rs::Writer;

fn code(s: &[u8; 4]) -> Rawcode {
    Rawcode(*s)
}

fn field(id: &[u8; 4], level: u32, column: u32, value: Value, end: Rawcode) -> Mod {
    Mod {
        field: code(id),
        level,
        column,
        value,
        end,
    }
}

/// A unit file by hand: one changed footman, one custom unit.
fn unit_bytes(version: u32) -> Vec<u8> {
    let mut w = Writer::new();
    w.u32(version);
    w.u32(1);
    w.bytes(b"hfoo\0\0\0\0");
    if version >= 3 {
        w.u32(1);
        w.u32(0);
    }
    w.u32(1);
    w.bytes(b"uhpm");
    w.u32(0);
    w.i32(500);
    w.bytes(b"hfoo");

    w.u32(1);
    w.bytes(b"hfooh000");
    if version >= 3 {
        w.u32(2);
        w.u32(0);
    }
    w.u32(2);
    w.bytes(b"unam");
    w.u32(3);
    w.bytes(b"Knight\0");
    w.bytes(b"h000");
    w.bytes(b"usca");
    w.u32(1);
    w.f32(1.5);
    w.bytes(b"\0\0\0\0");
    if version >= 3 {
        w.u32(2);
        w.u32(1);
        w.bytes(b"uhpm");
        w.u32(0);
        w.i32(900);
        w.bytes(b"h000");
    }
    w.into_bytes()
}

#[test]
fn reads_units_v2() {
    let bytes = unit_bytes(2);
    let data = parse(&bytes, Kind::Unit).unwrap();
    assert_eq!(data.version, 2);
    assert_eq!(data.originals.len(), 1);
    let foot = &data.originals[0];
    assert_eq!((foot.base, foot.id), (code(b"hfoo"), Rawcode::NONE));
    assert_eq!(
        foot.sets,
        vec![Set {
            flags: 0,
            mods: vec![field(b"uhpm", 0, 0, Value::Int(500), code(b"hfoo"))]
        }]
    );

    let knight = &data.customs[0];
    assert_eq!((knight.base, knight.id), (code(b"hfoo"), code(b"h000")));
    let mods: Vec<_> = knight.mods().cloned().collect();
    assert_eq!(
        mods,
        vec![
            field(b"unam", 0, 0, Value::Str("Knight".into()), code(b"h000")),
            field(b"usca", 0, 0, Value::Real(1.5), Rawcode::NONE),
        ]
    );
    assert_eq!(data.to_bytes().unwrap(), bytes);
}

#[test]
fn reads_sets_v3() {
    let bytes = unit_bytes(3);
    let data = parse(&bytes, Kind::Unit).unwrap();
    let knight = &data.customs[0];
    assert_eq!(knight.sets.len(), 2);
    assert_eq!(knight.sets[1].flags, 2);
    assert_eq!(
        knight.sets[1].mods,
        vec![field(b"uhpm", 0, 0, Value::Int(900), code(b"h000"))]
    );
    assert_eq!(knight.mods().count(), 3);
    assert_eq!(data.to_bytes().unwrap(), bytes);
}

#[test]
fn reads_levels() {
    let mut w = Writer::new();
    w.u32(2);
    w.u32(0);
    w.u32(1);
    w.bytes(b"AHbzA000");
    w.u32(1);
    w.bytes(b"Hbz1");
    w.u32(2);
    w.u32(3);
    w.u32(1);
    w.f32(0.25);
    w.bytes(b"A000");
    let bytes = w.into_bytes();

    let data = parse(&bytes, Kind::Ability).unwrap();
    assert_eq!(
        data.customs[0].sets[0].mods,
        vec![field(b"Hbz1", 3, 1, Value::Unreal(0.25), code(b"A000"))]
    );
    assert_eq!(data.to_bytes().unwrap(), bytes);

    // The same bytes as units: no levels, so the level is read as a value
    // and the rest no longer lines up.
    assert!(parse(&bytes, Kind::Unit).is_err());
}

#[test]
fn doodads_have_levels_since_v2() {
    assert!(!Kind::Doodad.has_levels(1));
    assert!(Kind::Doodad.has_levels(2));
    assert!(Kind::Ability.has_levels(1));
    assert!(!Kind::Unit.has_levels(3));

    let mut w = Writer::new();
    w.u32(1);
    w.u32(1);
    w.bytes(b"YOws\0\0\0\0");
    w.u32(1);
    w.bytes(b"dvr1");
    w.u32(0);
    w.i32(80);
    w.bytes(b"YOws");
    w.u32(0);
    let bytes = w.into_bytes();
    let data = parse(&bytes, Kind::Doodad).unwrap();
    assert_eq!(
        data.originals[0].sets[0].mods,
        vec![field(b"dvr1", 0, 0, Value::Int(80), code(b"YOws"))]
    );
    assert_eq!(data.to_bytes().unwrap(), bytes);
}

#[test]
fn errors_have_offsets() {
    let err = parse(&[9, 0, 0, 0], Kind::Unit).unwrap_err();
    assert_eq!(err.to_string(), "unknown w3u version 9 (at byte 0x0)");

    let mut bytes = unit_bytes(2);
    // The type tag of `uhpm`.
    bytes[24] = 7;
    let err = parse(&bytes, Kind::Unit).unwrap_err();
    assert_eq!(
        err.to_string(),
        "field uhpm has unknown value type 7 (at byte 0x18)"
    );

    let mut bytes = unit_bytes(2);
    bytes.push(0);
    assert_eq!(
        parse(&bytes, Kind::Unit).unwrap_err().message,
        "1 extra byte after the end of the data"
    );

    let bytes = unit_bytes(2);
    let err = parse(&bytes[..bytes.len() - 2], Kind::Unit).unwrap_err();
    assert_eq!(err.offset, bytes.len() - 4);
}

#[test]
fn write_checks_the_version() {
    let mut data = parse(&unit_bytes(3), Kind::Unit).unwrap();
    data.version = 2;
    let err = data.to_bytes().unwrap_err();
    assert_eq!(err.message, "h000: version 2 has one set without flags");

    data.customs.clear();
    assert_eq!(parse(&data.to_bytes().unwrap(), Kind::Unit).unwrap(), data);

    data.version = 4;
    assert!(data.to_bytes().is_err());
}

#[test]
fn write_empty() {
    let data = ObjectData::new(Kind::Item);
    let bytes = data.to_bytes().unwrap();
    assert_eq!(bytes, [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(parse(&bytes, Kind::Item).unwrap(), data);
}

#[test]
fn kinds_by_extension() {
    assert_eq!(Kind::from_path("maps/war3map.W3A"), Some(Kind::Ability));
    assert_eq!(Kind::from_extension(".w3h"), Some(Kind::Buff));
    assert_eq!(Kind::from_extension("w3x"), None);
    assert_eq!(Kind::from_path("w3u"), None);
    for kind in Kind::ALL {
        assert_eq!(Kind::from_extension(kind.extension()), Some(kind));
    }
}

/// A file of the game's files extracted to `dir`, looked up in any case.
fn game_file(dir: &str, name: &str) -> Option<String> {
    let mut path = std::path::PathBuf::from(dir);
    for part in name.split('\\') {
        let entry = std::fs::read_dir(&path).ok()?.flatten().find(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(part))
        })?;
        path = entry.path();
    }
    Some(String::from_utf8_lossy(&std::fs::read(path).ok()?).into_owned())
}

/// Every object-data file in `$W3ABDHQTU_CORPUS` reads and writes back
/// byte for byte, and goes through the text form unchanged:
/// `W3ABDHQTU_CORPUS=dir cargo test corpus -- --ignored`. With
/// `$W3ABDHQTU_GAME`, the game's files extracted to a folder, every field
/// gets a name too, the text with the names goes through as well, and
/// `check` finds no wrong types, unknown fields or fields of other objects.
#[test]
#[ignore]
fn corpus() {
    let dir =
        std::env::var("W3ABDHQTU_CORPUS").expect("set W3ABDHQTU_CORPUS to a folder of .w3? files");
    let game = std::env::var("W3ABDHQTU_GAME").ok();
    let names = |kind: Kind| {
        let Some(game) = &game else {
            return Metadata::default();
        };
        let metadata = game_file(game, kind.metadata_file()).expect("metadata table");
        let strings: Vec<String> = STRINGS_FILES
            .iter()
            .filter_map(|f| game_file(game, f))
            .collect();
        let strings: Vec<&str> = strings.iter().map(String::as_str).collect();
        let meta = Metadata::from_tables(&metadata, &strings);
        match kind.objects_file().and_then(|f| game_file(game, f)) {
            Some(objects) => meta.with_codes(&objects),
            None => meta,
        }
    };
    let names: Vec<(Kind, Metadata)> = Kind::ALL.into_iter().map(|k| (k, names(k))).collect();
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let Some(kind) = Kind::from_path(&path.to_string_lossy()) else {
            continue;
        };
        let bytes = std::fs::read(&path).unwrap();
        let data = parse(&bytes, kind).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        assert!(
            data.to_bytes().unwrap() == bytes,
            "{}: differs after writing",
            path.display()
        );
        let names = &names.iter().find(|(k, _)| *k == kind).unwrap().1;
        let text = data.to_text_named(names);
        let back = from_text(&text, names);
        assert_eq!(back.errors, vec![], "{}", path.display());
        // The same data, but the `end` ids, which the text leaves out.
        let mut plain = data.clone();
        plain.clear_ends();
        assert!(
            back.data.as_ref() == Some(&plain),
            "{}: differs after the text form",
            path.display()
        );
        if game.is_some() {
            // Real maps store every value as its type, and every field is
            // the game's and its object's. (Values past World Editor's
            // limits and level-0 leftovers of old versions do happen.)
            for d in check(&back, names) {
                assert!(
                    d.rule == parser_rs::rule::OBJECT_VALUE_RANGE
                        || d.rule == parser_rs::rule::OBJECT_FIELD_LEVEL,
                    "{}:{}: {}",
                    path.display(),
                    d.line,
                    d.message
                );
            }
            for m in data.objects().flat_map(|o| o.mods()) {
                assert!(
                    names.get(m.field).is_some(),
                    "{}: field {} has no name",
                    path.display(),
                    m.field
                );
            }
        }
        checked += 1;
    }
    assert!(checked > 0, "no object-data files in {dir}");
    eprintln!("{checked} files");
}
