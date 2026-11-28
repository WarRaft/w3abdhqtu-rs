//! What the game says about each field, from its files: each kind's
//! metadata table ([`Kind::metadata_file`]) gives a field's type, limits,
//! levels and data column, and its `displayName`, a key into the editor's
//! strings ([`STRINGS_FILES`]) that gives the name World Editor shows
//! (`uhpm` is "Hit Points Maximum (Base)"). Which game version's tables
//! they are decides which fields exist, so they come from the project's
//! game, never from a built-in copy.

use std::collections::HashMap;

use parser_rs::Rawcode;

use crate::{Kind, Value};

/// The game files with the editor's strings, `KEY=value` lines; a later
/// one wins over an earlier one.
pub const STRINGS_FILES: [&str; 2] = ["UI\\WorldEditStrings.txt", "UI\\WorldEditGameStrings.txt"];

impl Kind {
    /// The game's table of this kind's fields. Items share the units'.
    pub fn metadata_file(self) -> &'static str {
        match self {
            Kind::Ability => "Units\\AbilityMetaData.slk",
            Kind::Destructable => "Units\\DestructableMetaData.slk",
            Kind::Doodad => "Doodads\\DoodadMetaData.slk",
            Kind::Buff => "Units\\AbilityBuffMetaData.slk",
            Kind::Upgrade => "Units\\UpgradeMetaData.slk",
            Kind::Item | Kind::Unit => "Units\\UnitMetaData.slk",
        }
    }

    /// The game's table of this kind's objects, when the checks need it:
    /// abilities', whose `code` says which ability's data fields an
    /// ability has (`AIsz`, an item's slow poison, has `Aspo`'s).
    pub fn objects_file(self) -> Option<&'static str> {
        (self == Kind::Ability).then_some("Units\\AbilityData.slk")
    }
}

/// How a field's value is stored: the file's value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Storage {
    Int,
    Real,
    Unreal,
    Str,
}

/// Metadata types stored as integers: numbers, booleans and bit flags.
const INT_TYPES: &[&str] = &[
    "int",
    "bool",
    "attackBits",
    "channelFlags",
    "channelType",
    "deathType",
    "defenseTypeInt",
    "detectionType",
    "fullFlags",
    "interactionFlags",
    "morphFlags",
    "pickFlags",
    "silenceFlags",
    "spellDetail",
    "stackFlags",
    "teamColor",
    "versionFlags",
];

/// Metadata types stored as strings: text, ids, lists, file paths and
/// the names of the game's enumerations (`attackType` is `"normal"` ...).
const STRING_TYPES: &[&str] = &[
    "string",
    "char",
    "stringList",
    "orderString",
    "abilCode",
    "abilityList",
    "heroAbilityList",
    "aiBuffer",
    "armorType",
    "attackType",
    "attributeType",
    "buffList",
    "combatSound",
    "defenseType",
    "destructableCategory",
    "doodadCategory",
    "effectList",
    "icon",
    "intList",
    "itemClass",
    "itemList",
    "lightningEffect",
    "lightningList",
    "model",
    "modelList",
    "moveType",
    "pathingListPrevent",
    "pathingListRequire",
    "pathingTexture",
    "regenType",
    "shadowImage",
    "shadowTexture",
    "soundLabel",
    "targetList",
    "techList",
    "texture",
    "tilesetList",
    "uberSplat",
    "unitClass",
    "unitCode",
    "unitList",
    "unitRace",
    "unitSound",
    "upgradeClass",
    "upgradeCode",
    "upgradeEffect",
    "upgradeList",
    "weaponType",
];

impl Storage {
    /// How a value of the metadata type `ty` is stored; `None` for a type
    /// this crate doesn't know (a newer game's), which isn't checked.
    pub fn of_type(ty: &str) -> Option<Storage> {
        match ty {
            "real" => Some(Storage::Real),
            "unreal" => Some(Storage::Unreal),
            _ if INT_TYPES.contains(&ty) => Some(Storage::Int),
            _ if STRING_TYPES.contains(&ty) => Some(Storage::Str),
            _ => None,
        }
    }

    pub fn of_value(value: &Value) -> Storage {
        match value {
            Value::Int(_) => Storage::Int,
            Value::Real(_) => Storage::Real,
            Value::Unreal(_) => Storage::Unreal,
            Value::Str(_) => Storage::Str,
        }
    }

    /// What it is, for messages.
    pub fn describe(self) -> &'static str {
        match self {
            Storage::Int => "a whole number",
            Storage::Real => "a real",
            Storage::Unreal => "an unreal",
            Storage::Str => "a string",
        }
    }
}

