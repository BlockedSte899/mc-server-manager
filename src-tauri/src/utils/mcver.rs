/// Minecraft-aware version ranking so the version picker always shows the
/// newest entries first, regardless of what order each API returns them in.
///
/// Rules:
/// - releases (`1.20.6`, `1.21`) sort by numeric segments;
/// - pre-releases/RCs (`1.20.6-pre3`, `-rc1`) sort just below their release;
/// - snapshots (`24w05a`) sort by (year, week, letter) and above releases of
///   older cycles once the list is reversed.
fn rank(v: &str) -> Vec<i32> {
    if let Some(r) = snapshot_rank(v) {
        return r;
    }
    release_rank(v)
}

fn release_rank(v: &str) -> Vec<i32> {
    let mut out: Vec<i32> = vec![0]; // release bucket
    let (base, marker, num) = match v.rsplit_once('-') {
        Some((b, s)) => {
            if let Some(n) = s.strip_prefix("pre").and_then(|x| x.parse::<i32>().ok()) {
                (b, -1, Some(n))
            } else if let Some(n) = s.strip_prefix("rc").and_then(|x| x.parse::<i32>().ok()) {
                (b, -2, Some(n))
            } else {
                (v, 0, None)
            }
        }
        None => (v, 0, None),
    };
    for seg in base.split('.') {
        out.push(seg.parse::<i32>().unwrap_or(0));
    }
    match num {
        Some(n) => {
            out.extend_from_slice(&[marker, n]);
        }
        None => out.extend_from_slice(&[0, 0]),
    }
    out
}

fn snapshot_rank(v: &str) -> Option<Vec<i32>> {
    let idx = v.find('w')?;
    let (year_s, rest) = v.split_at(idx);
    if year_s.is_empty() || !year_s.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let rest = &rest[1..];
    let (week_s, letter) = match rest.bytes().last() {
        Some(b) if b.is_ascii_alphabetic() => (&rest[..rest.len() - 1], (b - b'a') as i32),
        _ => (rest, 0),
    };
    let year: i32 = year_s.parse().ok()?;
    let week: i32 = week_s.parse().ok()?;
    Some(vec![1, year, week, letter])
}

fn cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let ra = rank(a);
    let rb = rank(b);
    let n = ra.len().max(rb.len());
    for i in 0..n {
        let x = ra.get(i).copied().unwrap_or(0);
        let y = rb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Sorts so the newest version comes first.
pub fn newest_first(mut versions: Vec<String>) -> Vec<String> {
    versions.sort_by(|a, b| cmp(b, a));
    versions
}
