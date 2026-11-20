//! `w3abdhqtu-rs <file>...`: reads each object-data file, prints what it
//! holds, and checks that writing it back gives the same bytes.

use std::process::ExitCode;

use w3abdhqtu_rs::{Kind, Value, parse};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if paths.is_empty() {
        eprintln!("usage: w3abdhqtu-rs [-v] <file.w3a|w3b|w3d|w3h|w3q|w3t|w3u>...");
        eprintln!("  prints a summary of each file (-v: every field) and checks the round trip");
        return ExitCode::FAILURE;
    }

    let mut failed = 0;
    for path in paths {
        if let Err(err) = run(path, verbose) {
            eprintln!("{path}: {err}");
            failed += 1;
        }
    }
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run(path: &str, verbose: bool) -> Result<(), String> {
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
    if verbose {
        for object in data.objects() {
            match object.id.is_none() {
                true => println!("{}", object.base),
                false => println!("{} : {}", object.id, object.base),
            }
            for (i, set) in object.sets.iter().enumerate() {
                if object.sets.len() > 1 || set.flags != 0 {
                    println!("  set {i}, flags {}", set.flags);
                }
                for m in &set.mods {
                    let value = match &m.value {
                        Value::Int(v) => v.to_string(),
                        Value::Real(v) => format!("{v:?}"),
                        Value::Unreal(v) => format!("{v:?} (unreal)"),
                        Value::Str(s) => format!("{s:?}"),
                    };
                    match kind.has_levels(data.version) {
                        true => println!("  {} [{}, {}] = {value}", m.field, m.level, m.column),
                        false => println!("  {} = {value}", m.field),
                    }
                }
            }
        }
    }
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
    Ok(())
}
