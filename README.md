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
arlv=6
aret="TRIGSTR_287",end 0x00000000

[A000]
_parent=ANwm
anam="Blizzard"
aub1[2]="Line one"\n"Line two"
Hbz1[3].B=unreal 0.25,end A000
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
- **Values.** `500` is an integer; `1.5`, `1e-7`, `-0.0`, `inf` are
  reals; `unreal 0.25` is an unreal; `"text"` is a string. A real whose
  number can't say it exactly (a NaN with a payload) is written as its
  bits: `real 0x7FC00001`.
- **Strings.** BNI strings have no escapes and no line breaks, so what
  they can't hold goes between quoted parts, unquoted: `\n` `\r` `\t` and
  `\u{22}` for `"` (or any other character by its hex code). The parts
  join into one string: `"a"\r\n"b"`.
- **The `end` id.** Each field ends with an id the game doesn't read. The
  World Editor mostly writes the base id in changed standard objects and
  0 in custom ones; when a field has another, it's written after the
  value: `,end A000`.
- **Rare cases.** Sets (version 3) start with `_flags=` and their flags;
  fields before the first one are in a set with flags 0. `_table=original`
  or `_table=custom` puts an object in the other table than `_parent`
  says. Real files have none of these but sets.
- **Errors.** Every line with a problem is reported, with its place,
  and there's data only when there are none.
- Comments and spacing aren't kept: the binary file has no room for them.

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
- The text form reads back every one of them unchanged.
- Not yet: field names, types and value checks from the metadata SLKs.

## Tests

```sh
cargo test
W3ABDHQTU_CORPUS=path/to/files cargo test --lib corpus -- --ignored
```

The corpus test reads every `.w3?` file in the folder and checks both
round trips: through the bytes and through the text form.
