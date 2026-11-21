# w3abdhqtu-rs

Reader and writer for Warcraft III **object data**: the files where a map
keeps its changes to the game's objects, and the objects it adds.

| file              | objects       | levels |
|-------------------|---------------|--------|
| `war3map.w3a`     | abilities     | yes    |
| `war3map.w3b`     | destructables | no     |
| `war3map.w3d`     | doodads       | since version 2 (variations) |
| `war3map.w3h`     | buffs         | no     |
| `war3map.w3q`     | upgrades      | yes    |
| `war3map.w3t`     | items         | no     |
| `war3map.w3u`     | units (in `.w3m` maps, items too) | no |

Campaigns keep the same files as `war3campaign.w3?`; Reforged maps can add
`war3mapSkin.w3?`.

Built on [`parser-rs`](../parser-rs)'s `bin` module (see its design notes,
principle 12) for the
[`wc3-rs`](../wc3-rs) VS Code extension.

## Use

```rust
use w3abdhqtu_rs::{parse, Kind, Value};

let bytes = std::fs::read("war3map.w3u")?;
let data = parse(&bytes, Kind::Unit)?;     // Kind::from_path("war3map.w3u") works too
for object in &data.customs {
    println!("{} from {}", object.id, object.base);
    for m in object.mods() {
        if let Value::Str(name) = &m.value {
            println!("  {} = {name}", m.field);
        }
    }
}
assert_eq!(data.to_bytes()?, bytes);       // always the same bytes

let text = data.to_text();                 // the text form, in BNI
let back = w3abdhqtu_rs::from_text(&text);
assert!(back.errors.is_empty());
assert_eq!(back.data, Some(data));         // and back to the same data
```

The CLI:

```text
w3abdhqtu-rs <file.w3?>...          check: read, write back, and through the text form
w3abdhqtu-rs --text <file.w3?>      print the text form
w3abdhqtu-rs --text <file.w3?> <game>
                                    with field names, from the game's files
                                    extracted to the folder <game>
w3abdhqtu-rs --check <file> <game>  check a .w3? file or a text form against
                                    the game's metadata
w3abdhqtu-rs --build <text> <out>   write the binary file from a text form
```

## Format

Little-endian throughout. Ids are four bytes in reading order (`hfoo`).

```text
file      u32 version              1, 2; 3 since Reforged (1.33)
          table originals          standard objects the map changes
          table customs            objects the map adds
table     u32 count, object[count]
object    id  base                 the standard object (hfoo)
          id  custom               the new id (h000); 0 in originals
          version >= 3:  u32 count, set[count]
          otherwise:     mods      one set, no flags
set       u32 flags                game modes it applies to
          mods
mods      u32 count, mod[count]
mod       id  field                from the type's metadata SLK (unam)
          u32 type                 0 int, 1 real, 2 unreal (0..1), 3 string
          if levels:  u32 level    1-based, 0 if the field has none; doodads: the variation
                      u32 column   data column: 0 none, 1 = Data A, ...
          value                    i32 | f32 | f32 | NUL-terminated UTF-8
          id  end                  the object's id or 0; not checked by the game
```

Abilities and upgrades always store `level` and `column`; doodads only
since version 2. Version 1 doodads (Reign of Chaos campaign maps) go
straight from the type to the value.

## Text form

The text form is [BNI](../bni-rs/README.md), so the BNI parser, its
highlighting and its checks all apply; this crate reads the parsed
document (`from_text`, or `from_bni` for one already parsed). It's what
the [`wc3-rs`](../wc3-rs) editor shows and saves.

```ini
// Warcraft III abilities (war3map.w3a). Saving writes the binary file back.
[w3a]
version=2

[ACwe]
arlv=6 // Required Level: int
aret="TRIGSTR_287" // Tooltip - Learn: string

[A000]
_parent=AHbz
anam="Blizzard" // Name: string
aub1[2]="Line one"\n"Line two" // Tooltip - Normal - Extended: string
Hbz1[3].A=12 // Number of Waves: int
Hbz2[3].B=0.25 // Damage: unreal
```

