//! Checks a text form's fields against the game's metadata
//! ([`Metadata`]): what the format can hold but the game reads wrong or
//! World Editor wouldn't write. Each check is a lint rule
//! (`parser_rs::rule::OBJECT_*`), so a project can turn it off or change
//! its level. Without metadata nothing is checked.

use parser_rs::{Diagnostic, Edit, Fix, Rawcode, rule};

use crate::metadata::{FieldMeta, Storage};
use crate::text::{FieldAt, field_key, real_text, value_text};
use crate::{Kind, Metadata, Mod, Parsed, Value, id_text};

/// What's wrong with the fields of `parsed` (what [`crate::from_bni`]
/// read, errors or not) by `meta`, the metadata of its kind.
pub fn check(parsed: &Parsed, meta: &Metadata) -> Vec<Diagnostic> {
    let Some(kind) = parsed.kind else {
        return Vec::new();
    };
    if meta.is_empty() {
        return Vec::new();
    }
    let levels = kind.has_levels(parsed.version);
    let mut out = Vec::new();
    for at in &parsed.fields {
        let m = &at.field;
        let Some(field) = meta.get(m.field) else {
            out.push(
                Diagnostic::warning(
                    rule::OBJECT_UNKNOWN_FIELD,
                    format!(
                        "`{}` isn't a field of {} in this game's metadata",
                        id_text(m.field),
                        kind.title()
                    ),
                    at.key.start_line,
                )
                .at(at.key),
            );
            continue;
        };
        let what = format!("`{}` ({})", id_text(m.field), field.name);
        if let Some(d) = value_type(at, field, &what) {
            out.push(d);
        } else if let Some(d) = range(at, field, &what) {
            out.push(d);
        }
        if levels {
            out.extend(level(kind, meta, at, field, &what));
        }
        out.extend(owner(kind, meta, at, field, &what));
    }
    out
}

/// The value is stored as the field's type.
fn value_type(at: &FieldAt, field: &FieldMeta, what: &str) -> Option<Diagnostic> {
    let want = field.storage?;
    let have = Storage::of_value(&at.field.value);
    if have == want {
        return None;
    }
    let mut d = Diagnostic::error(
        rule::OBJECT_VALUE_TYPE,
        format!(
            "{what} is {}, not {}: the game would misread it",
            want.describe(),
            have.describe()
        ),
        at.value.start_line,
    )
    .at(at.value);
    if let Some(value) = convert(&at.field.value, want) {
        let text = value_text(&value, Some(want));
        d = d.with_fix(Fix {
            title: format!("Write {text}"),
            edits: vec![Edit {
                span: at.value,
                text,
            }],
        });
    }
    Some(d)
}

/// The same value stored as `want`, when it says the same thing.
fn convert(value: &Value, want: Storage) -> Option<Value> {
    let number = match value {
        Value::Int(v) => *v as f64,
        Value::Real(v) | Value::Unreal(v) => *v as f64,
        Value::Str(s) => s.trim().parse::<f64>().ok()?,
    };
    match want {
        Storage::Int => (number.fract() == 0.0 && number.abs() <= i32::MAX as f64)
            .then_some(Value::Int(number as i32)),
        Storage::Real => Some(Value::Real(number as f32)),
        Storage::Unreal => Some(Value::Unreal(number as f32)),
        Storage::Str => Some(Value::Str(match value {
            Value::Int(v) => v.to_string(),
            Value::Real(v) | Value::Unreal(v) => real_text(*v),
            Value::Str(s) => s.clone(),
        })),
    }
}

/// A number within World Editor's limits; a flag 0 or 1.
fn range(at: &FieldAt, field: &FieldMeta, what: &str) -> Option<Diagnostic> {
    let number = match at.field.value {
        Value::Int(v) => v as f64,
        Value::Real(v) | Value::Unreal(v) => v as f64,
        Value::Str(_) => return None,
    };
    let message = if field.ty == "bool" {
        (number != 0.0 && number != 1.0).then(|| format!("{what} is a flag: 0 or 1"))
    } else if let Some(min) = field.min.filter(|&min| number < min) {
        Some(format!(
            "{what} is below World Editor's minimum, {}",
            limit(min)
        ))
    } else {
        field
            .max
            .filter(|&max| number > max)
            .map(|max| format!("{what} is above World Editor's maximum, {}", limit(max)))
    }?;
    Some(Diagnostic::warning(rule::OBJECT_VALUE_RANGE, message, at.value.start_line).at(at.value))
}

fn limit(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        v.to_string()
    }
}

/// The level and data column fit the field. Only for kinds whose fields
/// have them (`Kind::has_levels`).
fn level(
    kind: Kind,
    meta: &Metadata,
    at: &FieldAt,
    field: &FieldMeta,
    what: &str,
) -> Option<Diagnostic> {
    let m = &at.field;
    // Only abilities have data columns.
    let column = if kind == Kind::Ability {
        field.column
    } else {
        0
    };
    let message = if !field.levels && m.level != 0 {
        format!("{what} has no levels")
    } else if field.levels && m.level == 0 {
        format!("{what} has a value per level; level 0 isn't one")
    } else if m.column != column {
        match column {
            0 => format!("{what} has no data column"),
            c => format!("{what} is in data column {}", column_name(c)),
        }
    } else {
        return None;
    };
    let fixed = Mod {
        level: match (field.levels, m.level) {
            (false, _) => 0,
            (true, 0) => 1,
            (true, l) => l,
        },
        column,
        ..m.clone()
    };
    // As the text writes it: by its name where it has one.
    let key = field_key(&fixed, kind, meta, &meta.keys(kind, at.base));
    Some(
        Diagnostic::warning(rule::OBJECT_FIELD_LEVEL, message, at.key.start_line)
            .at(at.key)
            .with_fix(Fix {
                title: format!("Write {key}"),
                edits: vec![Edit {
                    span: at.key,
                    text: key,
                }],
            }),
    )
}

fn column_name(c: u32) -> String {
    match c {
        1..=26 => ((b'A' + (c - 1) as u8) as char).to_string(),
        c => c.to_string(),
    }
}

/// The field belongs to the object: an ability's data to that ability,
/// a units-only field not to an item.
fn owner(
    kind: Kind,
    meta: &Metadata,
    at: &FieldAt,
    field: &FieldMeta,
    what: &str,
) -> Option<Diagnostic> {
    let base = at.base;
    // An ability has the data fields of its code (`AIsz` has `Aspo`'s).
    let code = meta.code(base);
    let named = |ids: &[Rawcode]| ids.contains(&base) || ids.contains(&code);
    let list = |ids: &[Rawcode]| {
        ids.iter()
            .map(|&id| id_text(id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let message = if kind == Kind::Item && field.items == Some(false) {
        format!("{what} is a field of units, not items")
    } else if !field.specific.is_empty() && !named(&field.specific) {
        format!(
            "{what} is a field of {}, not of {}",
            list(&field.specific),
            id_text(base)
        )
    } else if named(&field.not_specific) {
        format!("{what} isn't a field of {}", id_text(base))
    } else {
        return None;
    };
    Some(Diagnostic::warning(rule::OBJECT_FIELD_OWNER, message, at.key.start_line).at(at.key))
}

#[cfg(test)]
#[path = "check.test.rs"]
mod tests;
