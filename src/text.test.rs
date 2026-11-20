use super::*;
use crate::parse;

fn data(kind: Kind, version: u32, originals: Vec<Object>, customs: Vec<Object>) -> ObjectData {
    ObjectData {
        kind,
        version,
        originals,
        customs,
    }
}

fn object(base: &[u8; 4], id: Rawcode, mods: Vec<Mod>) -> Object {
    Object {
        base: Rawcode(*base),
        id,
        sets: vec![Set { flags: 0, mods }],
    }
}

fn m(field: &[u8; 4], level: u32, column: u32, value: Value, end: Rawcode) -> Mod {
    Mod {
        field: Rawcode(*field),
        level,
        column,
        value,
        end,
    }
}

fn round_trip(d: &ObjectData) -> String {
    let text = d.to_text();
    let parsed = from_text(&text);
    assert_eq!(parsed.errors, vec![], "{text}");
    assert_eq!(parsed.data.as_ref(), Some(d), "{text}");
    text
}

#[test]
fn units() {
    let foot = Rawcode(*b"hfoo");
    let d = data(
        Kind::Unit,
        2,
        vec![object(
            b"hfoo",
            Rawcode::NONE,
            vec![m(b"uhpm", 0, 0, Value::Int(500), foot)],
        )],
        vec![object(
            b"hfoo",
            Rawcode(*b"h000"),
            vec![
                m(
                    b"unam",
                    0,
                    0,
                    Value::Str("Knight, \"the\" // one".into()),
                    Rawcode::NONE,
                ),
                m(b"usca", 0, 0, Value::Real(1.5), foot),
                m(
                    b"utip",
                    0,
                    0,
                    Value::Str("Line one\r\nLine two\n".into()),
                    Rawcode::NONE,
                ),
            ],
        )],
    );
    let text = round_trip(&d);
    assert_eq!(
        text,
        r#"// Warcraft III units (war3map.w3u). Saving writes the binary file back.
[w3u]
version=2

[hfoo]
uhpm=500

[h000]
_parent=hfoo
unam="Knight, "\u{22}"the"\u{22}" // one"
usca=1.5,end hfoo
utip="Line one"\r\n"Line two"\n
"#
    );
    // The text is BNI, so the BNI tools read it as is.
    let bni = bni_rs::parse(&text);
    assert!(bni.errors.is_empty());
    assert!(is_object_text(&bni));
}

#[test]
fn levels_columns_and_unreal() {
    let id = Rawcode(*b"A000");
    let d = data(
        Kind::Ability,
        2,
        vec![],
        vec![object(
            b"AHbz",
            id,
            vec![
                m(b"Hbz1", 3, 2, Value::Unreal(0.25), id),
                m(b"anam", 0, 0, Value::Str("Blizzard".into()), Rawcode::NONE),
                m(b"Hbz2", 1, 30, Value::Real(-0.0), Rawcode::NONE),
                m(b"Hbz3", 0, 1, Value::Real(1e-7), Rawcode::NONE),
                m(
                    b"Hbz4",
                    0,
                    0,
                    Value::Real(f32::from_bits(0x7FC0_0001)),
                    Rawcode::NONE,
                ),
                m(b"Hbz5", 0, 0, Value::Real(f32::INFINITY), Rawcode::NONE),
            ],
        )],
    );
    let text = round_trip(&d);
    assert!(text.contains("Hbz1[3].B=unreal 0.25,end A000\n"), "{text}");
    assert!(text.contains("Hbz2[1].30=-0.0\n"), "{text}");
    assert!(text.contains("Hbz3.A=1e-7\n"), "{text}");
    assert!(text.contains("Hbz4=real 0x7FC00001\n"), "{text}");
    assert!(text.contains("Hbz5=inf\n"), "{text}");
}

#[test]
fn sets_in_version_3() {
    let mut knight = object(
        b"hfoo",
        Rawcode(*b"h000"),
        vec![m(b"unam", 0, 0, Value::Str("A".into()), Rawcode::NONE)],
    );
    knight.sets.push(Set {
        flags: 2,
        mods: vec![m(b"unam", 0, 0, Value::Str("B".into()), Rawcode::NONE)],
    });
    let empty = Object {
        base: Rawcode(*b"hpea"),
        id: Rawcode::NONE,
        sets: vec![],
    };
    let empty_set = Object {
        base: Rawcode(*b"hkni"),
        id: Rawcode::NONE,
        sets: vec![Set {
            flags: 0,
            mods: vec![],
        }],
    };
    let d = data(Kind::Unit, 3, vec![empty, empty_set], vec![knight]);
    let text = round_trip(&d);
    assert!(text.contains("[hpea]\n\n[hkni]\n_flags=0\n"), "{text}");
    assert!(
        text.contains("unam=\"A\"\n_flags=2\nunam=\"B\"\n"),
        "{text}"
    );
}

