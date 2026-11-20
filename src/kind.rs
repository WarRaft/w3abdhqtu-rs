/// Which object type a file holds. The binary layout is the same for all
/// of them, except that abilities, doodads and upgrades store a level and
/// a data column with every field ([`Kind::has_levels`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// `.w3a`
    Ability,
    /// `.w3b`
    Destructable,
    /// `.w3d`
    Doodad,
    /// `.w3h`
    Buff,
    /// `.w3q`
    Upgrade,
    /// `.w3t`
    Item,
    /// `.w3u`. In `.w3m` maps (Reign of Chaos) it holds items too.
    Unit,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::Ability,
        Kind::Destructable,
        Kind::Doodad,
        Kind::Buff,
        Kind::Upgrade,
        Kind::Item,
        Kind::Unit,
    ];

    /// The file extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Kind::Ability => "w3a",
            Kind::Destructable => "w3b",
            Kind::Doodad => "w3d",
            Kind::Buff => "w3h",
            Kind::Upgrade => "w3q",
            Kind::Item => "w3t",
            Kind::Unit => "w3u",
        }
    }

    /// The kind for an extension, with or without the dot, in any case.
    pub fn from_extension(ext: &str) -> Option<Kind> {
        let ext = ext.strip_prefix('.').unwrap_or(ext);
        Kind::ALL
            .into_iter()
            .find(|kind| kind.extension().eq_ignore_ascii_case(ext))
    }

    /// The kind for a file name or path, by its extension.
    pub fn from_path(path: &str) -> Option<Kind> {
        let (_, ext) = path.rsplit_once('.')?;
        Kind::from_extension(ext)
    }

    /// Whether every field of a file of this `version` carries a level
    /// (doodads: a variation) and a data column. Abilities and upgrades
    /// always do; doodads since version 2.
    pub fn has_levels(self, version: u32) -> bool {
        match self {
            Kind::Ability | Kind::Upgrade => true,
            Kind::Doodad => version >= 2,
            _ => false,
        }
    }
}
