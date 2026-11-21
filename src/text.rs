//! The text form of object data, in BNI (`bni-rs`): what the editor shows
//! and saves. Every file converts to text and back to the same data, but
//! the `end` id each field ends with: the game doesn't read it, so the
//! text leaves it out and saving writes 0. The syntax is in the README:
//!
//! ```ini
//! [w3a]
//! version=2
//!
//! [ACwe]
//! arlv=6
//!
//! [A000]
//! _parent=ANwm
//! aub1[2]="Line one"\n"Line two"
//! Hbz1[3].B=0.25
//! ```
//!
//! A number's type (integer, real, unreal) is the field's, from the game's
//! metadata ([`Metadata`]): `Hbz2` is an unreal, so `0.25` is one. Only a
//! value whose type differs from its field's, or any unreal without
//! metadata, says it: `unreal 0.25`, `real 1.5`, `int 3`.

use bni_rs::ast::{Field, Item, Section, ValueKind};
use parser_rs::{ParseError, Rawcode, Span};

use crate::{FieldMeta, Keys, Kind, Metadata, Mod, Object, ObjectData, Set, Storage, Value};

impl Kind {
    /// What the file holds, in words.
    pub fn title(self) -> &'static str {
        match self {
            Kind::Ability => "abilities",
            Kind::Destructable => "destructables",
            Kind::Doodad => "doodads",
            Kind::Buff => "buffs",
            Kind::Upgrade => "upgrades",
            Kind::Item => "items",
            Kind::Unit => "units",
        }
    }
}

/// The base object's id (`_parent=hfoo`). An object with one is custom.
const PARENT: &str = "_parent";
/// Puts an object in the other table than `_parent` says
/// (`_table=original` / `_table=custom`); real files never need it.
const TABLE: &str = "_table";
/// Starts a set with these flags (version 3).
const FLAGS: &str = "_flags";

/// An id in the text: its four characters when they are letters, digits
/// or `_`, otherwise `0x` and eight hex digits.
pub fn id_text(code: Rawcode) -> String {
    if code
        .0
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
    {
        code.0.iter().map(|&b| b as char).collect()
    } else {
        format!("0x{:08X}", code.to_int())
    }
}

/// The key of a field: its id, then `[level]` and `.column` when not 0
/// (`Hbz1[3].B`).
/// A field's key in an object's text: the field's name in the game's
/// tables when `keys` (the object's, [`Metadata::keys`]) has one and the
/// field is in its own data column (`DataA[3]`, `Cool[1]`, `HP`); else its
/// id, then `[level]` and `.column` when not 0 (`Hbz1[3].B`).
pub fn field_key(m: &Mod, kind: Kind, meta: &Metadata, keys: &Keys) -> String {
    let column = own_column(kind, meta, m.field);
    let name = keys.key(m.field).filter(|_| m.column == column);
    let mut out = match name {
        Some(name) => name.to_string(),
        // An id that reads as one of the keys says it in hex.
        None if keys.field(&id_text(m.field)).is_some() => format!("0x{:08X}", m.field.to_int()),
        None => id_text(m.field),
    };
    if m.level != 0 {
        out += &format!("[{}]", m.level);
    }
    if name.is_none() {
        match m.column {
            0 => {}
            c @ 1..=26 => out += &format!(".{}", (b'A' + (c - 1) as u8) as char),
            c => out += &format!(".{c}"),
        }
    }
    out
}

/// The data column a field of `kind` is in: its own for an ability's data,
/// else none.
fn own_column(kind: Kind, meta: &Metadata, field: Rawcode) -> u32 {
    if kind == Kind::Ability {
        meta.get(field).map_or(0, |f| f.column)
    } else {
        0
    }
}

pub(crate) fn real_text(v: f32) -> String {
    let text = format!("{v:?}");
    if text
        .parse::<f32>()
        .is_ok_and(|back| back.to_bits() == v.to_bits())
    {
        text
    } else {
        // A NaN with a payload: only its bits say which.
        format!("0x{:08X}", v.to_bits())
    }
}

