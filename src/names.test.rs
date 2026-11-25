use super::*;

/// An SLK table of `rows`, the first being the header.
fn slk(rows: &[&[&str]]) -> String {
    let mut out = String::from("ID;PWXL;N;E\n");
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if !cell.is_empty() {
                out += &format!("C;X{};Y{};K\"{cell}\"\n", x + 1, y + 1);
            }
        }
    }
    out + "E\n"
}

fn files(name: &str) -> Option<String> {
    Some(match name {
        "Units\\UnitData.slk" => slk(&[&["unitID", "race"], &["hfoo", "human"], &["Hart", "human"], &["nnon", "other"]]),
        "Units\\AbilityData.slk" => slk(&[&["alias", "hero"], &["AHhb", "1"], &["Aloc", "0"]]),
        "Units\\AbilityBuffData.slk" => slk(&[&["alias"], &["BHab"], &["Bxyz"]]),
        "Units\\UpgradeData.slk" => slk(&[&["upgradeid"], &["Rhme"]]),
        "Units\\DestructableData.slk" => slk(&[&["DestructableID", "Name", "EditorSuffix"], &["LTlt", "WESTRING_DEST_TREE", ""]]),
        "Units\\HumanUnitStrings.txt" => "[hfoo]\nName=Footman\n[Hart]\nName=Arthas\nEditorSuffix= (wielding Frostmourne)\n".into(),
        "Units\\HumanAbilityStrings.txt" => {
            "[AHhb]\nName=Holy Light\n[BHab]\nBufftip=Brilliance Aura\n[Bxyz]\nEditorName=Shown\nBufftip=Not shown\n".into()
        }
        "Units\\HumanUpgradeStrings.txt" => "[Rhme]\nName=Iron Forged Swords,Steel Forged Swords\n".into(),
        // A later file's key over an earlier one's.
        "Units\\CommandStrings.txt" => "[hfoo]\nName=\"Footman, the\"\n".into(),
        _ => return None,
    })
}

fn resolve(key: &str) -> String {
    if key == "WESTRING_DEST_TREE" { "Summer Tree Wall".into() } else { key.into() }
}

#[test]
fn the_games_objects_by_their_names() {
    let n = ObjectNames::load(files, resolve);
    let shown = |k: Kind, id: &str| n.get(k, id).map(Named::shown);
    assert_eq!(shown(Kind::Unit, "hfoo").as_deref(), Some("Footman, the"));
    assert_eq!(shown(Kind::Unit, "Hart").as_deref(), Some("Arthas (wielding Frostmourne)"));
    // Nothing names it: its id.
    assert_eq!(shown(Kind::Unit, "nnon").as_deref(), Some("nnon"));
    assert!(n.get(Kind::Ability, "AHhb").unwrap().hero && !n.get(Kind::Ability, "Aloc").unwrap().hero);
    assert_eq!(shown(Kind::Buff, "BHab").as_deref(), Some("Brilliance Aura"));
    assert_eq!(shown(Kind::Buff, "Bxyz").as_deref(), Some("Shown"));
    // An upgrade's first level's.
    assert_eq!(shown(Kind::Upgrade, "Rhme").as_deref(), Some("Iron Forged Swords"));
    // The table's column, resolved.
    assert_eq!(shown(Kind::Destructable, "LTlt").as_deref(), Some("Summer Tree Wall"));
    assert_eq!(n.of.get(&Kind::Item), None);
}

#[test]
fn the_maps_objects_over_them() {
    let mut n = ObjectNames::load(files, resolve);
    let meta = Metadata::from_tables(
        &slk(&[&["ID", "field"], &["unam", "Name"], &["unsf", "EditorSuffix"]]),
        &[],
    );
    let text = "[w3u]\nversion=2\n\n[Hart]\nunsf=\"(Evil)\"\n\n[h000]\n_parent=hfoo\nunam=\"TRIGSTR_001\"\n\n[h001]\n_parent=Hart\n";
    let p = crate::from_text(text, &meta);
    assert_eq!(p.errors, []);
    n.add_object_data(&p.data.unwrap(), &meta, |s| if s == "TRIGSTR_001" { "Captain".into() } else { s.into() });
    let get = |id: &str| n.get(Kind::Unit, id).unwrap();
    // A changed one: its suffix new, its name as it was.
    assert_eq!((get("Hart").shown(), get("Hart").custom), ("Arthas (Evil)".into(), false));
    // Custom ones: their own name, else their standard one's.
    assert_eq!((get("h000").shown(), get("h000").custom), ("Captain".into(), true));
    assert_eq!(get("h001").shown(), "Arthas (wielding Frostmourne)");
}

/// The game's files in a folder (`GAME_FILES`): every kind named.
#[test]
#[ignore]
fn names_of_a_real_game() {
    let dir = std::path::PathBuf::from(std::env::var("GAME_FILES").expect("GAME_FILES"));
    let read = |name: &str| std::fs::read(dir.join(name.replace('\\', "/"))).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
    let strings: HashMap<String, String> = ["UI\\WorldEditStrings.txt", "UI\\WorldEditGameStrings.txt"]
        .iter()
        .filter_map(|f| read(f))
        .flat_map(|t| bni_rs::parse(&t).sections.into_iter().flat_map(|s| s.items))
        .filter_map(|i| Some((i.key.as_ref()?.text.trim().to_string(), i.value_text().trim().to_string())))
        .collect();
    let n = ObjectNames::load(read, |k| strings.get(k).cloned().unwrap_or_else(|| k.to_string()));
    for kind in Kind::ALL {
        let all = &n.of[&kind];
        let unnamed: Vec<&String> = all.iter().filter(|(id, o)| &o.name == *id).map(|(id, _)| id).collect();
        println!("{kind:?}: {} ({} unnamed: {:?})", all.len(), unnamed.len(), &unnamed[..unnamed.len().min(8)]);
    }
    for (k, id) in [(Kind::Unit, "hfoo"), (Kind::Unit, "Uear"), (Kind::Ability, "AHhb"), (Kind::Buff, "BHab"), (Kind::Upgrade, "Rhme"), (Kind::Destructable, "LTlt"), (Kind::Doodad, "LOtr"), (Kind::Item, "ratc")] {
        println!("{id}: {:?}", n.get(k, id).map(Named::shown));
    }
}
