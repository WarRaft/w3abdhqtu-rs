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
```

The CLI reads files, prints a summary (`-v`: every field) and checks that
writing each one back gives the same bytes:

```text
$ w3abdhqtu-rs -v war3map.w3a
war3map.w3a: version 2, 2 changed, 7 custom, 108 fields
ACwe
  arlv [0, 0] = 6
  aret [0, 0] = "TRIGSTR_287"
A000 : ANwm
  aub1 [2, 0] = "TRIGSTR_165"
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
- Not yet: field names, types and value checks from the metadata SLKs,
  and the text form for editing in VS Code.

## Tests

```sh
cargo test
W3ABDHQTU_CORPUS=path/to/files cargo test --lib corpus -- --ignored
```

The corpus test reads every `.w3?` file in the folder and checks the
round trip.