/// A string as BNI can hold it. BNI strings have no escapes and no line
/// breaks, so what they can't hold goes between quoted parts, unquoted:
/// `"Line one"\n"Line two"`. The value always starts with a quoted part.
pub fn string_text(s: &str) -> String {
    let mut out = String::from("\"");
    let mut quoted = true;
    for c in s.chars() {
        let escape = match c {
            '\n' => Some("\\n".to_string()),
            '\r' => Some("\\r".to_string()),
            '\t' => Some("\\t".to_string()),
            '"' => Some("\\u{22}".to_string()),
            c if c.is_control() => Some(format!("\\u{{{:X}}}", c as u32)),
            _ => None,
        };
        match (escape, quoted) {
            (Some(e), true) => {
                out += "\"";
                out += &e;
                quoted = false;
            }
            (Some(e), false) => out += &e,
            (None, false) => {
                out.push('"');
                out.push(c);
                quoted = true;
            }
            (None, true) => out.push(c),
        }
    }
    if quoted {
        out.push('"');
    }
    out
}

/// How a value is written for a field stored as `storage` (`None` when
/// the metadata doesn't say): a plain number when reading it back as the
/// field's type gives the same value, else with its type first.
pub(crate) fn value_text(value: &Value, storage: Option<Storage>) -> String {
    match value {
        Value::Int(v) if matches!(storage, None | Some(Storage::Int | Storage::Str)) => {
            v.to_string()
        }
        Value::Int(v) => format!("int {v}"),
        Value::Real(v) => typed(
            "real",
            real_text(*v),
            matches!(storage, None | Some(Storage::Real | Storage::Str)),
        ),
        Value::Unreal(v) => typed("unreal", real_text(*v), storage == Some(Storage::Unreal)),
        Value::Str(s) => string_text(s),
    }
}

/// A real or unreal: plain when `plain` and it reads back as a real
/// (`1.0`, not `0x7FC00001`), else with its type first.
fn typed(ty: &str, text: String, plain: bool) -> String {
    if plain && !text.starts_with("0x") {
        text
    } else {
        format!("{ty} {text}")
    }
}

impl ObjectData {
    /// The text form. [`from_text`] reads it back into the same data, but
    /// the `end` ids, which it leaves out (0).
    pub fn to_text(&self) -> String {
        self.to_text_named(&Metadata::default())
    }

    /// [`to_text`](Self::to_text), with each field's name and type from
    /// `meta` as a comment after it:
    /// `uhpm=500 // Hit Points Maximum (Base): int`. Comments are only for
    /// reading; [`from_text`] skips them. A number is written plain when
    /// its type is its field's (`Hbz2[1].B=0.25`, an unreal): read it back
    /// with the same `meta`.
    pub fn to_text_named(&self, meta: &Metadata) -> String {
        self.to_text_with(meta, true)
    }

    /// The text form with `meta`: keys by name, numbers plain in their
    /// field's type, and, with `comments`, each field's name and type
    /// after it. An editor that shows those itself (as inlay hints,
    /// [`field_hints`]) leaves them out, so they aren't part of the text
    /// to edit.
    pub fn to_text_with(&self, meta: &Metadata, comments: bool) -> String {
        let ext = self.kind.extension();
        let mut out = format!(
            "// Warcraft III {} (war3map.{ext}). Saving writes the binary file back.\n[{ext}]\nversion={}\n",
            self.kind.title(),
            self.version
        );
        for (original, table) in [(true, &self.originals), (false, &self.customs)] {
            for object in table {
                out.push('\n');
                self.object_text(&mut out, meta, comments, original, object);
            }
        }
        out
    }