/// One field, as the metadata table describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldMeta {
    /// The name World Editor shows.
    pub name: String,
    /// The name the game's data tables give the field: the `field` column
    /// (`Cool`, `HP`), with the data column's letter for an ability's data
    /// (`DataA`, the column `AbilityData.slk` keeps it in). The text form
    /// writes it as the field's key where it names one field only
    /// ([`Metadata::keys`]). Empty for a field the table has no row of
    /// (`dvr2`, version 1 doodads' copies).
    pub key: String,
    /// Which value of a list field it is (`index`: `Buttonpos` is X at 0,
    /// Y at 1), when the table says.
    pub index: Option<u32>,
    /// The metadata type: `int`, `unreal`, `targetList`, ...
    pub ty: String,
    /// How it's stored, when the type is known.
    pub storage: Option<Storage>,
    /// World Editor's limits for a number.
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Whether it has a value per level (doodads: per variation).
    pub levels: bool,
    /// The data column (`Data A` is 1), 0 for none.
    pub column: u32,
    /// The only objects (abilities) it belongs to; empty for all.
    pub specific: Vec<Rawcode>,
    /// Objects it doesn't belong to.
    pub not_specific: Vec<Rawcode>,
    /// For the units' table: whether items have it, and whether units,
    /// heroes or buildings do. `None` when the table doesn't say.
    pub items: Option<bool>,
    pub units: Option<bool>,
    /// Whether a unit (not a hero or building), a hero, a building has
    /// it (`useUnit`, `useHero`, `useBuilding`), each `None` when the
    /// table doesn't say.
    pub unit_kinds: [Option<bool>; 3],
    /// World Editor's group of it (`category`: `abil`, `art`, `combat`,
    /// `data`, `stats`, `text` ...), empty when the table doesn't say.
    pub category: String,
    /// The game's table it's read from (`slk`: `UnitData`, `UnitBalance`,
    /// `Profile` for the text files ...), empty when the table doesn't say.
    pub table: String,
}

/// An object's fields by key and keys by field ([`Metadata::keys`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Keys {
    by_id: HashMap<Rawcode, String>,
    by_key: HashMap<String, Rawcode>,
}

impl Keys {
    /// The key that names `field`, if one does.
    pub fn key(&self, field: Rawcode) -> Option<&str> {
        self.by_id.get(&field).map(String::as_str)
    }

    /// The field a key names.
    pub fn field(&self, key: &str) -> Option<Rawcode> {
        self.by_key.get(key).copied()
    }
}

/// A kind's fields by id. Empty when the game's files aren't at hand.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    fields: HashMap<Rawcode, FieldMeta>,
    /// Standard objects' codes ([`Kind::objects_file`]), by id.
    codes: HashMap<Rawcode, Rawcode>,
}

impl Metadata {
    /// Reads the metadata table (`.slk`) and the strings files' texts,
    /// in their order. A field whose `displayName` isn't among the
    /// strings is named by its `field` column (`Name`, `Hotkey`). The
    /// first row of an id wins.
    ///
    /// A field the table repeats (`repeat`) whose id ends in `1` stands
    /// for its other copies too, `dvr2` to `dvr9` and `dvr0` (the tenth),
    /// unless the table has them itself: version 1 doodads have no levels,
    /// so a variation is the id's last digit.
    pub fn from_tables(metadata: &str, strings: &[&str]) -> Metadata {
        let mut keys = HashMap::new();
        for text in strings {
            for line in parser_rs::lines::lines(&text) {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                let value = value.trim();
                let value = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap_or(value);
                keys.insert(key.trim(), value);
            }
        }

        let rows = rows(metadata);
        let Some((header, rows)) = rows.split_first() else {
            return Metadata::default();
        };
        let column = |name: &str| header.iter().position(|h| h == name);
        let Some(id) = column("ID") else {
            return Metadata::default();
        };
        let cols = Columns {
            display: column("displayName"),
            field: column("field"),
            index: column("index"),
            ty: column("type"),
            min: column("minVal"),
            max: column("maxVal"),
            repeat: column("repeat"),
            data: column("data"),
            specific: column("useSpecific"),
            not_specific: column("notSpecific"),
            items: column("useItem"),
            units: [column("useUnit"), column("useHero"), column("useBuilding")],
            category: column("category"),
            table: column("slk"),
        };

        let mut fields = HashMap::new();
        let mut repeated = Vec::new();
        for row in rows {
            let Some(code) =
                cell(row, Some(id)).and_then(|s| <[u8; 4]>::try_from(s.as_bytes()).ok())
            else {
                continue;
            };
            let Some(meta) = cols.field(row, &keys) else {
                continue;
            };
            let times = cell(row, cols.repeat)
                .and_then(|r| r.parse::<u8>().ok())
                .unwrap_or(0);
            if times > 1 && code[3] == b'1' {
                repeated.push((code, times.min(10), meta.clone()));
            }
            fields.entry(Rawcode(code)).or_insert(meta);
        }
        for (code, times, meta) in repeated {
            for n in 2..=times {
                let mut id = code;
                id[3] = b'0' + n % 10;
                fields.entry(Rawcode(id)).or_insert_with(|| FieldMeta {
                    name: format!("{}, variation {n}", meta.name),
                    key: String::new(),
                    ..meta.clone()
                });
            }
        }
        Metadata {
            fields,
            codes: HashMap::new(),
        }
    }

