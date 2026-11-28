//! Each object's name, as World Editor shows it (`hfoo`: Footman,
//! `Hpal`: Paladin, `AHhb`: Holy Light), for an editor to show an id by
//! its name and to offer a kind's objects to pick from.
//!
//! The objects of a kind are the rows of its table ([`Kind::table_file`]:
//! `Units\UnitData.slk`'s `unitID` ...). The name is the field World
//! Editor shows: `Name` (a buff's `EditorName`, else its `Bufftip`; the
//! table's `comments` for what nothing names), with its `EditorSuffix`
//! after it; the table's column of that name when it
//! has one (destructables, doodads), else the object's section of the
//! profile files ([`PROFILE_FILES`]), a `WESTRING_` key resolved. An
//! upgrade's name is per level, its first shown. Then the map's object
//! data ([`ObjectNames::add_object_data`]): a custom object is its own,
//! named as its standard one unless it says otherwise; a changed one is
//! renamed.

use std::collections::{BTreeMap, HashMap};

use parser_rs::Rawcode;

use crate::{Kind, Metadata, ObjectData, Value};

/// The game's files of the objects' texts (`[hfoo]`, `Name=Footman`), the
/// profile: the game reads them all into one table of sections by id, a
/// later one over an earlier one.
pub const PROFILE_FILES: [&str; 22] = [
    "Units\\CampaignUnitStrings.txt",
    "Units\\HumanUnitStrings.txt",
    "Units\\NeutralUnitStrings.txt",
    "Units\\NightElfUnitStrings.txt",
    "Units\\OrcUnitStrings.txt",
    "Units\\UndeadUnitStrings.txt",
    "Units\\CampaignAbilityStrings.txt",
    "Units\\CommonAbilityStrings.txt",
    "Units\\HumanAbilityStrings.txt",
    "Units\\ItemAbilityStrings.txt",
    "Units\\NeutralAbilityStrings.txt",
    "Units\\NightElfAbilityStrings.txt",
    "Units\\OrcAbilityStrings.txt",
    "Units\\UndeadAbilityStrings.txt",
    "Units\\CampaignUpgradeStrings.txt",
    "Units\\HumanUpgradeStrings.txt",
    "Units\\NeutralUpgradeStrings.txt",
    "Units\\NightElfUpgradeStrings.txt",
    "Units\\OrcUpgradeStrings.txt",
    "Units\\UndeadUpgradeStrings.txt",
    "Units\\ItemStrings.txt",
    "Units\\CommandStrings.txt",
];

impl Kind {
    /// The game's table of this kind's objects, a row each, the id first.
    pub fn table_file(self) -> &'static str {
        match self {
            Kind::Ability => "Units\\AbilityData.slk",
            Kind::Destructable => "Units\\DestructableData.slk",
            Kind::Doodad => "Doodads\\Doodads.slk",
            Kind::Buff => "Units\\AbilityBuffData.slk",
            Kind::Upgrade => "Units\\UpgradeData.slk",
            Kind::Item => "Units\\ItemData.slk",
            Kind::Unit => "Units\\UnitData.slk",
        }
    }

    /// The fields its name is made of: the name (and what stands in for
    /// it when the object has none), the suffix.
    fn name_keys(self) -> (&'static [&'static str], &'static str) {
        match self {
            Kind::Buff => (&["EditorName", "Bufftip"], "EditorSuffix"),
            _ => (&["Name"], "EditorSuffix"),
        }
    }
}

/// An object and its name.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Named {
    /// Its name; its id when nothing names it.
    pub name: String,
    /// What World Editor shows after it (`(wielding Frostmourne)`).
    pub suffix: String,
    /// One the map adds.
    pub custom: bool,
    /// An ability a hero learns (`AbilityData.slk`'s `hero`).
    pub hero: bool,
}

/// The objects of each kind by id (`hfoo`), with their names.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectNames {
    pub of: HashMap<Kind, BTreeMap<String, Named>>,
}

/// A profile or table value as shown: quotes off; of a list (an upgrade's
/// levels) the first.
fn first_value(v: &str) -> String {
    let v = v.trim();
    if let Some(rest) = v.strip_prefix('"') {
        return rest.split_once('"').map_or(rest, |(a, _)| a).to_string();
    }
    v.split(',').next().unwrap_or("").trim().to_string()
}

impl Named {
    /// As World Editor shows it: the name, the suffix after it.
    pub fn shown(&self) -> String {
        let suffix = self.suffix.trim();
        if suffix.is_empty() { self.name.clone() } else { format!("{} {suffix}", self.name) }
    }
}

