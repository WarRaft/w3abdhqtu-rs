//! The names World Editor shows for fields (`uhpm` is "Hit Points Maximum
//! (Base)"), from the game's files: each kind's metadata table
//! ([`Kind::metadata_file`]) gives a field's `displayName`, a key into
//! the editor's strings ([`STRINGS_FILES`]).

use std::collections::HashMap;

use parser_rs::Rawcode;

use crate::Kind;

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
}

/// Field names by field id. Empty when the game's files aren't at hand.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldNames {
    names: HashMap<Rawcode, String>,
}

impl FieldNames {
    /// Reads the metadata table (`.slk`) and the strings files' texts,
    /// in their order. A field whose `displayName` isn't among the
    /// strings is named by its `field` column (`Name`, `Hotkey`). A field
    /// the table repeats (`repeat`) whose id ends in `1` names its other
    /// copies too, `dvr2` to `dvr9` and `dvr0` (the tenth), unless the table has them itself.
    pub fn from_metadata(metadata: &str, strings: &[&str]) -> FieldNames {
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
            return FieldNames::default();
        };
        let column = |name: &str| header.iter().position(|h| h == name);
        let repeat = column("repeat");
        let (Some(id), display, field) = (column("ID"), column("displayName"), column("field"))
        else {
            return FieldNames::default();
        };
        fn cell(row: &[String], col: Option<usize>) -> Option<&str> {
            col.and_then(|c| row.get(c))
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
        }

        let mut names = HashMap::new();
        let mut repeated = Vec::new();
        for row in rows {
            let Some(code) =
                cell(row, Some(id)).and_then(|s| <[u8; 4]>::try_from(s.as_bytes()).ok())
            else {
                continue;
            };
            let name = cell(row, display)
                .and_then(|key| keys.get(key).copied())
                .filter(|name| !name.is_empty())
                .or_else(|| cell(row, field));
            let Some(name) = name else { continue };
            names
                .entry(Rawcode(code))
                .or_insert_with(|| name.to_string());
            let times = cell(row, repeat)
                .and_then(|r| r.parse::<u8>().ok())
                .unwrap_or(0);
            if times > 1 && code[3] == b'1' {
                repeated.push((code, times.min(10), name));
            }
        }
        // Doodads of version 1 have no levels: the variation is the id's
        // last digit, `dvr2` for the second one's `dvr1`, `dvr0` for the
        // tenth's.
        for (code, times, name) in repeated {
            for n in 2..=times {
                let mut id = code;
                id[3] = b'0' + n % 10;
                names
                    .entry(Rawcode(id))
                    .or_insert_with(|| format!("{name}, variation {n}"));
            }
        }
        FieldNames { names }
    }

    pub fn get(&self, field: Rawcode) -> Option<&str> {
        self.names.get(&field).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

#[cfg(test)]
#[path = "names.test.rs"]
mod tests;
