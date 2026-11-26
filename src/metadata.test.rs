use super::*;

const META: &str = "ID;PWXL;N;E\nC;X1;Y1;K\"ID\"\nC;X2;K\"field\"\nC;X3;K\"displayName\"\n\
C;X1;Y2;K\"uhpm\"\nC;X2;K\"HP\"\nC;X3;K\"WESTRING_UEVAL_UHPM\"\n\
C;X1;Y3;K\"unam\"\nC;X2;K\"Name\"\nC;X3;K\"WESTRING_MISSING\"\n\
C;X1;Y4;K\"uabi\"\nC;X3;K\"WESTRING_UEVAL_UABI\"\n\
C;X1;Y5;K\"long\"\nC;X2;K\"x\"\n\
C;X1;Y6;K\"uhpm\"\nC;X3;K\"WESTRING_UEVAL_UABI\"\nE\n";

#[test]
fn names_from_tables_and_strings() {
    let strings = "[WorldEditStrings]\r\nWESTRING_UEVAL_UHPM=Hit Points Maximum (Base)\r\nWESTRING_UEVAL_UABI = \"Abilities\"\r\n";
    let over = "WESTRING_UEVAL_UABI=Abilities - Normal\n";
    let names = Metadata::from_tables(META, &[strings, over]);
    assert_eq!(
        names.name(Rawcode(*b"uhpm")),
        Some("Hit Points Maximum (Base)")
    );
    // Not among the strings: the `field` column.
    assert_eq!(names.name(Rawcode(*b"unam")), Some("Name"));
    // A later strings file wins.
    assert_eq!(names.name(Rawcode(*b"uabi")), Some("Abilities - Normal"));
    assert_eq!(names.name(Rawcode(*b"long")), Some("x"));
    assert_eq!(names.name(Rawcode(*b"umvs")), None);
    assert_eq!(names.len(), 4);
}

#[test]
fn no_table() {
    assert!(Metadata::from_tables("", &[]).is_empty());
    assert!(Metadata::from_tables("C;X1;Y1;K\"field\"\n", &[]).is_empty());
}

#[test]
fn metadata_files() {
    assert_eq!(Kind::Item.metadata_file(), Kind::Unit.metadata_file());
    assert_eq!(Kind::Buff.metadata_file(), "Units\\AbilityBuffMetaData.slk");
}

#[test]
fn repeated_fields() {
    let meta = "C;X1;Y1;K\"ID\"\nC;X2;K\"repeat\"\nC;X3;K\"field\"\n\
C;X1;Y2;K\"dvr1\"\nC;X2;K10\nC;X3;K\"Red\"\n\
C;X1;Y3;K\"Hbz1\"\nC;X2;K4\nC;X3;K\"Waves\"\n\
C;X1;Y4;K\"Hbz2\"\nC;X2;K4\nC;X3;K\"Damage\"\n";
    let names = Metadata::from_tables(meta, &[]);
    assert_eq!(names.name(Rawcode(*b"dvr1")), Some("Red"));
    assert_eq!(names.name(Rawcode(*b"dvr2")), Some("Red, variation 2"));
    assert_eq!(names.name(Rawcode(*b"dvr9")), Some("Red, variation 9"));
    assert_eq!(names.name(Rawcode(*b"dvr0")), Some("Red, variation 10"));
    // The table's own row wins.
    assert_eq!(names.name(Rawcode(*b"Hbz2")), Some("Damage"));
    assert_eq!(names.name(Rawcode(*b"Hbz4")), Some("Waves, variation 4"));
}

#[test]
fn what_a_field_is() {
    let table = "C;X1;Y1;K\"ID\"\nC;X2;K\"type\"\nC;X3;K\"minVal\"\nC;X4;K\"maxVal\"\nC;X5;K\"repeat\"\nC;X6;K\"data\"\nC;X7;K\"useSpecific\"\nC;X8;K\"field\"\n\
C;X1;Y2;K\"Hbz2\"\nC;X2;K\"unreal\"\nC;X3;K0.01\nC;X4;K\"99999\"\nC;X5;K4\nC;X6;K2\nC;X7;K\"AHbz, ACbz\"\nC;X8;K\"DataB\"\n\
C;X1;Y3;K\"anam\"\nC;X2;K\"string\"\nC;X4;K\"TTName\"\nC;X8;K\"Name\"\n\
C;X1;Y4;K\"anew\"\nC;X2;K\"fancyList\"\nC;X8;K\"New\"\n";
    let meta = Metadata::from_tables(table, &[]);
    let f = meta.get(Rawcode(*b"Hbz2")).unwrap();
    assert_eq!(f.ty, "unreal");
    assert_eq!(f.storage, Some(Storage::Unreal));
    assert_eq!((f.min, f.max), (Some(0.01), Some(99999.0)));
    assert!(f.levels);
    assert_eq!(f.column, 2);
    assert_eq!(f.specific, vec![Rawcode(*b"AHbz"), Rawcode(*b"ACbz")]);
    let f = meta.get(Rawcode(*b"anam")).unwrap();
    // A limit that names a constant isn't a number.
    assert_eq!(
        (f.storage, f.max, f.levels, f.column),
        (Some(Storage::Str), None, false, 0)
    );
    // A type this crate doesn't know isn't checked.
    assert_eq!(meta.get(Rawcode(*b"anew")).unwrap().storage, None);
}

#[test]
fn storage_of_types() {
    assert_eq!(Storage::of_type("bool"), Some(Storage::Int));
    assert_eq!(Storage::of_type("attackBits"), Some(Storage::Int));
    assert_eq!(Storage::of_type("real"), Some(Storage::Real));
    assert_eq!(Storage::of_type("targetList"), Some(Storage::Str));
    assert_eq!(Storage::of_type(""), None);
}

#[test]
fn codes() {
    let objects = "C;X1;Y1;K\"alias\"\nC;X2;K\"code\"\nC;X1;Y2;K\"AIsz\"\nC;X2;K\"Aspo\"\n";
    let meta = Metadata::default().with_codes(objects);
    assert_eq!(meta.code(Rawcode(*b"AIsz")), Rawcode(*b"Aspo"));
    assert_eq!(meta.code(Rawcode(*b"A000")), Rawcode(*b"A000"));
}

#[test]
fn a_fields_category_and_table_and_whom_it_applies_to() {
    let meta = "ID;PWXL;N;E\nC;X1;Y1;K\"ID\"\nC;X2;K\"field\"\nC;X3;K\"category\"\nC;X4;K\"slk\"\nC;X5;K\"useItem\"\n\
C;X1;Y2;K\"uhpm\"\nC;X2;K\"HP\"\nC;X3;K\"stats\"\nC;X4;K\"UnitBalance\"\nC;X5;K\"0\"\n\
C;X1;Y3;K\"unam\"\nC;X2;K\"Name\"\nC;X3;K\"text\"\nC;X4;K\"Profile\"\nC;X5;K\"1\"\nE\n";
    let m = Metadata::from_tables(meta, &[]);
    let hp = m.get(Rawcode(*b"uhpm")).unwrap();
    assert_eq!((hp.category.as_str(), hp.table.as_str()), ("stats", "UnitBalance"));
    let base = Rawcode(*b"ratc");
    assert!(!m.applies(Kind::Item, base, hp));
    assert!(m.applies(Kind::Unit, base, hp));
    let mut ids: Vec<String> = m.fields().filter(|(_, f)| m.applies(Kind::Item, base, f)).map(|(id, _)| id.to_string()).collect();
    ids.sort();
    assert_eq!(ids, ["unam"]);
}