    /// Adds the standard objects' codes from the objects table
    /// ([`Kind::objects_file`]): its `alias` (the id) and `code` columns.
    pub fn with_codes(mut self, objects: &str) -> Metadata {
        let rows = rows(objects);
        let Some((header, rows)) = rows.split_first() else {
            return self;
        };
        let column = |name: &str| header.iter().position(|h| h == name);
        let (Some(alias), Some(code)) = (column("alias"), column("code")) else {
            return self;
        };
        let id = |row: &[String], col| {
            cell(row, Some(col))
                .and_then(|s| <[u8; 4]>::try_from(s.as_bytes()).ok())
                .map(Rawcode)
        };
        for row in rows {
            if let (Some(alias), Some(code)) = (id(row, alias), id(row, code)) {
                self.codes.entry(alias).or_insert(code);
            }
        }
        self
    }

    /// The code of the standard object `id`: whose data fields it has.
    /// Itself when the objects table doesn't say.
    pub fn code(&self, id: Rawcode) -> Rawcode {
        self.codes.get(&id).copied().unwrap_or(id)
    }

    /// The metadata of `kind` from the game's files, `read` giving a file's
    /// text by its name (`Units\\AbilityMetaData.slk`; `None` when the game
    /// hasn't it): its table ([`Kind::metadata_file`]), the editor's strings
    /// ([`STRINGS_FILES`]) and its objects' codes ([`Kind::objects_file`]).
    /// Empty without the table. Finding the files (archives, game folders)
    /// is the caller's job.
    pub fn load(kind: Kind, read: impl Fn(&str) -> Option<String>) -> Metadata {
        let Some(table) = read(kind.metadata_file()) else {
            return Metadata::default();
        };
        let strings: Vec<String> = STRINGS_FILES.iter().filter_map(|f| read(f)).collect();
        let strings: Vec<&str> = strings.iter().map(String::as_str).collect();
        let meta = Metadata::from_tables(&table, &strings);
        match kind.objects_file().and_then(&read) {
            Some(objects) => meta.with_codes(&objects),
            None => meta,
        }
    }

    /// The keys the text form names an object's fields by: each field's
    /// [`FieldMeta::key`], for the fields an object of `kind` based on
    /// `base` has (an ability's by its code, items' and units' by their
    /// table), when no other of its fields has the same key, or with its
    /// `index` as a letter when that tells them apart (`ButtonposA`). The
    /// others keep their ids.
    pub fn keys(&self, kind: Kind, base: Rawcode) -> Keys {
        let has = |f: &FieldMeta| self.applies(kind, base, f);
        let mut by_key: HashMap<&str, Vec<(Rawcode, Option<u32>)>> = HashMap::new();
        for (&id, f) in &self.fields {
            if !f.key.is_empty() && has(f) {
                by_key.entry(&f.key).or_default().push((id, f.index));
            }
        }
        // A name several fields share is told apart by their `index`, as a
        // letter (`Buttonpos` at 0 and 1: `ButtonposA`, `ButtonposB`),
        // when each has its own; else they keep their ids.
        let mut named: HashMap<String, Vec<Rawcode>> = HashMap::new();
        for (key, fields) in by_key {
            if let [(id, _)] = fields[..] {
                named.entry(key.to_string()).or_default().push(id);
                continue;
            }
            let mut indexes: Vec<u32> = fields
                .iter()
                .filter_map(|f| f.1)
                .filter(|&i| i < 26)
                .collect();
            indexes.sort();
            indexes.dedup();
            if indexes.len() == fields.len() {
                for (id, index) in fields {
                    let letter = (b'A' + index.expect("all have one") as u8) as char;
                    named.entry(format!("{key}{letter}")).or_default().push(id);
                }
            }
        }
        let mut keys = Keys::default();
        for (key, ids) in named {
            if let [id] = ids[..] {
                keys.by_id.insert(id, key.clone());
                keys.by_key.insert(key, id);
            }
        }
        keys
    }

