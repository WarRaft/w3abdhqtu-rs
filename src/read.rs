use parser_rs::{BinResult, Reader};

use crate::{Kind, Mod, Object, ObjectData, Set, Value};

/// Smallest possible field change: id, tag, a one-byte string, end id.
const MIN_MOD: usize = 13;
/// Smallest possible set (version 3): flags and a count.
const MIN_SET: usize = 8;
/// Smallest possible object: two ids and a count.
const MIN_OBJECT: usize = 12;

/// Reads a whole file. Anything left after the data is an error.
pub fn parse(bytes: &[u8], kind: Kind) -> BinResult<ObjectData> {
    let mut r = Reader::new(bytes);
    let version = r.u32()?;
    if !crate::VERSIONS.contains(&version) {
        return Err(parser_rs::BinError::new(
            0,
            format!("unknown {} version {version}", kind.extension()),
        ));
    }
    let ctx = Ctx { kind, version };
    let originals = r.list(MIN_OBJECT, |r| ctx.object(r))?;
    let customs = r.list(MIN_OBJECT, |r| ctx.object(r))?;
    r.finish()?;
    Ok(ObjectData {
        kind,
        version,
        originals,
        customs,
    })
}

struct Ctx {
    kind: Kind,
    version: u32,
}

impl Ctx {
    fn min_mod(&self) -> usize {
        if self.kind.has_levels(self.version) {
            MIN_MOD + 8
        } else {
            MIN_MOD
        }
    }

    fn object(&self, r: &mut Reader) -> BinResult<Object> {
        let base = r.rawcode()?;
        let id = r.rawcode()?;
        let sets = if self.version >= 3 {
            r.list(MIN_SET, |r| {
                let flags = r.u32()?;
                Ok(Set {
                    flags,
                    mods: self.mods(r)?,
                })
            })?
        } else {
            vec![Set {
                flags: 0,
                mods: self.mods(r)?,
            }]
        };
        Ok(Object { base, id, sets })
    }

    fn mods(&self, r: &mut Reader) -> BinResult<Vec<Mod>> {
        r.list(self.min_mod(), |r| self.field(r))
    }

    fn field(&self, r: &mut Reader) -> BinResult<Mod> {
        let field = r.rawcode()?;
        let at = r.pos();
        let tag = r.u32()?;
        let (level, column) = if self.kind.has_levels(self.version) {
            (r.u32()?, r.u32()?)
        } else {
            (0, 0)
        };
        let value = match tag {
            0 => Value::Int(r.i32()?),
            1 => Value::Real(r.f32()?),
            2 => Value::Unreal(r.f32()?),
            3 => Value::Str(r.cstr()?),
            _ => {
                return Err(parser_rs::BinError::new(
                    at,
                    format!("field {field} has unknown value type {tag}"),
                ));
            }
        };
        let end = r.rawcode()?;
        Ok(Mod {
            field,
            level,
            column,
            value,
            end,
        })
    }
}
