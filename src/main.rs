//! `w3abdhqtu-rs`: checks object-data files, prints their text form, and
//! builds them back from it. See `usage`.

use std::process::ExitCode;

use w3abdhqtu_rs::{FieldNames, Kind, STRINGS_FILES, from_text, parse};

fn usage() {
    eprintln!(
        "usage: w3abdhqtu-rs <file.w3?>...           check: read, write back, and through the text form"
    );
    eprintln!(
        "       w3abdhqtu-rs --text <file.w3?> [<game>] print the text form (BNI); with the folder of"
    );
    eprintln!(
        "                                            the game's files (Units/UnitMetaData.slk, ...),"
    );
    eprintln!("                                            fields get their names as comments");
    eprintln!("       w3abdhqtu-rs --build <text> <out>    write the binary file from a text form");
    eprintln!();
    eprintln!("  .w3? is any of .w3a .w3b .w3d .w3h .w3q .w3t .w3u");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["-h" | "--help"] => {
            usage();
            return ExitCode::FAILURE;
        }
        ["--text", path] => read(path).map(|data| print!("{}", data.to_text())),
        ["--text", path, game] => {
            read(path).map(|data| print!("{}", data.to_text_named(&names(game, data.kind))))
        }
        ["--build", text, out] => build(text, out),
        paths if !paths.iter().any(|p| p.starts_with('-')) => {
            let failed = paths
                .iter()
                .filter(|p| check(p).map_err(|err| eprintln!("{p}: {err}")).is_err())
                .count();
            return if failed > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            };
        }
        _ => {
            usage();
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn read(path: &str) -> Result<w3abdhqtu_rs::ObjectData, String> {
    let kind = Kind::from_path(path).ok_or(format!(
        "{path}: not an object-data file (.w3a .w3b .w3d .w3h .w3q .w3t .w3u)"
    ))?;
    let bytes = std::fs::read(path).map_err(|err| format!("{path}: {err}"))?;
    parse(&bytes, kind).map_err(|err| format!("{path}: {err}"))
}

/// Field names from the game's files extracted to the folder `game`; a
/// file is looked up in any case. Missing files mean fewer names.
fn names(game: &str, kind: Kind) -> FieldNames {
    let read = |name: &str| {
        let mut path = std::path::PathBuf::from(game);
        for part in name.split('\\') {
            let entry = std::fs::read_dir(&path).ok()?.flatten().find(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(part))
            })?;
            path = entry.path();
        }
        let bytes = std::fs::read(&path).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    };
    let Some(metadata) = read(kind.metadata_file()) else {
        eprintln!("{game}: no {}", kind.metadata_file());
        return FieldNames::default();
    };
    let strings: Vec<String> = STRINGS_FILES.iter().filter_map(|f| read(f)).collect();
    let strings: Vec<&str> = strings.iter().map(String::as_str).collect();
    FieldNames::from_metadata(&metadata, &strings)
}

fn build(text_path: &str, out: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(text_path).map_err(|err| format!("{text_path}: {err}"))?;
    let parsed = from_text(&text);
    for e in &parsed.errors {
        eprintln!("{text_path}:{}:{}: {}", e.line, e.col, e.message);
    }
    let data = parsed
        .data
        .ok_or(format!("{text_path}: {} errors", parsed.errors.len()))?;
    if Kind::from_path(out).is_some_and(|k| k != data.kind) {
        return Err(format!(
            "{out}: the text is .{} data",
            data.kind.extension()
        ));
    }
    let bytes = data.to_bytes().map_err(|err| format!("{out}: {err}"))?;
    std::fs::write(out, bytes).map_err(|err| format!("{out}: {err}"))
}

fn check(path: &str) -> Result<(), String> {
    let kind = Kind::from_path(path)
        .ok_or("not an object-data file (.w3a .w3b .w3d .w3h .w3q .w3t .w3u)")?;
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    let data = parse(&bytes, kind).map_err(|err| err.to_string())?;
    let fields: usize = data.objects().map(|o| o.mods().count()).sum();
    println!(
        "{path}: version {}, {} changed, {} custom, {fields} fields",
        data.version,
        data.originals.len(),
        data.customs.len()
    );
    let written = data
        .to_bytes()
        .map_err(|err| format!("can't write it back: {err}"))?;
    if written != bytes {
        let at = written
            .iter()
            .zip(&bytes)
            .position(|(a, b)| a != b)
            .unwrap_or(written.len().min(bytes.len()));
        return Err(format!(
            "written back differs from the original at byte 0x{at:X}"
        ));
    }
    let back = from_text(&data.to_text());
    if let Some(e) = back.errors.first() {
        return Err(format!(
            "its text form has errors, first at line {}: {}",
            e.line, e.message
        ));
    }
    if back.data.as_ref() != Some(&data) {
        return Err("differs after going through the text form".into());
    }
    Ok(())
}
