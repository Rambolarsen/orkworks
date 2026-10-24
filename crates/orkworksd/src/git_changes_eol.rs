//! Built-in Git CRLF cleaning only; never loads or executes external filters.
use git2::{AttrCheckFlags, AttrValue, Oid, Repository};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Raw,
    Auto,
    AutoIndexSame,
    Text,
}
impl Eol {
    pub fn clean(self, bytes: &mut Vec<u8>) {
        if self == Self::Raw {
            return;
        }
        if matches!(self, Self::Auto | Self::AutoIndexSame) {
            let mut stats = Stats::default();
            stats.add(bytes);
            if stats.binary() {
                return;
            }
        }
        let mut to = 0;
        for from in 0..bytes.len() {
            if bytes[from] == b'\r' && bytes.get(from + 1) == Some(&b'\n') {
                continue;
            }
            bytes[to] = bytes[from];
            to += 1;
        }
        bytes.truncate(to);
    }
    pub fn converts(self, stats: &Stats) -> bool {
        self == Self::Text || (matches!(self, Self::Auto | Self::AutoIndexSame) && !stats.binary())
    }
}
#[derive(Default)]
pub struct Stats {
    pub crlf: u64,
    cr: u64,
    nul: bool,
    printable: u64,
    control: u64,
    last: Option<u8>,
}
impl Stats {
    pub fn add(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.last == Some(b'\r') && b == b'\n' {
                self.crlf += 1
            }
            match b {
                b'\r' => self.cr += 1,
                b'\n' => {}
                0 => {
                    self.nul = true;
                    self.control += 1
                }
                b'\t' | 8 | 11 | 12 | 27 => self.printable += 1,
                32..=126 | 128..=255 => self.printable += 1,
                _ => self.control += 1,
            }
            self.last = Some(b);
        }
    }
    pub fn binary(&self) -> bool {
        self.nul
            || self.cr != self.crlf
            || self
                .control
                .saturating_sub(u64::from(self.last == Some(26)))
                > (self.printable >> 7)
    }
}
pub fn policy(
    attrs: &Repository,
    repo: &Repository,
    path: &str,
    index: Option<Oid>,
    head: Option<Oid>,
) -> Result<Eol, String> {
    let attr = |name| {
        attrs
            .get_attr(
                std::path::Path::new(path),
                name,
                AttrCheckFlags::FILE_THEN_INDEX | AttrCheckFlags::NO_SYSTEM,
            )
            .map(AttrValue::from_string)
            .map_err(|e| e.to_string())
    };
    let text = attr("text")?;
    if text == AttrValue::False {
        return Ok(Eol::Raw);
    }
    let mut mode = match text {
        AttrValue::True => Some(Eol::Text),
        AttrValue::String("auto") => Some(Eol::Auto),
        _ => None,
    };
    if mode.is_none() {
        mode = match attr("crlf")? {
            AttrValue::False => Some(Eol::Raw),
            AttrValue::True | AttrValue::String("input") => Some(Eol::Text),
            AttrValue::String("auto") => Some(Eol::Auto),
            _ => None,
        };
    }
    if mode == Some(Eol::Raw) {
        return Ok(Eol::Raw);
    }
    if mode.is_none() && matches!(attr("eol")?, AttrValue::String("lf" | "crlf")) {
        mode = Some(Eol::Text)
    }
    let config = repo.config().map_err(|e| e.to_string())?;
    let auto = config.get_bool("core.autocrlf").unwrap_or(false)
        || config
            .get_string("core.autocrlf")
            .is_ok_and(|s| s.eq_ignore_ascii_case("input"));
    let mode = mode.unwrap_or(if auto { Eol::Auto } else { Eol::Raw });
    if mode == Eol::Auto {
        if let Some(id) = index {
            let size = repo
                .odb()
                .and_then(|odb| odb.read_header(id))
                .map_err(|e| e.to_string())?
                .0;
            if size <= super::git_changes::INPUT_LIMIT {
                if repo
                    .find_blob(id)
                    .map_err(|e| e.to_string())?
                    .content()
                    .contains(&b'\r')
                {
                    return Ok(Eol::Raw);
                }
            } else if Some(id) != head {
                return Err(
                    "Index content exceeds normalization budget; patch unavailable.".into(),
                );
            } else {
                return Ok(Eol::AutoIndexSame);
            }
        }
    }
    Ok(mode)
}
