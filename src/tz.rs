//! The local time zone, read from the platform's own data.
//!
//! Rust's standard library has no local-time API, so each platform is
//! asked in its own terms.
//!
//! On Unix this reads the TZif file the system already keeps
//! (RFC 8536): `/etc/localtime`, or the file `TZ` names under
//! `/usr/share/zoneinfo`. Only the 64-bit data block of a version 2
//! or later file is used; the POSIX rule string in the footer, which
//! governs instants after the last recorded transition, is not read,
//! so a date beyond the file's table takes the last offset in it.
//!
//! Windows keeps nothing of the sort — its zone data lives in the
//! registry in a shape of its own — so there it calls the API that
//! reads that data: the year's rules from
//! `GetTimeZoneInformationForYear`, applied by
//! `SystemTimeToTzSpecificLocalTime`. The offset is then the
//! difference the system itself computed, not one this file derived
//! from a rule. `TZ` is not consulted there, because the Win32 clock
//! does not consult it either.

/// What a zone was doing at one instant.
pub struct Zone {
    /// Seconds east of UTC: 3600 for +01:00, -18000 for -05:00.
    pub offset: i32,
    /// What the platform calls that period. On Unix it is the zone's
    /// own abbreviation, which for a zone that has none is already
    /// numeric ("+1245" for Chatham). On Windows it is the system's
    /// name for the zone ("W. Europe Daylight Time"), which is a full
    /// name and in the system's own language: Windows has no
    /// abbreviation to give, and the alternative was to throw its
    /// only naming away in favour of a number `offset` already holds.
    pub abbr: String,
    /// Whether that period is the zone's summer time.
    pub dst: bool,
}

/// The zone in effect at `at` (seconds since the epoch), or `None`
/// where the platform keeps nothing this can read — the wasm
/// playground, and on Unix a `TZ` naming a POSIX rule rather than a
/// file. Reporting nothing is deliberate: a caller told "unknown"
/// can say so, where a caller told UTC cannot tell the difference.
pub fn zone_at(at: i64) -> Option<Zone> {
    platform(at)
}

#[cfg(unix)]
fn platform(at: i64) -> Option<Zone> {
    parse(&read_tzif()?, at)
}

#[cfg(windows)]
fn platform(at: i64) -> Option<Zone> {
    win::zone_at(at)
}

