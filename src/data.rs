use parser_rs::Rawcode;

use crate::Kind;

/// A whole object-data file: the changes a map makes to the game's
/// standard objects, and the custom objects it adds.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectData {
    pub kind: Kind,
    /// Format version: 1 and 2 before Reforged, 3 since 1.33 (adds
    /// [`Set`]s).
    pub version: u32,
    /// Standard objects the map changes; their [`Object::id`] is
    /// [`Rawcode::NONE`].
    pub originals: Vec<Object>,
    /// Objects the map adds, each copied from a standard one.
    pub customs: Vec<Object>,
}

/// One object and the fields it changes.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// The standard object it starts from (`hfoo`).
    pub base: Rawcode,
    /// The object's own id (`h000`), or [`Rawcode::NONE`] for a changed
    /// standard object.
    pub id: Rawcode,
    /// Before version 3 there is always exactly one set.
    pub sets: Vec<Set>,
}

/// A group of field changes. Version 3 can keep several per object, told
/// apart by `flags` (which game modes they apply to); older versions have
/// one, with `flags` 0.
#[derive(Debug, Clone, PartialEq)]
pub struct Set {
    pub flags: u32,
    pub mods: Vec<Mod>,
}

/// One changed field.
#[derive(Debug, Clone, PartialEq)]
pub struct Mod {
    /// The field id from the type's metadata SLK (`unam`, `aher`).
    pub field: Rawcode,
    /// The level (doodads: the variation) when [`Kind::has_levels`],
    /// otherwise 0 and not stored.
    pub level: u32,
    /// The data column (`Data A` = 1, ...) when [`Kind::has_levels`],
    /// otherwise 0 and not stored.
    pub column: u32,
    pub value: Value,
    /// The id written after the value. The editor writes the object's id
    /// or zero; the game doesn't check it. Kept so files round-trip.
    pub end: Rawcode,
}

/// A field's value. The type tag is stored in the file; which one a field
/// takes comes from its metadata. Reals compare by their bits, so a NaN
/// equals itself and `0.0` doesn't equal `-0.0`: equal values write equal
/// bytes.
#[derive(Debug, Clone)]
pub enum Value {
    /// Tag 0: integers, booleans, enums and ids stored as integers.
    Int(i32),
    /// Tag 1.
    Real(f32),
    /// Tag 2: a real limited to 0..1.
    Unreal(f32),
    /// Tag 3: text, paths, lists, `TRIGSTR_` references.
    Str(String),
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Real(a), Value::Real(b)) | (Value::Unreal(a), Value::Unreal(b)) => {
                a.to_bits() == b.to_bits()
            }
            (Value::Str(a), Value::Str(b)) => a == b,
            _ => false,
        }
    }
}

impl Value {
    /// The type tag stored in the file.
    pub fn tag(&self) -> u32 {
        match self {
            Value::Int(_) => 0,
            Value::Real(_) => 1,
            Value::Unreal(_) => 2,
            Value::Str(_) => 3,
        }
    }
}

impl ObjectData {
    /// Sets every field's `end` id to 0, as the text form reads it: the
    /// game doesn't read it.
    pub fn clear_ends(&mut self) {
        for object in self.originals.iter_mut().chain(&mut self.customs) {
            for set in &mut object.sets {
                for m in &mut set.mods {
                    m.end = Rawcode::NONE;
                }
            }
        }
    }

    /// An empty file of the latest version.
    pub fn new(kind: Kind) -> Self {
        ObjectData {
            kind,
            version: crate::LATEST_VERSION,
            originals: Vec::new(),
            customs: Vec::new(),
        }
    }

    /// Every object: the changed standard ones, then the custom ones.
    pub fn objects(&self) -> impl Iterator<Item = &Object> {
        self.originals.iter().chain(&self.customs)
    }
}

impl Object {
    /// Every changed field, across all sets.
    pub fn mods(&self) -> impl Iterator<Item = &Mod> {
        self.sets.iter().flat_map(|set| &set.mods)
    }
}