    fn object_text(
        &self,
        out: &mut String,
        meta: &Metadata,
        comments: bool,
        original: bool,
        object: &Object,
    ) {
        let custom = !original;
        let shown = if custom || !object.id.is_none() {
            object.id
        } else {
            object.base
        };
        *out += &format!("[{}]\n", id_text(shown));
        // Customs name their base; so does an original with an id of its
        // own, or the base would be lost.
        if custom || !object.id.is_none() {
            *out += &format!("{PARENT}={}\n", id_text(object.base));
        }
        if original && !object.id.is_none() {
            *out += &format!("{TABLE}=original\n");
        }

        let keys = meta.keys(self.kind, object.base);
        for (i, set) in object.sets.iter().enumerate() {
            let implicit = i == 0 && set.flags == 0 && (self.version < 3 || !set.mods.is_empty());
            if !implicit {
                *out += &format!("{FLAGS}={}\n", set.flags);
            }
            for m in &set.mods {
                let storage = meta.get(m.field).and_then(|f| f.storage);
                *out += &format!(
                    "{}={}",
                    field_key(m, self.kind, meta, &keys),
                    value_text(&m.value, storage)
                );
                if let Some(label) = meta.get(m.field).filter(|_| comments).map(label) {
                    *out += &format!(" // {label}");
                }
                out.push('\n');
            }
        }
    }
}

/// Is this BNI the text form of object data: does it start with a
/// `[w3?]` section?
pub fn is_object_text(doc: &bni_rs::Document) -> bool {
    doc.sections
        .first()
        .is_some_and(|s| Kind::from_extension(&s.name.text).is_some())
}

/// What [`from_text`] read: the data, when the text has no errors, and
/// every error, in order; and, errors or not, what it could read of the
/// file's kind and version and of each field, with where it is, for
/// [`check`](crate::check).
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub data: Option<ObjectData>,
    pub errors: Vec<ParseError>,
    /// The file's kind, when its section says.
    pub kind: Option<Kind>,
    pub version: u32,
    /// Every field read without an error, in order.
    pub fields: Vec<FieldAt>,
}

/// A field of the text and where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldAt {
    /// The object it belongs to: the standard one a custom object is
    /// based on, or the standard one itself.
    pub base: Rawcode,
    pub field: Mod,
    /// The key (`Hbz1[3].B`) and the value.
    pub key: Span,
    pub value: Span,
}

/// Reads the text form; a plain number has its field's type in `meta`
/// (an empty one: a whole number is an integer, any other a real). Every
/// error is reported, not just the first; there is data only without
/// errors. Fields end with 0 (`Mod::end`).
pub fn from_text(text: &str, meta: &Metadata) -> Parsed {
    from_bni(&bni_rs::parse(text), meta)
}

/// [`from_text`] for text already parsed as BNI. Its BNI errors count.
pub fn from_bni(doc: &bni_rs::Document, meta: &Metadata) -> Parsed {
    let mut errors = doc.errors.clone();
    for item in &doc.preamble {
        errors.push(error(
            item_span(item),
            "expected the file's [w3?] section first",
        ));
    }
    let Some((head, objects)) = doc.sections.split_first() else {
        errors.push(ParseError::at(
            "expected the file's section first, like [w3u]",
            1,
            1,
        ));
        return Parsed::failed(errors);
    };
    let Some(kind) = Kind::from_extension(&head.name.text) else {
        errors.push(error(
            head.name.span,
            "expected the file's section first: [w3a] [w3b] [w3d] [w3h] [w3q] [w3t] or [w3u]",
        ));
        return Parsed::failed(errors);
    };
    let version = header(head, &mut errors);
    let mut data = ObjectData {
        kind,
        version,
        originals: Vec::new(),
        customs: Vec::new(),
    };
    let mut fields = Vec::new();
    for section in objects {
        if let Some((original, object)) =
            object(section, kind, version, meta, &mut errors, &mut fields)
        {
            if original {
                data.originals.push(object)
            } else {
                data.customs.push(object)
            }
        }
    }
    let data = errors.is_empty().then_some(data);
    Parsed {
        data,
        errors,
        kind: Some(kind),
        version,
        fields,
    }
}

impl Parsed {
    fn failed(errors: Vec<ParseError>) -> Parsed {
        Parsed {
            data: None,
            errors,
            kind: None,
            version: crate::LATEST_VERSION,
            fields: Vec::new(),
        }
    }
}

fn error(span: Span, message: impl Into<String>) -> ParseError {
    let end_col = if span.end_line == span.start_line {
        span.end_col.max(span.start_col + 1)
    } else {
        span.start_col + 1
    };
    ParseError {
        message: message.into(),
        line: span.start_line,
        col: span.start_col,
        end_col,
    }
}

