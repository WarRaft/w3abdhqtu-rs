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
            for line in text.lines() {
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

        let rows = crate::slk::table(metadata);
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
            ty: column("type"),
            min: column("minVal"),
            max: column("maxVal"),
            repeat: column("repeat"),
            data: column("data"),
            specific: column("useSpecific"),
            not_specific: column("notSpecific"),
            items: column("useItem"),
            units: [column("useUnit"), column("useHero"), column("useBuilding")],
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
        let rows = crate::slk::table(objects);
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
    ty: Option<usize>,
    min: Option<usize>,
    max: Option<usize>,
    repeat: Option<usize>,
    data: Option<usize>,
    specific: Option<usize>,
    not_specific: Option<usize>,
    items: Option<usize>,
    units: [Option<usize>; 3],
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
        Some(FieldMeta {
            name: name.to_string(),
            storage: Storage::of_type(&ty),
            ty,
            min: number(self.min),
            max: number(self.max),
            levels: number(self.repeat).is_some_and(|r| r > 0.0),
            column: number(self.data).map_or(0, |d| d as u32),
            specific: ids(self.specific),
            not_specific: ids(self.not_specific),
            items: flag(self.items),
            units,
        })
    }
}

fn cell(row: &[String], col: Option<usize>) -> Option<&str> {
    col.and_then(|c| row.get(c))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
#[path = "metadata.test.rs"]
mod tests;