- **The file's section** comes first: `[w3a]` ... `[w3u]`, with
  `version=` 1, 2 or 3.
- **An object** is a section named by its id. A custom object has
  `_parent=` its base; a changed standard object doesn't. Ids are four
  letters, digits or `_`, or else `0x` and eight hex digits (the value
  JASS gives `'hfoo'`).
- **A field** is `id=value`. Abilities, upgrades and doodads (since
  version 2) add `[level]` and the data column as `.A` for `Data A` (1)
  ... `.Z`, or `.27` past that; 0s are left out.
- **Values.** `"text"` is a string, anything else a number of the field's
  type, from the game's metadata (`from_text(text, &meta)`): `Hbz2` is an
  unreal, so `Hbz2[3].B=0.25` is one; the comment and the editor's hover
  say which type it is. A number whose type isn't its field's says it:
  `int 3`, `real 1.5`, `unreal 0.25`; so does any unreal when there's no
  metadata, where a whole number is an integer and any other a real
  (`1.5`, `1e-7`, `-0.0`, `inf`). A real whose number can't say it exactly
  (a NaN with a payload) is written as its bits: `real 0x7FC00001`. Real
  files need none of this: over 104,000 fields, none says its type.
- **Strings.** BNI strings have no escapes and no line breaks, so what
  they can't hold goes between quoted parts, unquoted: `\n` `\r` `\t` and
  `\u{22}` for `"` (or any other character by its hex code). The parts
  join into one string: `"a"\r\n"b"`.
- **No `end` id.** Each field ends with an id the game doesn't read
  (World Editor writes the base id in changed standard objects, 0 in
  custom ones). The text leaves it out, and saving writes 0, so a file
  that goes through the text changes there and nowhere else
  (`ObjectData::clear_ends`).
- **Rare cases.** Sets (version 3) start with `_flags=` and their flags;
  fields before the first one are in a set with flags 0. `_table=original`
  or `_table=custom` puts an object in the other table than `_parent`
  says. Real files have none of these but sets.
- **Errors.** Every line with a problem is reported, with its place,
  and there's data only when there are none.
- **Field names and types** (`to_text_named`) are comments after the
  fields: the names World Editor shows and the metadata types. They're
  only for reading, and come from the game's files (below); without them
  the text is the same minus the comments.
- Comments and spacing aren't kept: the binary file has no room for them.

## Metadata

`Metadata::from_tables(table, strings)` reads a kind's metadata table
(`Kind::metadata_file`: `Units\AbilityMetaData.slk` for abilities,
`Units\UnitMetaData.slk` for units and items, ...) and the editor's
strings (`STRINGS_FILES`: `UI\WorldEditStrings.txt`, then
`UI\WorldEditGameStrings.txt` over it). `with_codes(objects)` adds the
abilities' codes from `Units\AbilityData.slk` (`Kind::objects_file`).
They come from the project's game, never from a copy in this crate: the
game version decides which fields there are.

Each row is a field (`FieldMeta`), by its `ID`:

- **name**: `displayName` is a key into the strings (`WESTRING_UEVAL_UHPM`
  → `Hit Points Maximum (Base)`); a key the strings don't have falls back
  to the row's `field` (`HP`). The first row of an id wins.
- **type** (`type`) and how it's stored (`Storage`). Checked on 104,000
  fields of real maps: `int`, `bool` and the flag types (`attackBits`,
  `channelFlags`, ...) are integers, `real` reals, `unreal` unreals, and
  everything else (lists, ids, models, the game's enumerations like
  `attackType`) strings. A type not in these lists, a newer game's, isn't
  checked.
- **limits** (`minVal`, `maxVal`) when they're numbers; `TTName` and the
  like aren't.
- **levels** (`repeat` above 0) and the **data column** (`data`, `Data A`
  is 1).
