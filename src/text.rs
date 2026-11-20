//! The text form of object data, in BNI (`bni-rs`): what the editor shows
//! and saves. Every file converts to text and back to the same bytes, so
//! nothing is lost by editing it as text. The syntax is in the README:
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
//! Hbz1[3].B=unreal 0.25,end A000
//! ```

use bni_rs::ast::{Field, Item, Section, ValueKind};
use parser_rs::{ParseError, Rawcode, Span};

use crate::{FieldNames, Kind, Mod, Object, ObjectData, Set, Value};

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

/// The id an object's fields end with unless the text says otherwise
/// (`,end ...`): what the World Editor writes most of the time.
fn default_end(original: bool, object: &Object) -> Rawcode {
    if original { object.base } else { Rawcode::NONE }
}

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
fn key_text(m: &Mod) -> String {
    let mut out = id_text(m.field);
    if m.level != 0 {
        out += &format!("[{}]", m.level);
    }
    match m.column {
        0 => {}
        c @ 1..=26 => out += &format!(".{}", (b'A' + (c - 1) as u8) as char),
        c => out += &format!(".{c}"),
    }
    out
}

fn real_text(v: f32) -> String {
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

fn value_text(value: &Value) -> String {
    match value {
        Value::Int(v) => v.to_string(),
        Value::Real(v) => {
            let text = real_text(*v);
            if text.starts_with("0x") {
                format!("real {text}")
            } else {
                text
            }
        }
        Value::Unreal(v) => format!("unreal {}", real_text(*v)),
        Value::Str(s) => string_text(s),
    }
}

impl ObjectData {
    /// The text form. [`from_text`] reads it back into the same data.
    pub fn to_text(&self) -> String {
        self.to_text_named(&FieldNames::default())
    }

    /// [`to_text`](Self::to_text), with each field's name from `names` as
    /// a comment after it: `uhpm=500 // Hit Points Maximum (Base)`.
    /// Comments are only for reading; [`from_text`] skips them.
    pub fn to_text_named(&self, names: &FieldNames) -> String {
        let ext = self.kind.extension();
        let mut out = format!(
            "// Warcraft III {} (war3map.{ext}). Saving writes the binary file back.\n[{ext}]\nversion={}\n",
            self.kind.title(),
            self.version
        );
        for (original, table) in [(true, &self.originals), (false, &self.customs)] {
            for object in table {
                out.push('\n');
                self.object_text(&mut out, names, original, object);
            }
        }
        out
    }

    fn object_text(&self, out: &mut String, names: &FieldNames, original: bool, object: &Object) {
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

        let end = default_end(original, object);
        for (i, set) in object.sets.iter().enumerate() {
            let implicit = i == 0 && set.flags == 0 && (self.version < 3 || !set.mods.is_empty());
            if !implicit {
                *out += &format!("{FLAGS}={}\n", set.flags);
            }
            for m in &set.mods {
                *out += &format!("{}={}", key_text(m), value_text(&m.value));
                if m.end != end {
                    *out += &format!(",end {}", id_text(m.end));
                }
                if let Some(name) = names.get(m.field) {
                    *out += &format!(" // {name}");
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
/// every error, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub data: Option<ObjectData>,
    pub errors: Vec<ParseError>,
}

/// Reads the text form. Every error is reported, not just the first;
/// there is data only without errors.
pub fn from_text(text: &str) -> Parsed {
    from_bni(&bni_rs::parse(text))
}

/// [`from_text`] for text already parsed as BNI. Its BNI errors count.
pub fn from_bni(doc: &bni_rs::Document) -> Parsed {
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
        return Parsed { data: None, errors };
    };
    let Some(kind) = Kind::from_extension(&head.name.text) else {
        errors.push(error(
            head.name.span,
            "expected the file's section first: [w3a] [w3b] [w3d] [w3h] [w3q] [w3t] or [w3u]",
        ));
        return Parsed { data: None, errors };
    };
    let version = header(head, &mut errors);
    let mut data = ObjectData {
        kind,
        version,
        originals: Vec::new(),
        customs: Vec::new(),
    };
    for section in objects {
        if let Some((original, object)) = object(section, kind, version, &mut errors) {
            if original {
                data.originals.push(object)
            } else {
                data.customs.push(object)
            }
        }
    }
    let data = errors.is_empty().then_some(data);
    Parsed { data, errors }
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
    errors: &mut Vec<ParseError>,
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
    let end = default_end(original, &object);

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
                if let Some(m) = field(item, key, kind, version, end, errors) {
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
fn parse_key(text: &str) -> Result<(Rawcode, u32, u32, bool), String> {
    let id_len = if text.starts_with("0x") { 10 } else { 4 };
    let (id, mut rest) = text.split_at(id_len.min(text.len()));
    let field = parse_id(id).ok_or(format!("expected a field id: {ID_FORM}"))?;
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
    let mut column = 0;
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
        return Err("expected `[level]` or `.column` after the field id".into());
    }
    Ok((field, level, column, addressed))
}

fn field(
    item: &Item,
    key: &bni_rs::ast::Name,
    kind: Kind,
    version: u32,
    end: Rawcode,
    errors: &mut Vec<ParseError>,
) -> Option<Mod> {
    let (field, level, column, addressed) = match parse_key(key.text.trim()) {
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
    let (value_field, end_field) = match item.fields.as_slice() {
        [] => {
            errors.push(error(item.eq, "expected a value after `=`"));
            return None;
        }
        [v] => (v, None),
        [v, e] => (v, Some(e)),
        [_, _, extra, ..] => {
            errors.push(error(
                extra.span,
                "expected the value, and maybe `end` and an id",
            ));
            return None;
        }
    };
    let value = value(value_field).map_err(|e| errors.push(e)).ok()?;
    let end = match end_field {
        None => end,
        Some(f) => {
            let text = f.text();
            match text
                .trim()
                .strip_prefix("end")
                .map(str::trim)
                .and_then(parse_id)
            {
                Some(id) => id,
                None => {
                    errors.push(error(
                        f.span,
                        format!("expected `end` and an id: {ID_FORM}"),
                    ));
                    return None;
                }
            }
        }
    };
    Some(Mod {
        field,
        level,
        column,
        value,
        end,
    })
}

fn value(field: &Field) -> Result<Value, ParseError> {
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
    if let Ok(v) = first.parse::<i32>() {
        return Ok(Value::Int(v));
    }
    first
        .parse::<f32>()
        .map(Value::Real)
        .map_err(|_| error(field.span, VALUE_FORM))
}

const VALUE_FORM: &str = "expected a value: a number, `unreal` and a number, or a \"string\"";

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
