//! The local time zone, read from the platform's own data.
//!
//! Rust's standard library has no local-time API, so this reads the
//! TZif file the system already keeps (RFC 8536): `/etc/localtime`,
//! or the file `TZ` names under `/usr/share/zoneinfo`. Only the
//! 64-bit data block of a version 2 or later file is used; the
//! POSIX rule string in the footer, which governs instants after the
//! last recorded transition, is not read, so a date beyond the
//! file's table takes the last offset in it.

/// What a zone was doing at one instant.
pub struct Zone {
    /// Seconds east of UTC: 3600 for +01:00, -18000 for -05:00.
    pub offset: i32,
    /// The zone's own abbreviation for that period, e.g. "CEST".
    pub abbr: String,
    /// Whether that period is the zone's summer time.
    pub dst: bool,
}

/// The zone in effect at `at` (seconds since the epoch), or `None`
/// where the platform keeps nothing this can read — which is every
/// answer on Windows, and a `TZ` naming a POSIX rule rather than a
/// file. Reporting nothing is deliberate: a caller told "unknown"
/// can say so, where a caller told UTC cannot tell the difference.
pub fn zone_at(at: i64) -> Option<Zone> {
    parse(&read_tzif()?, at)
}

/// The file to read for a given `TZ`: the system's own zone when it
/// is unset, and otherwise the file that names. A name is a path
/// under the zone directory, so one that could climb out of it —
/// or that is a POSIX rule string rather than a name — is refused
/// rather than resolved to some other zone.
#[cfg(unix)]
fn zone_path(tz: Option<&str>) -> Option<std::path::PathBuf> {
    let Some(tz) = tz.filter(|t| !t.is_empty()) else {
        return Some(std::path::PathBuf::from("/etc/localtime"));
    };
    let name = tz.strip_prefix(':').unwrap_or(tz);
    if name.starts_with('/') {
        return Some(std::path::PathBuf::from(name));
    }
    if name.is_empty()
        || name
            .split('/')
            .any(|p| p == ".." || p == "." || p.is_empty())
    {
        return None;
    }
    Some(std::path::Path::new("/usr/share/zoneinfo").join(name))
}

#[cfg(unix)]
fn read_tzif() -> Option<Vec<u8>> {
    let tz = std::env::var("TZ").ok();
    std::fs::read(zone_path(tz.as_deref())?).ok()
}

