use flate2::read::GzDecoder;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct WorldSettings {
    pub seed: Option<i64>,
    pub difficulty: String,
    pub hardcore: bool,
    pub allow_cheats: bool,
    pub gamerules: Vec<Gamerule>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Gamerule {
    pub name: String,
    pub value: String,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self {
            seed: None,
            difficulty: "normal".into(),
            hardcore: false,
            allow_cheats: false,
            gamerules: Vec::new(),
        }
    }
}

/// Reads `level.dat` (gzip-compressed NBT, big-endian) from `<world>/level.dat`
/// and extracts seed / difficulty / hardcore / allow-cheats / gamerules without
/// pulling in a heavy NBT dependency.
pub fn read_settings(server_dir: &Path, world: &str) -> Result<WorldSettings, String> {
    let mut out = WorldSettings::default();
    let path = server_dir.join(world).join("level.dat");
    let raw = match std::fs::read(&path) {
        Ok(b) if !b.is_empty() => b,
        _ => return Ok(out),
    };
    let bytes = decompress(&raw).unwrap_or(raw);
    let Ok(v) = nbt_parse(&bytes) else {
        return Ok(out);
    };
    let data = match &v {
        Val::Compound(map) => map.get("Data").and_then(|d| match d {
            Val::Compound(m) => Some(m),
            _ => None,
        }),
        _ => None,
    };
    let Some(data) = data else { return Ok(out) };

    // seed: WorldGenSettings.seed (1.18+) else RandomSeed (1.17-)
    let seed = match data.get("WorldGenSettings") {
        Some(Val::Compound(wg)) => as_int(wg.get("seed")),
        _ => None,
    }
    .or_else(|| as_int(data.get("RandomSeed")));
    out.seed = seed;

    if let Some(d) = as_int(data.get("Difficulty")) {
        out.difficulty = match d {
            0 => "peaceful".into(),
            1 => "easy".into(),
            2 => "normal".into(),
            3 => "hard".into(),
            _ => "normal".into(),
        };
    }
    out.hardcore = as_int(data.get("hardcore")).unwrap_or(0) != 0;
    out.allow_cheats = as_int(data.get("allowCommands")).unwrap_or(0) != 0;

    if let Some(Val::Compound(rules)) = data.get("GameRules") {
        let mut list: Vec<Gamerule> = rules
            .iter()
            .filter_map(|(k, v)| match v {
                Val::Str(s) => Some(Gamerule {
                    name: k.clone(),
                    value: s.clone(),
                }),
                _ => None,
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        out.gamerules = list;
    }
    Ok(out)
}

fn as_int(v: Option<&Val>) -> Option<i64> {
    match v {
        Some(Val::Byte(b)) => Some(*b as i64),
        Some(Val::Short(s)) => Some(*s as i64),
        Some(Val::Int(i)) => Some(*i as i64),
        Some(Val::Long(l)) => Some(*l),
        _ => None,
    }
}

fn decompress(raw: &[u8]) -> Option<Vec<u8>> {
    if !(raw.len() > 2 && raw[0] == 0x1f && raw[1] == 0x8b) {
        return None;
    }
    let mut dec = GzDecoder::new(raw);
    let mut out = Vec::new();
    dec.read_to_end(&mut out).ok()?;
    Some(out)
}

// ---- minimal NBT reader (big-endian TAG formats, only what we need) ---------

#[derive(Clone, Debug, PartialEq)]
enum Val {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Str(String),
    Compound(BTreeMap<String, Val>),
}

fn nbt_parse(bytes: &[u8]) -> Result<Val, String> {
    let mut p = Parser { b: bytes, pos: 0 };
    if p.u8()? != 0x0A {
        return Err("not an NBT compound".into());
    }
    p.name()?;
    let v = p.compound_payload()?;
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.pos + n > self.b.len() {
            return Err("nbt: unexpected end".into());
        }
        let s = &self.b[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn i16(&mut self) -> Result<i16, String> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn name(&mut self) -> Result<String, String> {
        let len = self.i16()? as usize;
        Ok(String::from_utf8_lossy(self.take(len)?).into_owned())
    }
    fn compound_payload(&mut self) -> Result<Val, String> {
        let mut map = BTreeMap::new();
        loop {
            let tag = self.u8()?;
            if tag == 0 {
                break;
            }
            let key = self.name()?;
            let v = self.tag(tag)?;
            map.insert(key, v);
        }
        Ok(Val::Compound(map))
    }
    fn tag(&mut self, tag: u8) -> Result<Val, String> {
        match tag {
            1 => Ok(Val::Byte(self.take(1)?[0] as i8)),
            2 => Ok(Val::Short(i16::from_be_bytes(
                self.take(2)?.try_into().unwrap(),
            ))),
            3 => Ok(Val::Int(i32::from_be_bytes(
                self.take(4)?.try_into().unwrap(),
            ))),
            4 => Ok(Val::Long(i64::from_be_bytes(
                self.take(8)?.try_into().unwrap(),
            ))),
            5 => {
                let _ = self.take(4)?;
                Ok(Val::Int(0))
            }
            6 => {
                let _ = self.take(8)?;
                Ok(Val::Int(0))
            }
            7 => {
                let n = self.i32()?;
                let n = n.max(0) as usize;
                if n > self.b.len() {
                    return Err("nbt: bad byte array".into());
                }
                let _ = self.take(n)?;
                Ok(Val::Int(0))
            }
            8 => {
                let len = self.i16()? as usize;
                Ok(Val::Str(
                    String::from_utf8_lossy(self.take(len)?).into_owned(),
                ))
            }
            9 => {
                let _et = self.u8()?;
                let n = self.i32()?;
                let n = n.max(0) as usize;
                for _ in 0..n {
                    let sub = self.tag(_et)?;
                    let _ = sub;
                }
                Ok(Val::Int(0))
            }
            10 => self.compound_payload(),
            11 | 12 => {
                let n = self.i32()?;
                let n = n.max(0) as usize;
                let sz = if tag == 11 { 4 } else { 8 };
                let total = n.saturating_mul(sz);
                if self.pos + total > self.b.len() {
                    return Err("nbt: bad int array".into());
                }
                let _ = self.take(total)?;
                Ok(Val::Int(0))
            }
            _ => Err("nbt: unknown tag".into()),
        }
    }
}
