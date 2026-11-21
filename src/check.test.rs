use super::*;
use crate::from_text;

/// An SLK table of `rows`, the first being the header.
pub(crate) fn slk(rows: &[&[&str]]) -> String {
    let mut out = String::from("ID;PWXL;N;E\n");
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if !cell.is_empty() {
                out += &format!("C;X{};Y{};K\"{cell}\"\n", x + 1, y + 1);
            }
        }
    }
    out + "E\n"
}

fn abilities() -> Metadata {
    let table = slk(&[
        &[
            "ID",
            "field",
            "type",
            "minVal",
            "maxVal",
            "repeat",
            "data",
            "useSpecific",
        ],
        &["anam", "Name", "string", "", "TTName", "0", "0", ""],
        &["aher", "hero", "bool", "", "", "0", "0", ""],
        &["acdn", "Cool", "unreal", "0", "300", "4", "0", ""],
        &["Hbz1", "DataA", "int", "1", "99999", "4", "1", "AHbz,ACbz"],
        &[
            "Hbz2",
            "DataB",
            "unreal",
            "1",
            "99999",
            "4",
            "2",
            "AHbz,ACbz",
        ],
        &["atar", "targs", "targetList", "", "", "4", "0", ""],
        &["anew", "new", "someNewType", "", "", "0", "0", ""],
    ]);
    let objects = slk(&[&["alias", "code"], &["AIbz", "AHbz"]]);
    Metadata::from_tables(&table, &[]).with_codes(&objects)
}

fn found(text: &str, meta: &Metadata) -> Vec<(usize, usize, &'static str, String)> {
    let parsed = from_text(text, meta);
    assert_eq!(parsed.errors, vec![], "{text}");
    check(&parsed, meta)
        .into_iter()
        .map(|d| {
            let span = d.span.unwrap();
            (span.start_line, span.start_col, d.rule, d.message)
        })
        .collect()
}

#[test]
fn a_good_file_has_nothing() {
    // Plain numbers take their field's type: `acdn[1]=6` is an unreal.
    let text = "[w3a]\nversion=2\n[A000]\n_parent=AHbz\nanam=\"Storm\"\naher=1\nacdn[1]=6\nHbz1[2].A=3\nHbz2[2].B=unreal 25.0\natar[1]=\"air,ground\"\nanew=5\n";
    assert_eq!(found(text, &abilities()), vec![]);
    // No metadata, no checks.
    assert_eq!(
        found(
            "[w3a]\nversion=2\n[A000]\n_parent=AHbz\nzzzz=1\n",
            &Metadata::default()
        ),
        vec![]
    );
}

#[test]
fn value_types() {
    // Only a type written out can differ from the field's.
    let text = "[w3a]\nversion=2\n[AHbz]\nanam=5\nacdn[1]=int 6\nHbz1[1].A=unreal 2.0\nHbz2[1].B=real 1.5\naher=\"1\"\n";
    let meta = abilities();
    let parsed = from_text(text, &meta);
    let found = check(&parsed, &meta);
    let fixes: Vec<(usize, &str, String)> = found
        .iter()
        .map(|d| {
            (
                d.line,
                d.rule,
                d.fix
                    .as_ref()
                    .map_or(String::new(), |f| f.edits[0].text.clone()),
            )
        })
        .collect();
    assert_eq!(
        fixes,
        vec![
            (4, rule::OBJECT_VALUE_TYPE, "\"5\"".into()),
            (5, rule::OBJECT_VALUE_TYPE, "6.0".into()),
            (6, rule::OBJECT_VALUE_TYPE, "2".into()),
            (7, rule::OBJECT_VALUE_TYPE, "1.5".into()),
            (8, rule::OBJECT_VALUE_TYPE, "1".into()),
        ]
    );
    assert_eq!(
        found[1].message,
        "`acdn` (Cool) is an unreal, not a whole number: the game would misread it"
    );
    assert_eq!(found[1].level, parser_rs::Level::Error);
    // The fix covers the value.
    let span = found[1].fix.as_ref().unwrap().edits[0].span;
    assert_eq!((span.start_col, span.end_col), (9, 14));
}