#[cfg(not(unix))]
fn read_tzif() -> Option<Vec<u8>> {
    None
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be64(b: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The six counts a TZif header ends with, and where its data begins.
struct Counts {
    isut: usize,
    isstd: usize,
    leap: usize,
    times: usize,
    types: usize,
    chars: usize,
    data: usize,
    version: u8,
}

fn header(b: &[u8], at: usize) -> Option<Counts> {
    if b.get(at..at + 4)? != b"TZif" {
        return None;
    }
    let n = |i: usize| be32(b, at + 20 + i * 4).map(|v| v as usize);
    Some(Counts {
        isut: n(0)?,
        isstd: n(1)?,
        leap: n(2)?,
        times: n(3)?,
        types: n(4)?,
        chars: n(5)?,
        data: at + 44,
        version: b[at + 4],
    })
}

/// The length of a data block whose transition times are `width`
/// bytes each (4 in the first block, 8 in the second).
fn block_len(c: &Counts, width: usize) -> usize {
    c.times * width + c.times + c.types * 6 + c.chars + c.leap * (width + 4) + c.isstd + c.isut
}

fn parse(b: &[u8], at: i64) -> Option<Zone> {
    let first = header(b, 0)?;
    // Version 1 files carry only 32-bit times, which run out in 2038.
    // Every system file has been version 2 or later for many years,
    // and the second block is the one worth reading.
    if first.version < b'2' {
        return None;
    }
    let c = header(b, first.data + block_len(&first, 4))?;
    if c.types == 0 {
        return None;
    }
    let types_at = c.data + c.times * 8;
    let infos_at = types_at + c.times;
    let chars_at = infos_at + c.types * 6;

    // The type in effect at `at`: the last transition at or before it.
    // Before the first transition a file says nothing, and RFC 8536
    // asks for the first type that is not summer time.
    let mut which = None;
    for i in 0..c.times {
        if be64(b, c.data + i * 8)? <= at {
            which = Some(*b.get(types_at + i)? as usize);
        } else {
            break;
        }
    }
    let which = match which {
        Some(i) if i < c.types => i,
        _ => (0..c.types)
            .find(|&i| b.get(infos_at + i * 6 + 4) == Some(&0))
            .unwrap_or(0),
    };

    let info = infos_at + which * 6;
    let offset = be32(b, info)? as i32;
    let dst = *b.get(info + 4)? != 0;
    let start = chars_at + *b.get(info + 5)? as usize;
    let end = b
        .get(start..chars_at + c.chars)?
        .iter()
        .position(|&c| c == 0)
        .map(|n| start + n)?;
    let abbr = String::from_utf8(b.get(start..end)?.to_vec()).ok()?;
    Some(Zone { offset, abbr, dst })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// The offsets `date` reports for Europe/Zurich, which is what
    /// this file exists to agree with. The last two are the point:
    /// a minute either side of a transition, where a rule guessed
    /// from the month would land on the wrong side.
    const ZURICH: [(i64, i32, &str, bool); 8] = [
        (1768478400, 3600, "CET", false), // 2026-01-15T12:00Z
        (1774745940, 3600, "CET", false), // 2026-03-29T00:59Z
        (1774746000, 7200, "CEST", true), // 2026-03-29T01:00Z
        (1784116800, 7200, "CEST", true), // 2026-07-15T12:00Z
        (1792889940, 7200, "CEST", true), // 2026-10-25T00:59Z
        (1792890000, 3600, "CET", false), // 2026-10-25T01:00Z
        (0, 3600, "CET", false),          // the epoch itself
        (328665600, 3600, "CET", false),  // 1980-06-01, before
                                          // Switzerland kept summer
                                          // time at all: June, and
                                          // still +01:00
    ];

    fn zurich() -> Option<Vec<u8>> {
        std::fs::read("/usr/share/zoneinfo/Europe/Zurich").ok()
    }

    #[test]
    fn a_zone_file_answers_what_date_answers() {
        let Some(b) = zurich() else {
            eprintln!("no /usr/share/zoneinfo/Europe/Zurich here; nothing to check");
            return;
        };
        for (at, offset, abbr, dst) in ZURICH {
            let z = parse(&b, at).expect("Europe/Zurich parses");
            assert_eq!(z.offset, offset, "offset at {at}");
            assert_eq!(z.abbr, abbr, "abbreviation at {at}");
            assert_eq!(z.dst, dst, "summer time at {at}");
        }
    }

    #[test]
    fn a_zone_that_never_moves_says_so() {
        let Ok(b) = std::fs::read("/usr/share/zoneinfo/UTC") else {
            eprintln!("no UTC zone file here; nothing to check");
            return;
        };
        let z = parse(&b, 1784116800).expect("UTC parses");
        assert_eq!(z.offset, 0);
        assert!(!z.dst);
    }

    #[test]
    fn nothing_that_is_not_a_zone_file_parses() {
        assert!(parse(b"", 0).is_none(), "empty");
        assert!(parse(b"not a tzif file at all", 0).is_none(), "wrong magic");
        let mut truncated = zurich().unwrap_or_default();
        if truncated.len() > 60 {
            truncated.truncate(60);
            assert!(parse(&truncated, 0).is_none(), "cut off mid-file");
        }
    }

    #[test]
    fn tz_names_a_file_under_the_zone_directory_and_nowhere_else() {
        let p = |tz: Option<&str>| zone_path(tz).map(|p| p.display().to_string());
        assert_eq!(
            p(None).as_deref(),
            Some("/etc/localtime"),
            "unset is the system zone"
        );
        assert_eq!(
            p(Some("")).as_deref(),
            Some("/etc/localtime"),
            "and so is empty"
        );
        assert_eq!(
            p(Some("Europe/Zurich")).as_deref(),
            Some("/usr/share/zoneinfo/Europe/Zurich"),
            "a name is a file under the zone directory"
        );
        assert_eq!(
            p(Some(":Europe/Zurich")).as_deref(),
            Some("/usr/share/zoneinfo/Europe/Zurich"),
            "the colon POSIX allows is not part of the name"
        );
        assert_eq!(
            p(Some("/etc/localtime")).as_deref(),
            Some("/etc/localtime"),
            "an absolute TZ is the file itself"
        );
        for out in ["../../etc/passwd", "..", "a/../../b", "a//b", "./x"] {
            assert_eq!(p(Some(out)), None, "{out} must not resolve to a path");
        }
    }
}