fn item_span(item: &Item) -> Span {
    let start = item.key.as_ref().map_or(item.eq, |k| k.span);
    let end = item.fields.last().map_or(item.eq, |f| f.span);
    Span::new(start.start_line, start.start_col, end.end_line, end.end_col)
}

/// The text of an item's only field, or an error.
fn single<'a>(item: &'a Item, what: &str, errors: &mut Vec<ParseError>) -> Option<&'a Field> {
    match item.fields.as_slice() {
        [field] if !field.parts.is_empty() => Some(field),
        [] | [_] => {
            errors.push(error(item.eq, format!("expected {what} after `=`")));
            None
        }
        [_, extra, ..] => {
            errors.push(error(extra.span, format!("expected only {what}")));
            None
        }
    }
}

/// `version=2`, the only item of the header section.
fn header(section: &Section, errors: &mut Vec<ParseError>) -> u32 {
    let mut version = None;
    let mut given = false;
    for item in &section.items {
        let key = item.key.as_ref();
        if key.is_none_or(|k| k.text.trim() != "version") || given {
            errors.push(error(
                item_span(item),
                "the file's section has only `version=`",
            ));
            continue;
        }
        given = true;
        let Some(field) = single(item, "the format version: 1, 2 or 3", errors) else {
            continue;
        };
        match field.text().trim().parse::<u32>() {
            Ok(v) if crate::VERSIONS.contains(&v) => version = Some(v),
            _ => errors.push(error(field.span, "unknown format version: 1, 2 or 3")),
        }
    }
    version.unwrap_or_else(|| {
        if !given {
            errors.push(error(section.name.span, "missing `version=` (1, 2 or 3)"));
        }
        crate::LATEST_VERSION
    })
}

/// What's said about a field next to it: its name and type,
/// `Cooldown: unreal`.
fn label(f: &FieldMeta) -> String {
    if f.ty.is_empty() {
        f.name.clone()
    } else {
        format!("{}: {}", f.name, f.ty)
    }
}

/// A field's name and type to show after its line, for an editor's inlay
/// hints ([`field_hints`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    /// Where it goes: the line (1-based) and the column just past the
    /// field's value (1-based, in characters, as BNI spans count).
    pub line: usize,
    pub col: usize,
    /// `Cooldown: unreal`.
    pub label: String,
}

/// The hints of the fields on lines `first..=last` (1-based) of a text
/// form: each field's name and type from `meta`, after its value. Only
/// those lines are worked out, so an editor asks for what's in view.
pub fn field_hints(
    doc: &bni_rs::Document,
    meta: &Metadata,
    first: usize,
    last: usize,
) -> Vec<Hint> {
    let mut out = Vec::new();
    if meta.is_empty() {
        return out;
    }
    let Some((head, objects)) = doc.sections.split_first() else {
        return out;
    };
    let Some(kind) = Kind::from_extension(&head.name.text) else {
        return out;
    };
    for section in objects {
        let lines = section.line
            ..=section
                .items
                .iter()
                .map(|i| i.line)
                .max()
                .unwrap_or(section.line);
        if *lines.end() < first || *lines.start() > last {
            continue;
        }
        let named = parse_id(section.name.text.trim());
        let parent = section.items.iter().find_map(|i| {
            let key = i.key.as_ref()?;
            (key.text.trim() == PARENT).then(|| parse_id(i.fields.first()?.text().trim()))?
        });
        let keys = meta.keys(kind, parent.or(named).unwrap_or(Rawcode::NONE));
        for item in section
            .items
            .iter()
            .filter(|i| (first..=last).contains(&i.line))
        {
            let Some(key) = &item.key else { continue };
            let Ok((field, ..)) = parse_key(key.text.trim(), kind, meta, &keys) else {
                continue;
            };
            let Some(f) = meta.get(field) else { continue };
            let end = item.fields.last().map_or(item.eq, |f| f.span);
            out.push(Hint {
                line: end.end_line,
                col: end.end_col,
                label: label(f),
            });
        }
    }
    out
}