#[test]
fn ranges() {
    let text = "[w3a]\nversion=2\n[AHbz]\nHbz1[1].A=0\nacdn[1]=unreal 300.5\naher=2\nacdn[2]=unreal 300.0\n";
    assert_eq!(
        found(text, &abilities()),
        vec![
            (
                4,
                11,
                rule::OBJECT_VALUE_RANGE,
                "`Hbz1` (DataA) is below World Editor's minimum, 1".into()
            ),
            (
                5,
                9,
                rule::OBJECT_VALUE_RANGE,
                "`acdn` (Cool) is above World Editor's maximum, 300".into()
            ),
            (
                6,
                6,
                rule::OBJECT_VALUE_RANGE,
                "`aher` (hero) is a flag: 0 or 1".into()
            ),
        ]
    );
}

#[test]
fn levels_and_columns() {
    let text = "[w3a]\nversion=2\n[AHbz]\nanam[1]=\"x\"\nacdn=unreal 1.0\nHbz1[1]=3\nHbz2[1].A=unreal 1.0\n";
    let meta = abilities();
    let parsed = from_text(text, &meta);
    let found: Vec<(usize, String, String)> = check(&parsed, &meta)
        .into_iter()
        .map(|d| (d.line, d.message, d.fix.unwrap().edits[0].text.clone()))
        .collect();
    assert_eq!(
        found,
        vec![
            (4, "`anam` (Name) has no levels".into(), "anam".into()),
            (
                5,
                "`acdn` (Cool) has a value per level; level 0 isn't one".into(),
                "acdn[1]".into()
            ),
            (
                6,
                "`Hbz1` (DataA) is in data column A".into(),
                "Hbz1[1].A".into()
            ),
            (
                7,
                "`Hbz2` (DataB) is in data column B".into(),
                "Hbz2[1].B".into()
            ),
        ]
    );
}

#[test]
fn owners_and_unknown_fields() {
    // AIbz's code is AHbz, so it has Blizzard's data.
    let text = "[w3a]\nversion=2\n[A000]\n_parent=AHtb\nHbz1[1].A=3\n[A001]\n_parent=AIbz\nHbz1[1].A=3\n[AHtb]\nzzzz=1\n";
    assert_eq!(
        found(text, &abilities()),
        vec![
            (
                5,
                1,
                rule::OBJECT_FIELD_OWNER,
                "`Hbz1` (DataA) is a field of AHbz, ACbz, not of AHtb".into()
            ),
            (
                10,
                1,
                rule::OBJECT_UNKNOWN_FIELD,
                "`zzzz` isn't a field of abilities in this game's metadata".into()
            ),
        ]
    );
}

#[test]
fn units_only_fields_of_items() {
    let table = slk(&[
        &[
            "ID",
            "field",
            "type",
            "useUnit",
            "useHero",
            "useBuilding",
            "useItem",
        ],
        &["uhpm", "HP", "int", "1", "1", "1", "0"],
        &["iabi", "abilList", "abilityList", "0", "0", "0", "1"],
        &["unam", "Name", "string", "1", "1", "1", "1"],
    ]);
    let meta = Metadata::from_tables(&table, &[]);
    let items = "[w3t]\nversion=2\n[I000]\n_parent=ratc\nuhpm=5\niabi=\"AIat\"\nunam=\"x\"\n";
    assert_eq!(
        found(items, &meta),
        vec![(
            5,
            1,
            rule::OBJECT_FIELD_OWNER,
            "`uhpm` (HP) is a field of units, not items".into()
        )]
    );
    // Units aren't checked: Reign of Chaos maps keep items in `.w3u`.
    assert_eq!(
        found("[w3u]\nversion=2\n[hfoo]\niabi=\"AIat\"\n", &meta),
        vec![]
    );
}

#[test]
fn version_1_doodads_have_no_levels_to_check() {
    let table = slk(&[
        &["ID", "field", "type", "repeat"],
        &["dvr1", "red", "int", "10"],
    ]);
    let meta = Metadata::from_tables(&table, &[]);
    assert_eq!(found("[w3d]\nversion=1\n[YOws]\ndvr2=5\n", &meta), vec![]);
    assert_eq!(
        found("[w3d]\nversion=2\n[YOws]\ndvr1[2]=5\n", &meta),
        vec![]
    );
    assert_eq!(found("[w3d]\nversion=2\n[YOws]\ndvr1=5\n", &meta).len(), 1);
}