- **whose** it is: `useSpecific` / `notSpecific` list ability codes;
  `useItem` and `useUnit` / `useHero` / `useBuilding` say whether items
  and units have a units' table field.

## Checks

`check(&parsed, &meta)` checks a parsed text form (`from_text` /
`from_bni` keep each field with its place, `Parsed::fields`, even when
other lines have errors) and gives lint diagnostics, each its own rule:

| rule | level | finds |
|------|-------|-------|
| `object-value-type` | error | a value stored as another type than the field's: a number in a string field (`anam=5`), a string in a number field, a type written out that isn't the field's (`acdn[1]=int 6` for an unreal). The game misreads it. Quick fix: the same value as the field's type (`"5"`, `6.0`) |
| `object-value-range` | warning | a number outside World Editor's limits, a `bool` other than 0 or 1. World Editor lets a map go past its limits (Shift+Enter), so real maps do: 562 of 104,000 fields |
| `object-unknown-field` | warning | a field the game's metadata doesn't have: a typo, or a field of a newer game |
| `object-field-level` | warning | a level on a field without levels, level 0 on one with (old maps have a few, Blizzard's campaign too), another data column than the field's. Quick fix: the right key |
| `object-field-owner` | warning | an ability's data field on an ability whose code isn't among its `useSpecific` (`Hbz1` on `AHtb`; `AIsz`'s code is `Aspo`, so it has `Aspo`'s fields), a units-only field on an item. `.w3u` isn't checked for items' fields: Reign of Chaos maps keep items there |

Version 1 doodads have no levels, so nothing is checked about them.
Without metadata, nothing is checked.

`field_at(&doc, line, col)` finds the field key under a cursor (for a
hover): the field, its object and the standard object that is based on,
read from the key and the section alone, so a broken value doesn't matter.

Version 1 doodads have no levels, so a variation is the id's last digit:
`dvr2` is the second variation's `dvr1`, `dvr0` the tenth's. A row with
`repeat` whose id ends in `1` names these too (`..., variation 2`),
unless the table has a row of their own.

The tables are read with [`slk-rs`](../slk-rs/README.md). Finding the
files (archives, game folders) is the caller's job.

## Data model

`ObjectData { kind, version, originals, customs }` →
`Object { base, id, sets }` → `Set { flags, mods }` →
`Mod { field, level, column, value, end }`, `Value::{Int, Real, Unreal, Str}`.

Everything the file stores is kept, including the `end` id, so a file
writes back byte for byte. Before version 3 an object has exactly one set
with `flags` 0; `to_bytes` refuses anything else, an unknown version, or
a string containing NUL.

## Status

- Reads and writes versions 1–3 of all seven files.
- Checked byte for byte on 311 files from 208 maps and a campaign
  (Blizzard's melee and campaign maps, custom maps). No version 3 file
  was among them; version 3 is covered by unit tests.
- The text form reads back every one of them unchanged but the `end`
  ids, with 1.26's metadata and without.
- Every field of those files gets a name from 1.26's metadata.
- Checked against 1.26's metadata: no wrong types, unknown fields or
  fields of other objects in any of them; 562 values past World Editor's
  limits and 20 level-0 values (rules `object-value-range` and
  `object-field-level`).

## Tests

```sh
cargo test
W3ABDHQTU_CORPUS=path/to/files cargo test --lib corpus -- --ignored
W3ABDHQTU_CORPUS=path/to/files W3ABDHQTU_GAME=path/to/extracted/game \
    cargo test --lib corpus -- --ignored
```

The corpus test reads every `.w3?` file in the folder and checks both
round trips: through the bytes and through the text form. With
`W3ABDHQTU_GAME`, the game's files extracted to a folder
(`Units/UnitMetaData.slk`, `UI/WorldEditStrings.txt`, ...), the text has
the field names, every field must get one, and `check` must find nothing
but values past the limits and level-0 leftovers.
