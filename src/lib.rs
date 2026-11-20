//! Reader and writer for Warcraft III object data: the `war3map.w3?`
//! files where a map keeps its changes to the game's abilities (`.w3a`),
//! destructables (`.w3b`), doodads (`.w3d`), buffs (`.w3h`), upgrades
//! (`.w3q`), items (`.w3t`) and units (`.w3u`).
//!
//! ```no_run
//! use w3abdhqtu_rs::{parse, Kind};
//!
//! let bytes = std::fs::read("war3map.w3u").unwrap();
//! let data = parse(&bytes, Kind::Unit).unwrap();
//! for object in &data.customs {
//!     println!("{} from {}: {} fields", object.id, object.base, object.mods().count());
//! }
//! assert_eq!(data.to_bytes().unwrap(), bytes);
//! ```
//!
//! Reading and writing are exact: `to_bytes` gives back the bytes `parse`
//! read. The text form (`to_text`, `from_text`) is BNI, and exact too:
//! `from_text(to_text(data))` is `data`. Both formats are described in the
//! README.

mod data;
mod kind;
mod read;
mod text;
mod write;

pub use data::{Mod, Object, ObjectData, Set, Value};
pub use kind::Kind;
pub use parser_rs::{BinError, BinResult, Rawcode};
pub use read::parse;
pub use text::{Parsed, from_bni, from_text, id_text, is_object_text, string_text};

/// The format versions this crate reads and writes.
pub const VERSIONS: std::ops::RangeInclusive<u32> = 1..=3;
/// The version the current game writes.
pub const LATEST_VERSION: u32 = 3;

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
