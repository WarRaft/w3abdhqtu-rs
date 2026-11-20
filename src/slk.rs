//! Just enough SYLK (`.slk`) to read the game's metadata tables: the
//! cells' values, by row and column. Formatting records are skipped.
//!
//! A line is a record: its type, then `;`-separated fields, each a letter
//! and a value (`C;X2;Y5;K"Name"`). `;;` is a `;` inside a value. A cell
//! (`C`) and a format record (`F`) move the cursor with `X` (column) and
//! `Y` (row); a field they leave out keeps the previous one. `K` is the
//! value: a quoted string, or a number or anything else as written.

/// The table's rows, each its cells by column, from the first; missing
/// cells are empty.
pub(crate) fn table(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let (mut x, mut y) = (1usize, 1usize);
    for line in text.lines() {
        let fields = split(line);
        let Some((kind, fields)) = fields.split_first() else {
            continue;
        };
        if kind != "C" && kind != "F" {
            continue;
        }
        let mut value = None;
        for field in fields {
            let mut chars = field.chars();
            let Some(letter) = chars.next() else { continue };
            let rest = chars.as_str();
            match letter {
                'X' => x = rest.parse().unwrap_or(x),
                'Y' => y = rest.parse().unwrap_or(y),
                'K' if kind == "C" => value = Some(unquote(rest)),
                _ => {}
            }
        }
        let (Some(value), Some(row), Some(col)) = (value, y.checked_sub(1), x.checked_sub(1))
        else {
            continue;
        };
        if rows.len() <= row {
            rows.resize(row + 1, Vec::new());
        }
        let cells = &mut rows[row];
        if cells.len() <= col {
            cells.resize(col + 1, String::new());
        }
        cells[col] = value;
    }
    rows
}

/// A record's fields; `;;` is a `;` inside one.
fn split(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut chars = line.trim_end_matches('\r').chars().peekable();
    while let Some(c) = chars.next() {
        if c != ';' {
            fields.last_mut().expect("never empty").push(c);
        } else if chars.peek() == Some(&';') {
            chars.next();
            fields.last_mut().expect("never empty").push(';');
        } else {
            fields.push(String::new());
        }
    }
    fields
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .map(|v| v.strip_suffix('"').unwrap_or(v))
        .unwrap_or(value)
        .to_string()
}

#[cfg(test)]
#[path = "slk.test.rs"]
mod tests;