/// The field whose key is at `line`, `col` (1-indexed, columns in
/// characters, as BNI spans count), for a hover: [`FieldRef`]. Read from
/// the key alone, so its value may be anything, even broken.
pub fn field_at(
    doc: &bni_rs::Document,
    line: usize,
    col: usize,
    meta: &Metadata,
) -> Option<FieldRef> {
    let (head, objects) = doc.sections.split_first()?;
    let kind = Kind::from_extension(&head.name.text)?;
    for section in objects {
        for item in &section.items {
            let Some(key) = &item.key else { continue };
            let span = key.span;
            if span.start_line != line || col < span.start_col || col > span.end_col {
                continue;
            }
            let named = parse_id(section.name.text.trim());
            let parent = section.items.iter().find_map(|i| {
                let key = i.key.as_ref()?;
                (key.text.trim() == PARENT).then(|| parse_id(i.fields.first()?.text().trim()))?
            });
            let base = parent.or(named).unwrap_or(Rawcode::NONE);
            let (field, ..) =
                parse_key(key.text.trim(), kind, meta, &meta.keys(kind, base)).ok()?;
            return Some(FieldRef {
                kind,
                field,
                key: span,
                object: named.unwrap_or(Rawcode::NONE),
                base,
            });
        }
    }
    None
}

/// A field key in a text form ([`field_at`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldRef {
    pub kind: Kind,
    pub field: Rawcode,
    /// Where the key is.
    pub key: Span,
    /// The object it's in, and the standard one that is based on (itself
    /// for a standard one).
    pub object: Rawcode,
    pub base: Rawcode,
}

/// `hfoo` or `0x6866F6F6`.
fn parse_id(text: &str) -> Option<Rawcode> {
    let bytes = text.as_bytes();
    if bytes.len() == 10 && text.starts_with("0x") {
        return u32::from_str_radix(&text[2..], 16)
            .ok()
            .map(Rawcode::from_int);
    }
    let code: [u8; 4] = bytes.try_into().ok()?;
    code.iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
        .then_some(Rawcode(code))
}

const ID_FORM: &str = "four letters, digits or `_`, or 0x and eight hex digits";

fn object(
    section: &Section,
    kind: Kind,
    version: u32,
    meta: &Metadata,
    errors: &mut Vec<ParseError>,
    fields: &mut Vec<FieldAt>,
) -> Option<(bool, Object)> {
    let before = errors.len();
    let named = parse_id(section.name.text.trim());
    if named.is_none() {
        errors.push(error(
            section.name.span,
            format!("expected an object id: {ID_FORM}"),
        ));
    }

    // Where it goes and what it's based on, wherever those items are.
    let mut parent = None;
    let mut table = None;
    for item in &section.items {
        let Some(key) = &item.key else { continue };
        let name = key.text.trim();
        if name != PARENT && name != TABLE {
            continue;
        }
        let Some(field) = single(item, "a value", errors) else {
            continue;
        };
        let text = field.text();
        if name == PARENT {
            match parse_id(text.trim()) {
                _ if parent.is_some() => errors.push(error(key.span, "`_parent` is already given")),
                Some(id) => parent = Some(id),
                None => errors.push(error(
                    field.span,
                    format!("expected the base object's id: {ID_FORM}"),
                )),
            }
        } else {
            match text.trim() {
                _ if table.is_some() => errors.push(error(key.span, "`_table` is already given")),
                "original" => table = Some(true),
                "custom" => table = Some(false),
                _ => errors.push(error(field.span, "expected `original` or `custom`")),
            }
        }
    }
    let original = table.unwrap_or(parent.is_none());
    let name = named.unwrap_or(Rawcode::NONE);
    let (base, id) = match parent {
        Some(base) => (base, name),
        None => (name, Rawcode::NONE),
    };
    let mut object = Object {
        base,
        id,
        sets: Vec::new(),
    };
    let keys = meta.keys(kind, base);
    for item in &section.items {
        let Some(key) = &item.key else {
            errors.push(error(item.eq, "expected a field id before `=`"));
            continue;
        };
        match key.text.trim() {
            PARENT | TABLE => {}
            FLAGS => {
                if version < 3 {
                    errors.push(error(
                        key.span,
                        format!("sets need format version 3; this is version {version}"),
                    ));
                    continue;
                }
                let Some(field) = single(item, "the set's flags", errors) else {
                    continue;
                };
                match field.text().trim().parse() {
                    Ok(flags) => object.sets.push(Set {
                        flags,
                        mods: Vec::new(),
                    }),
                    Err(_) => errors.push(error(
                        field.span,
                        "expected the set's flags, a whole number",
                    )),
                }
            }
            _ => {
                if let Some((m, value)) = field(item, key, kind, version, meta, &keys, errors) {
                    fields.push(FieldAt {
                        base,
                        field: m.clone(),
                        key: key.span,
                        value,
                    });
                    if object.sets.is_empty() {
                        object.sets.push(Set {
                            flags: 0,
                            mods: Vec::new(),
                        });
                    }
                    object.sets.last_mut().expect("just made sure").mods.push(m);
                }
            }
        }
    }
    if version < 3 && object.sets.is_empty() {
        object.sets.push(Set {
            flags: 0,
            mods: Vec::new(),
        });
    }
    (errors.len() == before).then_some((original, object))
}