    /// Whether an object of `kind` based on `base` has field `f`: an
    /// ability's by its code (`useSpecific`, `notSpecific`), items' and
    /// units' by their table's `useItem`, `useUnit` ... columns.
    pub fn applies(&self, kind: Kind, base: Rawcode, f: &FieldMeta) -> bool {
        let code = self.code(base);
        let named = |ids: &[Rawcode]| ids.contains(&base) || ids.contains(&code);
        (f.specific.is_empty() || named(&f.specific))
            && !named(&f.not_specific)
            && match kind {
                Kind::Item => f.items != Some(false),
                Kind::Unit => f.units != Some(false),
                _ => true,
            }
    }

    /// Whether a unit (`0`), a hero (`1`) or a building (`2`) has field
    /// `f`, as World Editor shows each its own: by `useUnit`, `useHero`,
    /// `useBuilding`; one the table says nothing of, yes.
    pub fn for_unit(f: &FieldMeta, which: usize) -> bool {
        f.unit_kinds.get(which).copied().flatten() != Some(false)
    }

    /// Every field, by id, in no order.
    pub fn fields(&self) -> impl Iterator<Item = (Rawcode, &FieldMeta)> {
        self.fields.iter().map(|(id, f)| (*id, f))
    }

    pub fn get(&self, field: Rawcode) -> Option<&FieldMeta> {
        self.fields.get(&field)
    }

    /// The name World Editor shows for `field`.
    pub fn name(&self, field: Rawcode) -> Option<&str> {
        self.get(field).map(|f| f.name.as_str())
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

/// Where the table keeps each thing.
struct Columns {
    display: Option<usize>,
    field: Option<usize>,
    index: Option<usize>,
    ty: Option<usize>,
    min: Option<usize>,
    max: Option<usize>,
    repeat: Option<usize>,
    data: Option<usize>,
    specific: Option<usize>,
    not_specific: Option<usize>,
    items: Option<usize>,
    units: [Option<usize>; 3],
    category: Option<usize>,
    table: Option<usize>,
}

impl Columns {
    fn field(&self, row: &[String], keys: &HashMap<&str, &str>) -> Option<FieldMeta> {
        let name = cell(row, self.display)
            .and_then(|key| keys.get(key).copied())
            .filter(|name| !name.is_empty())
            .or_else(|| cell(row, self.field))?;
        let ty = cell(row, self.ty).unwrap_or("").to_string();
        let number = |col| cell(row, col).and_then(|v| v.parse::<f64>().ok());
        let flag = |col| cell(row, col).map(|v| v != "0");
        let ids = |col| {
            cell(row, col).map_or(Vec::new(), |list| {
                list.split(',')
                    .filter_map(|id| <[u8; 4]>::try_from(id.trim().as_bytes()).ok())
                    .map(Rawcode)
                    .collect()
            })
        };
        let units = self
            .units
            .iter()
            .filter_map(|&c| flag(c))
            .reduce(|a, b| a || b);
        let column = number(self.data).map_or(0, |d| d as u32);
        let mut key = cell(row, self.field).unwrap_or("").to_string();
        if (1..=26).contains(&column) {
            key.push((b'A' + (column - 1) as u8) as char);
        }
        Some(FieldMeta {
            name: name.to_string(),
            key,
            index: cell(row, self.index).and_then(|i| i.parse().ok()),
            storage: Storage::of_type(&ty),
            ty,
            min: number(self.min),
            max: number(self.max),
            levels: number(self.repeat).is_some_and(|r| r > 0.0),
            column,
            specific: ids(self.specific),
            not_specific: ids(self.not_specific),
            items: flag(self.items),
            units,
            unit_kinds: self.units.map(|c| flag(c)),
            category: cell(row, self.category).unwrap_or("").to_string(),
            table: cell(row, self.table).unwrap_or("").to_string(),
        })
    }
}

/// A table's rows, each its cells' text by column from the first, read
/// with `slk-rs`; missing cells are empty.
fn rows(text: &str) -> Vec<Vec<String>> {
    let table = slk_rs::Table::new(&slk_rs::parse(text));
    // `cols` goes over every cell: once.
    let cols = table.cols();
    (1..=table.rows())
        .map(|r| (1..=cols).map(|c| table.text(r, c)).collect())
        .collect()
}

fn cell(row: &[String], col: Option<usize>) -> Option<&str> {
    col.and_then(|c| row.get(c))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
#[path = "metadata.test.rs"]
mod tests;