impl ObjectNames {
    /// Each kind's objects named by the game's files (`read` gives a
    /// file's text, `resolve` an editor's string for a `WESTRING_` key).
    pub fn load(read: impl Fn(&str) -> Option<String>, resolve: impl Fn(&str) -> String) -> ObjectNames {
        // The profile: one table of sections, the last file's over the
        // earlier ones', key by key.
        let mut profile: HashMap<String, HashMap<String, String>> = HashMap::new();
        for file in PROFILE_FILES {
            let Some(text) = read(file) else { continue };
            for section in bni_rs::parse(&text).sections {
                let entry = profile.entry(section.name.text.trim().to_lowercase()).or_default();
                for item in section.items {
                    if let Some(key) = &item.key {
                        // Its first field (an upgrade's first level's),
                        // quotes off: BNI's fields are the profile's.
                        let first = item.fields.first().map_or(String::new(), |f| f.text().trim().to_string());
                        entry.insert(key.text.trim().to_lowercase(), first);
                    }
                }
            }
        }
        let mut out = ObjectNames::default();
        for kind in Kind::ALL {
            let Some(text) = read(kind.table_file()) else { continue };
            let table = slk_rs::Table::new(&slk_rs::parse(&text));
            // The columns by their names (the first of a name), read once:
            // `cols` goes over every cell.
            let mut columns: HashMap<String, u32> = HashMap::new();
            for c in 1..=table.cols() {
                columns.entry(table.text(1, c).trim().to_ascii_lowercase()).or_insert(c);
            }
            let column = |n: &str| columns.get(&n.to_ascii_lowercase()).copied();
            let hero = column("hero");
            let (names, suffix) = kind.name_keys();
            let objects = out.of.entry(kind).or_default();
            for row in 2..=table.rows() {
                let id = table.text(row, 1).trim().to_string();
                if id.len() != 4 {
                    continue;
                }
                let own = profile.get(&id.to_lowercase());
                // The table's column, else the profile's key.
                let field = |key: &str| -> String {
                    let v = column(key).map(|c| first_value(&table.text(row, c))).filter(|v| !matches!(v.as_str(), "" | "-" | "_"));
                    let v = v.or_else(|| own.and_then(|s| s.get(&key.to_lowercase())).cloned()).unwrap_or_default();
                    resolve(&v)
                };
                // Nothing names some (item abilities' auras): the table's
                // `comments` say what they are.
                let name = names.iter().chain(&["comments"]).map(|k| field(k)).find(|n| !n.is_empty()).unwrap_or_else(|| id.clone());
                let hero = hero.is_some_and(|c| table.text(row, c).trim() == "1");
                objects.insert(id, Named { name, suffix: field(suffix), custom: false, hero });
            }
        }
        out
    }

    /// The map's object data of a kind over the game's: its custom objects
    /// added, named as their standard ones unless they say; changed ones
    /// renamed. `meta` (the kind's) gives the name fields' ids, `text` a
    /// string as shown (a `TRIGSTR_` resolved).
    pub fn add_object_data(&mut self, data: &ObjectData, meta: &Metadata, text: impl Fn(&str) -> String) {
        let kind = data.kind;
        let (names, suffix) = kind.name_keys();
        let objects = self.of.entry(kind).or_default();
        // A custom object starts from its standard one as the game has it,
        // not as the map changes it.
        let game = objects.clone();
        for o in data.objects() {
            let own = o.id != Rawcode::NONE;
            let keys = meta.keys(kind, o.base);
            let fields: Vec<Rawcode> = names.iter().filter_map(|k| keys.field(k)).collect();
            let suffix_field = keys.field(suffix);
            let hero_field = keys.field("hero");
            let id = if own { o.id } else { o.base }.to_string();
            let base = game.get(&o.base.to_string()).cloned().unwrap_or_else(|| Named { name: o.base.to_string(), ..Named::default() });
            // The base's parts, the name and suffix apart: the suffix is
            // what's after the name.
            let mods: Vec<&crate::Mod> = o.sets.iter().flat_map(|s| &s.mods).collect();
            let string = |f: Rawcode| {
                mods.iter().rev().find(|m| m.field == f && m.level <= 1).and_then(|m| match &m.value {
                    Value::Str(s) => Some(first_value(&text(s))),
                    _ => None,
                })
            };
            // What it doesn't change is its standard one's.
            let name = fields.iter().find_map(|f| string(*f).filter(|n| !n.is_empty())).unwrap_or(base.name);
            let suffix = suffix_field.and_then(string).unwrap_or(base.suffix);
            let hero = hero_field.and_then(|f| mods.iter().rev().find(|m| m.field == f)).map_or(base.hero, |m| matches!(m.value, Value::Int(1)));
            objects.insert(id, Named { name, suffix, custom: own, hero });
        }
    }

    /// Object `id` of `kind`.
    pub fn get(&self, kind: Kind, id: &str) -> Option<&Named> {
        self.of.get(&kind)?.get(id)
    }
}

#[cfg(test)]
#[path = "names.test.rs"]
mod tests;