/// `Hbz1[3].B` → the field, its level and its column.
/// `DataA[3]` or `Hbz1[3].A` → the field, its level and its column (a
/// key from the game's tables is in its own column) and whether a level
/// or column is given.
fn parse_key(
    text: &str,
    kind: Kind,
    meta: &Metadata,
    keys: &Keys,
) -> Result<(Rawcode, u32, u32, bool), String> {
    let name_len = text.find(['[', '.']).unwrap_or(text.len());
    let named = keys.field(&text[..name_len]);
    let (field, mut rest) = match named {
        Some(field) => (field, &text[name_len..]),
        None => {
            let id_len = if text.starts_with("0x") { 10 } else { 4 };
            let (id, rest) = text.split_at(id_len.min(text.len()));
            let field =
                parse_id(id).ok_or(format!("expected a field: its name or id ({ID_FORM})"))?;
            (field, rest)
        }
    };
    let mut addressed = false;
    let mut level = 0;
    if let Some(after) = rest.strip_prefix('[') {
        let (n, after) = after
            .split_once(']')
            .ok_or("expected `]` after the level")?;
        level = n
            .trim()
            .parse()
            .map_err(|_| "expected the level, a whole number")?;
        rest = after;
        addressed = true;
    }
    let mut column = if named.is_some() {
        own_column(kind, meta, field)
    } else {
        0
    };
    if let Some(c) = rest.strip_prefix('.') {
        column = match c.as_bytes() {
            [c @ b'A'..=b'Z'] => (c - b'A' + 1) as u32,
            _ => c
                .parse()
                .map_err(|_| "expected the data column: a letter A-Z or a number")?,
        };
        rest = "";
        addressed = true;
    }
    if !rest.is_empty() {
        return Err("expected `[level]` or `.column` after the field".into());
    }
    Ok((field, level, column, addressed))
}

fn field(
    item: &Item,
    key: &bni_rs::ast::Name,
    kind: Kind,
    version: u32,
    meta: &Metadata,
    keys: &Keys,
    errors: &mut Vec<ParseError>,
) -> Option<(Mod, Span)> {
    let (field, level, column, addressed) = match parse_key(key.text.trim(), kind, meta, keys) {
        Ok(parts) => parts,
        Err(message) => {
            errors.push(error(key.span, message));
            return None;
        }
    };
    if addressed && !kind.has_levels(version) {
        let what = if kind == Kind::Doodad {
            "version 1 doodads"
        } else {
            kind.title()
        };
        errors.push(error(
            key.span,
            format!("fields of {what} have no level or data column"),
        ));
        return None;
    }
    let value_field = match item.fields.as_slice() {
        [] => {
            errors.push(error(item.eq, "expected a value after `=`"));
            return None;
        }
        [v] => v,
        [_, extra, ..] => {
            errors.push(error(
                extra.span,
                "expected only the value; a string with a comma is quoted",
            ));
            return None;
        }
    };
    let storage = meta.get(field).and_then(|f| f.storage);
    let value = value(value_field, storage)
        .map_err(|e| errors.push(e))
        .ok()?;
    let m = Mod {
        field,
        level,
        column,
        value,
        // The game doesn't read it.
        end: Rawcode::NONE,
    };
    Some((m, value_field.span))
}