/// Everywhere else — the wasm playground is the one that matters —
/// there is nothing to read, and saying so is the whole point.
#[cfg(not(any(unix, windows)))]
fn platform(_at: i64) -> Option<Zone> {
    None
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

#[cfg(unix)]
fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

#[cfg(unix)]
fn be64(b: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The six counts a TZif header ends with, and where its data begins.
#[cfg(unix)]
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

#[cfg(unix)]
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
#[cfg(unix)]
fn block_len(c: &Counts, width: usize) -> usize {
    c.times * width + c.times + c.types * 6 + c.chars + c.leap * (width + 4) + c.isstd + c.isut
}

#[cfg(unix)]
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

/// The civil date (year, month, day) a day count since 1970-01-01
/// falls on, and its inverse. Windows speaks in broken-down time and
/// this file speaks in instants, so the two have to meet somewhere.
/// The algorithms are the well-known era-based ones, which hold for
/// any year rather than for a table of them.
#[cfg(any(windows, test))]
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(any(windows, test))]
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = i64::from(if m > 2 { m - 3 } else { m + 9 });
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Windows, which keeps its zones in the registry rather than in a
/// file this could read, and hands them out only through these three
/// calls. Nothing here parses a rule: `SystemTimeToTzSpecificLocalTime`
/// applies the year's rules, and the offset is the difference between
/// what it returns and what it was given.
#[cfg(any(windows, test))]
mod win {
    #[cfg(windows)]
    use super::Zone;
    use super::{civil_from_days, days_from_civil};

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }

    #[cfg(windows)]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct TimeZoneInformation {
        bias: i32,
        standard_name: [u16; 32],
        standard_date: SystemTime,
        standard_bias: i32,
        daylight_name: [u16; 32],
        daylight_date: SystemTime,
        daylight_bias: i32,
    }

    #[cfg(windows)]
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct DynamicTimeZoneInformation {
        bias: i32,
        standard_name: [u16; 32],
        standard_date: SystemTime,
        standard_bias: i32,
        daylight_name: [u16; 32],
        daylight_date: SystemTime,
        daylight_bias: i32,
        time_zone_key_name: [u16; 128],
        dynamic_daylight_time_disabled: u8,
    }

    // These three layouts are a contract with a compiler that is not
    // this one, and a field in the wrong place would not fail to
    // compile — it would answer the wrong hour. The sizes are what
    // MSVC computes for SYSTEMTIME, TIME_ZONE_INFORMATION and
    // DYNAMIC_TIME_ZONE_INFORMATION, so checking them turns a layout
    // mistake into a build error on the platform that would suffer it.
    const _: () = assert!(size_of::<SystemTime>() == 16);
    #[cfg(windows)]
    const _: () = assert!(size_of::<TimeZoneInformation>() == 172);
    #[cfg(windows)]
    const _: () = assert!(size_of::<DynamicTimeZoneInformation>() == 432);

    #[cfg(windows)]
    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "GetDynamicTimeZoneInformation"]
        fn get_dynamic_time_zone_information(info: *mut DynamicTimeZoneInformation) -> u32;
        #[link_name = "GetTimeZoneInformationForYear"]
        fn get_time_zone_information_for_year(
            year: u16,
            dynamic: *const DynamicTimeZoneInformation,
            info: *mut TimeZoneInformation,
        ) -> i32;
        #[link_name = "SystemTimeToTzSpecificLocalTime"]
        fn system_time_to_tz_specific_local_time(
            info: *const TimeZoneInformation,
            universal: *const SystemTime,
            local: *mut SystemTime,
        ) -> i32;
    }

    /// TIME_ZONE_ID_INVALID, the one return that means the call failed.
    #[cfg(windows)]
    const INVALID: u32 = u32::MAX;

    fn broken_down(at: i64) -> SystemTime {
        let days = at.div_euclid(86400);
        let rest = at.rem_euclid(86400);
        let (y, m, d) = civil_from_days(days);
        SystemTime {
            year: y as u16,
            month: m as u16,
            // 1970-01-01 was a Thursday. The call ignores this on the
            // way in, but a struct that lies is a struct that will be
            // believed one day.
            day_of_week: (days + 4).rem_euclid(7) as u16,
            day: d as u16,
            hour: (rest / 3600) as u16,
            minute: (rest / 60 % 60) as u16,
            second: (rest % 60) as u16,
            milliseconds: 0,
        }
    }

    fn instant(st: &SystemTime) -> i64 {
        days_from_civil(i64::from(st.year), u32::from(st.month), u32::from(st.day)) * 86400
            + i64::from(st.hour) * 3600
            + i64::from(st.minute) * 60
            + i64::from(st.second)
    }

    fn name(chars: &[u16]) -> String {
        let end = chars.iter().position(|&c| c == 0).unwrap_or(chars.len());
        String::from_utf16_lossy(&chars[..end])
    }

    /// What a zone with no name of its own is called: the offset
    /// itself, spelled the way the zone files spell it ("+1245").
    fn numeric(offset: i32) -> String {
        let sign = if offset < 0 { '-' } else { '+' };
        let minutes = offset.abs() / 60;
        format!("{sign}{:02}{:02}", minutes / 60, minutes % 60)
    }

    #[cfg(windows)]
    pub fn zone_at(at: i64) -> Option<Zone> {
        let utc = broken_down(at);
        // Windows counts years in a u16 and its zone data starts in
        // 1601. Outside that it has nothing to say, and neither has
        // this.
        let (y, ..) = civil_from_days(at.div_euclid(86400));
        if !(1601..=30827).contains(&y) {
            return None;
        }

        let mut dynamic = std::mem::MaybeUninit::<DynamicTimeZoneInformation>::zeroed();
        let mut info = std::mem::MaybeUninit::<TimeZoneInformation>::zeroed();
        let mut local = SystemTime::default();
        // SAFETY: each call is given a pointer to storage of exactly
        // its own struct, and its result is checked before that
        // storage is read.
        let (info, local) = unsafe {
            let dynamic = if get_dynamic_time_zone_information(dynamic.as_mut_ptr()) == INVALID {
                std::ptr::null()
            } else {
                dynamic.as_ptr()
            };
            if get_time_zone_information_for_year(utc.year, dynamic, info.as_mut_ptr()) == 0 {
                return None;
            }
            let info = info.assume_init();
            if system_time_to_tz_specific_local_time(&info, &utc, &mut local) == 0 {
                return None;
            }
            (info, local)
        };

        let offset = (instant(&local) - at) as i32;
        // Windows biases are minutes *west* of UTC, so standard time
        // is east by the negative of their sum. A zone with no summer
        // time leaves DaylightDate zeroed and never moves off it.
        let standard = -(info.bias + info.standard_bias) * 60;
        let dst = info.daylight_date.month != 0 && offset != standard;
        let abbr = name(if dst {
            &info.daylight_name
        } else {
            &info.standard_name
        });
        let abbr = if abbr.is_empty() {
            numeric(offset)
        } else {
            abbr
        };
        Some(Zone { offset, abbr, dst })
    }

    /// What this host can check of the Windows path: everything
    /// except the three calls themselves. The broken-down time is the
    /// part written here, so it is the part that can be wrong.
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn an_instant_and_a_broken_down_time_are_the_same_thing_twice() {
            // Anchors read from `date -u`, not from this file.
            for (at, y, mo, d, h, mi, sec, dow) in [
                (0i64, 1970, 1, 1, 0, 0, 0, 4),         // a Thursday
                (1784116800, 2026, 7, 15, 12, 0, 0, 3), // a Wednesday
                (1774746000, 2026, 3, 29, 1, 0, 0, 0),  // a Sunday
                (-1, 1969, 12, 31, 23, 59, 59, 3),      // before the epoch
                (951782400, 2000, 2, 29, 0, 0, 0, 2),   // the leap day
            ] {
                let st = broken_down(at);
                assert_eq!(
                    (
                        st.year,
                        st.month,
                        st.day,
                        st.hour,
                        st.minute,
                        st.second,
                        st.day_of_week
                    ),
                    (y, mo, d, h, mi, sec, dow),
                    "broken down at {at}"
                );
                assert_eq!(instant(&st), at, "and back again from {at}");
            }
        }

        #[test]
        fn a_zone_with_no_name_is_called_by_its_offset() {
            assert_eq!(numeric(0), "+0000");
            assert_eq!(numeric(3600), "+0100");
            assert_eq!(numeric(45900), "+1245", "Chatham, which has no name either");
            assert_eq!(numeric(-14400), "-0400");
            assert_eq!(numeric(-1800), "-0030", "and a half hour west is not -0-30");
        }

        #[test]
        fn a_name_stops_at_the_nul_windows_pads_with() {
            let mut chars = [0u16; 32];
            for (i, c) in "W. Europe Standard Time".encode_utf16().enumerate() {
                chars[i] = c;
            }
            assert_eq!(name(&chars), "W. Europe Standard Time");
            assert_eq!(name(&[0u16; 32]), "", "a zone Windows did not name");
            assert_eq!(name(&[]), "", "and no room to name one at all");
        }
    }
}

#[cfg(test)]
mod civil {
    use super::*;

    #[test]
    fn a_day_count_and_a_date_are_the_same_thing_twice() {
        for (days, y, m, d) in [
            (0, 1970, 1, 1),
            (-1, 1969, 12, 31),
            (19723, 2024, 1, 1),
            (19782, 2024, 2, 29), // the leap day a wrong year rule loses
            (20543, 2026, 3, 31),
            (20544, 2026, 4, 1), // the month rolls where date says it does
            (-719162, 1, 1, 1),
            (2932896, 9999, 12, 31),
        ] {
            assert_eq!(civil_from_days(days), (y, m, d), "day {days}");
            assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
        }
    }

    #[test]
    fn every_day_of_four_centuries_round_trips() {
        // 1600-01-01 through 2000-01-01: every leap rule, including
        // the century that is a leap year and the three that are not.
        for days in -135140..10958 {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days, "{y}-{m:02}-{d:02}");
        }
    }
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
