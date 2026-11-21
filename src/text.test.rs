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

/// The text of `d`, checked to read back as `d` with its `end` ids 0.
fn round_trip(d: &ObjectData) -> String {
    round_trip_with(d, &Metadata::default())
}

fn round_trip_with(d: &ObjectData, meta: &Metadata) -> String {
    let text = d.to_text_named(meta);
    let parsed = from_text(&text, meta);
    assert_eq!(parsed.errors, vec![], "{text}");
    let mut want = d.clone();
    want.clear_ends();
    assert_eq!(parsed.data.as_ref(), Some(&want), "{text}");
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
usca=1.5
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
    // Without metadata an unreal says so; `end` is never written.
    assert!(text.contains("Hbz1[3].B=unreal 0.25\n"), "{text}");
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
        text.contains("[0x00000000]\n_parent=rat6\n0x69000001=-3\n"),
        "{text}"
    );
}

#[test]
fn friendly_input() {
    let parsed = from_text(
        &String::from(
            "[w3t]\nversion = 1\n\n[I000]  // a sword\n_parent = ratc\niabi = \"AIat\" // abilities\n",
        ),
        &Metadata::default(),
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
    let parsed = from_text(text, &Metadata::default());
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
            (5, 6, "expected a value: a number, or a \"string\" (`int`, `real` or `unreal` before a number gives its type)".into()),
            (6, 6, "string isn't closed: `\"` missing".into()),
            (9, 1, "fields of units have no level or data column".into()),
            (10, 1, "sets need format version 3; this is version 2".into()),
            (11, 1, "expected a field: its name or id (four letters, digits or `_`, or 0x and eight hex digits)".into()),
            (12, 9, "between quoted parts only escapes: \\n \\r \\t or \\u{...}".into()),
            (13, 8, "expected only the value; a string with a comma is quoted".into()),
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
        from_text(
            "[w3d]\nversion=2\n[YOws]\ndvr1[1]=80\n",
            &Metadata::default()
        )
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
    let text = round_trip_with(&d, &names);
    assert!(
        text.contains("Hbz1[3].B=6 // Number of Waves // all\nanam=\"Storm\"\n"),
        "{text}"
    );
}

#[test]
fn fields_under_the_cursor() {
    let doc =
        bni_rs::parse("[w3a]\nversion=2\n[A000]\nHbz1[2].A=oops\n_parent=AHbz\n[ACwe]\n  arlv=6\n");
    let at = |line, col| {
        field_at(&doc, line, col, &Metadata::default()).map(|f| (f.field, f.object, f.base))
    };
    let (hbz1, a000, ahbz) = (Rawcode(*b"Hbz1"), Rawcode(*b"A000"), Rawcode(*b"AHbz"));
    // Anywhere on the key, to its end; a broken value doesn't matter, nor
    // where `_parent` is.
    assert_eq!(at(4, 1), Some((hbz1, a000, ahbz)));
    assert_eq!(at(4, 10), Some((hbz1, a000, ahbz)));
    assert_eq!(at(4, 11), None);
    let acwe = Rawcode(*b"ACwe");
    assert_eq!(at(7, 3), Some((Rawcode(*b"arlv"), acwe, acwe)));
    // The key's span takes its indent in.
    assert_eq!(at(7, 1), Some((Rawcode(*b"arlv"), acwe, acwe)));
    // Not on a key, or not object text.
    assert_eq!(at(2, 1), None);
    assert_eq!(
        field_at(
            &bni_rs::parse("[a]\n[b]\nx=1\n"),
            3,
            1,
            &Metadata::default()
        ),
        None
    );
    assert_eq!(
        field_at(&doc, 4, 1, &Metadata::default()).unwrap().kind,
        Kind::Ability
    );
}

#[test]
fn numbers_take_their_fields_type() {
    let table = "C;X1;Y1;K\"ID\"\nC;X2;K\"type\"\nC;X3;K\"field\"\n\
C;X1;Y2;K\"Hbz1\"\nC;X2;K\"int\"\nC;X3;K\"Waves\"\n\
C;X1;Y3;K\"Hbz2\"\nC;X2;K\"unreal\"\nC;X3;K\"Damage\"\n\
C;X1;Y4;K\"Hbz3\"\nC;X2;K\"real\"\nC;X3;K\"Area\"\n";
    let meta = Metadata::from_tables(table, &[]);
    let id = Rawcode(*b"A000");
    let d = data(
        Kind::Ability,
        2,
        vec![],
        vec![object(
            b"AHbz",
            id,
            vec![
                m(b"Hbz1", 1, 1, Value::Int(3), id),
                m(b"Hbz2", 1, 2, Value::Unreal(25.0), id),
                m(b"Hbz3", 1, 3, Value::Real(2.5), Rawcode::NONE),
                // Types other than the field's say so, to read back the same.
                m(b"Hbz1", 2, 1, Value::Real(3.0), Rawcode::NONE),
                m(b"Hbz2", 2, 2, Value::Int(25), Rawcode::NONE),
                m(b"Hbz3", 2, 3, Value::Unreal(2.5), Rawcode::NONE),
            ],
        )],
    );
    let text = round_trip_with(&d, &meta);
    for line in [
        "Hbz1[1].A=3 // Waves: int",
        "Hbz2[1].B=25.0 // Damage: unreal",
        "Hbz3[1].C=2.5 // Area: real",
        "Hbz1[2].A=real 3.0 // Waves: int",
        "Hbz2[2].B=int 25 // Damage: unreal",
        "Hbz3[2].C=unreal 2.5 // Area: real",
    ] {
        assert!(text.contains(&format!("{line}\n")), "{line}\n{text}");
    }
    // A plain whole number in a real field is a real; without metadata,
    // it is an integer.
    let plain = "[w3a]\nversion=2\n[A000]\n_parent=AHbz\nHbz2[1].B=25\n";
    let value = |meta: &Metadata| {
        from_text(plain, meta).data.unwrap().customs[0].sets[0].mods[0]
            .value
            .clone()
    };
    assert_eq!(value(&meta), Value::Unreal(25.0));
    assert_eq!(value(&Metadata::default()), Value::Int(25));
}

/// Abilities' metadata with names from the tables' `field` column.
fn named_abilities() -> Metadata {
    let rows: &[&[&str]] = &[
        &["ID", "field", "type", "repeat", "data", "useSpecific"],
        &["acdn", "Cool", "unreal", "4", "0", ""],
        &["anam", "Name", "string", "0", "0", ""],
        &["abpx", "Buttonpos", "int", "0", "0", ""],
        &["abpy", "Buttonpos", "int", "0", "0", ""],
        &["Hbz1", "Data", "int", "4", "1", "AHbz"],
        &["Hbz2", "Data", "unreal", "4", "2", "AHbz"],
        &["Oww1", "Data", "unreal", "4", "1", "AOww"],
    ];
    let mut slk = String::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if !cell.is_empty() {
                slk += &format!("C;X{};Y{};K\"{cell}\"\n", x + 1, y + 1);
            }
        }
    }
    let codes = "C;X1;Y1;K\"alias\"\nC;X2;K\"code\"\nC;X1;Y2;K\"AIbz\"\nC;X2;K\"AHbz\"\n";
    Metadata::from_tables(&slk, &[]).with_codes(codes)
}