#[test]
fn unusual_tables_and_ids() {
    let d = data(
        Kind::Item,
        1,
        vec![object(b"ratc", Rawcode(*b"I000"), vec![])],
        vec![object(
            b"rat6",
            Rawcode::NONE,
            vec![m(
                b"i\0\0\x01",
                0,
                0,
                Value::Int(-3),
                Rawcode(*b"\0\0\0\x01"),
            )],
        )],
    );
    let text = round_trip(&d);
    assert!(
        text.contains("[I000]\n_parent=ratc\n_table=original\n"),
        "{text}"
    );
    assert!(
        text.contains("[0x00000000]\n_parent=rat6\n0x69000001=-3,end 0x00000001\n"),
        "{text}"
    );
}

#[test]
fn friendly_input() {
    let parsed = from_text(
        "[w3t]\nversion = 1\n\n[I000]  // a sword\n_parent = ratc\niabi = \"AIat\" // abilities\n",
    );
    assert_eq!(parsed.errors, vec![]);
    let d = parsed.data.unwrap();
    assert_eq!(
        d.customs[0].sets[0].mods,
        vec![m(b"iabi", 0, 0, Value::Str("AIat".into()), Rawcode::NONE)]
    );
    assert_eq!(parse(&d.to_bytes().unwrap(), Kind::Item).unwrap(), d);
}

fn errors(text: &str) -> Vec<(usize, usize, String)> {
    let parsed = from_text(text);
    assert!(parsed.data.is_none());
    parsed
        .errors
        .into_iter()
        .map(|e| (e.line, e.col, e.message))
        .collect()
}

#[test]
fn every_error_is_reported() {
    let text = "[w3u]\nversion=2\n[h000]\n_parent=hfo\nuhpm=five\nunam=\"open\n[h001]\n_parent=hfoo\nuhpm[2]=1\n_flags=1\nabc=1\nunam=\"a\"x\nusca=1,end\n";
    assert_eq!(
        errors(text),
        vec![
            (4, 9, "expected the base object's id: four letters, digits or `_`, or 0x and eight hex digits".into()),
            (5, 6, "expected a value: a number, `unreal` and a number, or a \"string\"".into()),
            (6, 6, "string isn't closed: `\"` missing".into()),
            (9, 1, "fields of units have no level or data column".into()),
            (10, 1, "sets need format version 3; this is version 2".into()),
            (11, 1, "expected a field id: four letters, digits or `_`, or 0x and eight hex digits".into()),
            (12, 9, "between quoted parts only escapes: \\n \\r \\t or \\u{...}".into()),
            (13, 8, "expected `end` and an id: four letters, digits or `_`, or 0x and eight hex digits".into()),
        ]
    );
}

#[test]
fn header_errors() {
    assert_eq!(
        errors(""),
        vec![(1, 1, "expected the file's section first, like [w3u]".into())]
    );
    assert_eq!(errors("[w3z]\n").len(), 1);
    assert_eq!(
        errors("[w3u]\n"),
        vec![(1, 2, "missing `version=` (1, 2 or 3)".into())]
    );
    assert_eq!(
        errors("[w3u]\nversion=7\n"),
        vec![(2, 9, "unknown format version: 1, 2 or 3".into())]
    );
    assert_eq!(
        errors("x=1\n[w3u]\nversion=2\n"),
        vec![(1, 1, "expected the file's [w3?] section first".into())]
    );
    // Doodads have levels only since version 2.
    assert_eq!(errors("[w3d]\nversion=1\n[YOws]\ndvr1[1]=80\n").len(), 1);
    assert!(
        from_text("[w3d]\nversion=2\n[YOws]\ndvr1[1]=80\n")
            .data
            .is_some()
    );
}

#[test]
fn not_object_text() {
    assert!(!is_object_text(&bni_rs::parse(
        "[TriggerCategories]\nTC_UNIT=Unit\n"
    )));
    assert!(!is_object_text(&bni_rs::parse("")));
}

#[test]
fn field_names_as_comments() {
    let meta = "ID;PWXL;N;E\nC;X1;Y1;K\"ID\"\nC;X2;K\"displayName\"\nC;X1;Y2;K\"Hbz1\"\nC;X2;K\"WESTRING_AEVAL_HBZ1\"\nE\n";
    let names = Metadata::from_tables(meta, &["WESTRING_AEVAL_HBZ1=Number of Waves // all"]);
    let id = Rawcode(*b"A000");
    let d = data(
        Kind::Ability,
        2,
        vec![],
        vec![object(
            b"AHbz",
            id,
            vec![
                m(b"Hbz1", 3, 2, Value::Int(6), id),
                m(b"anam", 0, 0, Value::Str("Storm".into()), Rawcode::NONE),
            ],
        )],
    );
    let text = d.to_text_named(&names);
    assert!(
        text.contains("Hbz1[3].B=6,end A000 // Number of Waves // all\nanam=\"Storm\"\n"),
        "{text}"
    );
    let parsed = from_text(&text);
    assert_eq!(parsed.errors, vec![]);
    assert_eq!(parsed.data, Some(d));
}