/// A value for a field stored as `storage`: a plain number takes the
/// field's type; one that doesn't fit it (`1.5` for an integer field) and
/// any without metadata is an integer when whole, else a real.
fn value(field: &Field, storage: Option<Storage>) -> Result<Value, ParseError> {
    if field.parts.is_empty() {
        return Err(error(field.span, "expected a value"));
    }
    if field
        .parts
        .iter()
        .any(|p| matches!(p.kind, ValueKind::Quoted { .. }))
    {
        return string(field).map(Value::Str);
    }
    let text = field.text();
    let mut words = text.split_whitespace();
    let (first, second) = (words.next().unwrap_or(""), words.next());
    if words.next().is_some() {
        return Err(error(field.span, VALUE_FORM));
    }
    if let ("int", Some(n)) = (first, second) {
        return n
            .parse::<i32>()
            .map(Value::Int)
            .map_err(|_| error(field.span, "expected a whole number after `int`"));
    }
    let typed = match (first, second) {
        ("real", Some(n)) => Some((Value::Real as fn(f32) -> Value, n)),
        ("unreal", Some(n)) => Some((Value::Unreal as fn(f32) -> Value, n)),
        (_, Some(_)) => return Err(error(field.span, VALUE_FORM)),
        _ => None,
    };
    if let Some((make, n)) = typed {
        return real(n).map(make).ok_or_else(|| {
            error(
                field.span,
                "expected a real number, or 0x and its eight hex digits",
            )
        });
    }
    let number = first
        .parse::<f32>()
        .map_err(|_| error(field.span, VALUE_FORM))?;
    Ok(match (storage, first.parse::<i32>()) {
        (Some(Storage::Real), _) => Value::Real(number),
        (Some(Storage::Unreal), _) => Value::Unreal(number),
        (_, Ok(v)) => Value::Int(v),
        (_, Err(_)) => Value::Real(number),
    })
}

const VALUE_FORM: &str = "expected a value: a number, or a \"string\" (`int`, `real` or `unreal` before a number gives its type)";

/// Quoted parts as written; unquoted parts between them are escapes.
fn string(field: &Field) -> Result<String, ParseError> {
    let mut out = String::new();
    for part in &field.parts {
        match part.kind {
            ValueKind::Quoted { closed: false } => {
                return Err(error(part.span, "string isn't closed: `\"` missing"));
            }
            ValueKind::Quoted { closed: true } => out += &part.text,
            _ => unescape(&part.text, &mut out).map_err(|m| error(part.span, m))?,
        }
    }
    Ok(out)
}

fn unescape(text: &str, out: &mut String) -> Result<(), String> {
    const FORM: &str = "between quoted parts only escapes: \\n \\r \\t or \\u{...}";
    let mut rest = text;
    while !rest.is_empty() {
        let after = rest.strip_prefix('\\').ok_or(FORM)?;
        let (c, next) = match after.as_bytes().first() {
            Some(b'n') => ('\n', &after[1..]),
            Some(b'r') => ('\r', &after[1..]),
            Some(b't') => ('\t', &after[1..]),
            Some(b'u') => {
                let (hex, next) = after[1..]
                    .strip_prefix('{')
                    .and_then(|h| h.split_once('}'))
                    .ok_or(FORM)?;
                let c = u32::from_str_radix(hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .filter(|&c| c != '\0');
                (
                    c.ok_or("\\u{...} takes a character's hex code, not 0")?,
                    next,
                )
            }
            _ => return Err(FORM.into()),
        };
        out.push(c);
        rest = next;
    }
    Ok(())
}

/// A real written as a number, or as `0x` and its bits.
fn real(word: &str) -> Option<f32> {
    match word.strip_prefix("0x") {
        Some(hex) if hex.len() == 8 => u32::from_str_radix(hex, 16).ok().map(f32::from_bits),
        Some(_) => None,
        None => word.parse().ok(),
    }
}

#[cfg(test)]
#[path = "text.test.rs"]
mod tests;
