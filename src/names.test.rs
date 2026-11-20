use super::*;

const META: &str = "ID;PWXL;N;E\nC;X1;Y1;K\"ID\"\nC;X2;K\"field\"\nC;X3;K\"displayName\"\n\
C;X1;Y2;K\"uhpm\"\nC;X2;K\"HP\"\nC;X3;K\"WESTRING_UEVAL_UHPM\"\n\
C;X1;Y3;K\"unam\"\nC;X2;K\"Name\"\nC;X3;K\"WESTRING_MISSING\"\n\
C;X1;Y4;K\"uabi\"\nC;X3;K\"WESTRING_UEVAL_UABI\"\n\
C;X1;Y5;K\"long\"\nC;X2;K\"x\"\n\
C;X1;Y6;K\"uhpm\"\nC;X3;K\"WESTRING_UEVAL_UABI\"\nE\n";

#[test]
fn names_from_metadata_and_strings() {
    let strings = "[WorldEditStrings]\r\nWESTRING_UEVAL_UHPM=Hit Points Maximum (Base)\r\nWESTRING_UEVAL_UABI = \"Abilities\"\r\n";
    let over = "WESTRING_UEVAL_UABI=Abilities - Normal\n";
    let names = FieldNames::from_metadata(META, &[strings, over]);
    assert_eq!(
        names.get(Rawcode(*b"uhpm")),
        Some("Hit Points Maximum (Base)")
    );
    // Not among the strings: the `field` column.
    assert_eq!(names.get(Rawcode(*b"unam")), Some("Name"));
    // A later strings file wins.
    assert_eq!(names.get(Rawcode(*b"uabi")), Some("Abilities - Normal"));
    assert_eq!(names.get(Rawcode(*b"long")), Some("x"));
    assert_eq!(names.get(Rawcode(*b"umvs")), None);
    assert_eq!(names.len(), 4);
}

#[test]
fn no_table() {
    assert!(FieldNames::from_metadata("", &[]).is_empty());
    assert!(FieldNames::from_metadata("C;X1;Y1;K\"field\"\n", &[]).is_empty());
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
    let names = FieldNames::from_metadata(meta, &[]);
    assert_eq!(names.get(Rawcode(*b"dvr1")), Some("Red"));
    assert_eq!(names.get(Rawcode(*b"dvr2")), Some("Red, variation 2"));
    assert_eq!(names.get(Rawcode(*b"dvr9")), Some("Red, variation 9"));
    assert_eq!(names.get(Rawcode(*b"dvr0")), Some("Red, variation 10"));
    // The table's own row wins.
    assert_eq!(names.get(Rawcode(*b"Hbz2")), Some("Damage"));
    assert_eq!(names.get(Rawcode(*b"Hbz4")), Some("Waves, variation 4"));
}