#[test]
fn keys_are_the_tables_names() {
    let meta = named_abilities();
    let id = Rawcode(*b"A000");
    let d = data(
        Kind::Ability,
        2,
        vec![],
        vec![object(
            b"AIbz",
            id,
            vec![
                m(b"acdn", 1, 0, Value::Unreal(12.0), Rawcode::NONE),
                m(b"Hbz1", 3, 1, Value::Int(5), Rawcode::NONE),
                m(b"Hbz2", 3, 2, Value::Unreal(0.5), Rawcode::NONE),
                // Two fields share `Buttonpos`: ids.
                m(b"abpx", 0, 0, Value::Int(1), Rawcode::NONE),
                // Not in its own column: its id and column.
                m(b"Hbz1", 2, 3, Value::Int(7), Rawcode::NONE),
                // Another ability's data: its id.
                m(b"Oww1", 1, 1, Value::Unreal(1.0), Rawcode::NONE),
            ],
        )],
    );
    let text = round_trip_with(&d, &meta);
    // AIbz has AHbz's data, by its code: `DataA` is Hbz1 there.
    for line in [
        "Cool[1]=12.0",
        "DataA[3]=5",
        "DataB[3]=0.5",
        "abpx=1",
        "Hbz1[2].C=7",
        "Oww1[1].A=1.0",
    ] {
        assert!(
            text.lines()
                .any(|l| l.starts_with(&format!("{line} //")) || l == line),
            "{line}\n{text}"
        );
    }
    // Ids still read.
    let by_id = "[w3a]\nversion=2\n[A000]\n_parent=AHbz\nHbz1[3].A=5\nDataA[2]=6\nCool=1.0\n";
    let read = from_text(by_id, &meta);
    let mods = &read.data.unwrap().customs[0].sets[0].mods;
    assert_eq!((mods[0].field, mods[0].column), (Rawcode(*b"Hbz1"), 1));
    assert_eq!(
        (mods[1].field, mods[1].level, mods[1].column),
        (Rawcode(*b"Hbz1"), 2, 1)
    );
    assert_eq!(
        (mods[2].field, mods[2].value.clone()),
        (Rawcode(*b"acdn"), Value::Unreal(1.0))
    );
    // Hovers read names too.
    let doc = bni_rs::parse(by_id);
    assert_eq!(
        field_at(&doc, 6, 1, &meta).map(|f| f.field),
        Some(Rawcode(*b"Hbz1"))
    );
}

#[test]
fn hints_instead_of_comments() {
    let meta = named_abilities();
    let id = Rawcode(*b"A000");
    let d = data(
        Kind::Ability,
        2,
        vec![],
        vec![object(
            b"AHbz",
            id,
            vec![
                m(b"anam", 0, 0, Value::Str("Storm".into()), Rawcode::NONE),
                m(b"acdn", 1, 0, Value::Unreal(12.0), Rawcode::NONE),
                m(b"Hbz1", 3, 1, Value::Int(5), Rawcode::NONE),
            ],
        )],
    );
    // Names and plain numbers, but no comments: the editor shows those.
    let text = d.to_text_with(&meta, false);
    assert!(
        text.ends_with("[A000]\n_parent=AHbz\nName=\"Storm\"\nCool[1]=12.0\nDataA[3]=5\n"),
        "{text}"
    );
    let doc = bni_rs::parse(&text);
    let hints = |first, last| -> Vec<(usize, usize, String)> {
        field_hints(&doc, &meta, first, last)
            .into_iter()
            .map(|h| (h.line, h.col, h.label))
            .collect()
    };
    // Only the lines asked for; after the value.
    assert_eq!(
        hints(8, 9),
        vec![(8, 13, "Cool: unreal".into()), (9, 11, "Data: int".into())]
    );
    assert_eq!(hints(7, 7), vec![(7, 13, "Name: string".into())]);
    assert_eq!(hints(1, 5), vec![]);
    assert!(field_hints(&doc, &Metadata::default(), 1, 100).is_empty());
}
