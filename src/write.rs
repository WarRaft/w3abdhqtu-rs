use parser_rs::{BinError, BinResult, Writer};

use crate::{Mod, Object, ObjectData, Value};

impl ObjectData {
    /// The file's bytes. Fails on what the format can't hold: an unknown
    /// version, several sets (or flags) before version 3, a NUL in a
    /// string.
    pub fn to_bytes(&self) -> BinResult<Vec<u8>> {
        if !crate::VERSIONS.contains(&self.version) {
            return Err(BinError::new(
                0,
                format!("unknown {} version {}", self.kind.extension(), self.version),
            ));
        }
        let mut w = Writer::new();
        w.u32(self.version);
        for table in [&self.originals, &self.customs] {
            w.count(table.len())?;
            for object in table {
                self.object(&mut w, object)?;
            }
        }
        Ok(w.into_bytes())
    }

    fn object(&self, w: &mut Writer, object: &Object) -> BinResult<()> {
        w.rawcode(object.base);
        w.rawcode(object.id);
        if self.version >= 3 {
            w.count(object.sets.len())?;
            for set in &object.sets {
                w.u32(set.flags);
                self.mods(w, &set.mods)?;
            }
            return Ok(());
        }
        match object.sets.as_slice() {
            [] => w.u32(0),
            [set] if set.flags == 0 => self.mods(w, &set.mods)?,
            _ => {
                let name = if object.id.is_none() {
                    object.base
                } else {
                    object.id
                };
                return Err(BinError::new(
                    w.len(),
                    format!("{name}: version {} has one set without flags", self.version),
                ));
            }
        }
        Ok(())
    }

    fn mods(&self, w: &mut Writer, mods: &[Mod]) -> BinResult<()> {
        w.count(mods.len())?;
        for m in mods {
            w.rawcode(m.field);
            w.u32(m.value.tag());
            if self.kind.has_levels(self.version) {
                w.u32(m.level);
                w.u32(m.column);
            }
            match &m.value {
                Value::Int(v) => w.i32(*v),
                Value::Real(v) | Value::Unreal(v) => w.f32(*v),
                Value::Str(s) => w.cstr(s)?,
            }
            w.rawcode(m.end);
        }
        Ok(())
    }
}
