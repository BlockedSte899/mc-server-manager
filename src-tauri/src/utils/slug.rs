use std::path::PathBuf;

/// Transliterates Cyrillic (and a few neighbours) to plain ASCII. Folder ids
/// travel through URL hashes, where non-ASCII gets percent-encoded and never
/// matches the on-disk folder name again — so ids must be ASCII-only.
fn cyrillic_translit(c: char) -> Option<&'static str> {
    let t = match c {
        'а' => "a",
        'б' => "b",
        'в' => "v",
        'г' => "g",
        'д' => "d",
        'е' => "e",
        'ё' => "yo",
        'ж' => "zh",
        'з' => "z",
        'и' => "i",
        'й' => "y",
        'к' => "k",
        'л' => "l",
        'м' => "m",
        'н' => "n",
        'о' => "o",
        'п' => "p",
        'р' => "r",
        'с' => "s",
        'т' => "t",
        'у' => "u",
        'ф' => "f",
        'х' => "h",
        'ц' => "ts",
        'ч' => "ch",
        'ш' => "sh",
        'щ' => "sch",
        'ъ' => "",
        'ы' => "y",
        'ь' => "",
        'э' => "e",
        'ю' => "yu",
        'я' => "ya",
        'і' => "i",
        'ї' => "yi",
        'є' => "ye",
        'ґ' => "g",
        _ => return None,
    };
    Some(t)
}

/// Turns an arbitrary name into a url/folder friendly slug.
pub fn slugify(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_string())
            } else if let Some(t) = cyrillic_translit(c) {
                Some(t.to_string())
            } else {
                Some('-'.to_string())
            }
        })
        .collect();
    let parts: Vec<&str> = cleaned.split('-').filter(|s| !s.is_empty()).collect();
    let base = if parts.is_empty() {
        "server".to_string()
    } else {
        parts.join("-")
    };
    base
}

/// Returns a unique folder name and its path under servers_dir.
pub fn unique_folder(servers_dir: &std::path::Path, slug: &str) -> (String, PathBuf) {
    let mut candidate = slug.to_string();
    let mut n = 2;
    let mut path = servers_dir.join(&candidate);
    while path.exists() {
        candidate = format!("{slug}-{n}");
        path = servers_dir.join(&candidate);
        n += 1;
    }
    (candidate, path)
}

#[cfg(test)]
mod tests {
    use super::{slugify, unique_folder};

    #[test]
    fn cyrillic_names_become_ascii_slugs() {
        assert_eq!(slugify("Мой Сервер"), "moy-server");
        assert_eq!(slugify("Ёлки-Палки"), "yolki-palki");
        assert_eq!(slugify("!!!"), "server");
        assert_eq!(slugify("Test 123"), "test-123");
    }

    #[test]
    fn unique_folder_skips_existing() {
        let dir = std::env::temp_dir().join(format!("mcsm-slug-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (id1, _) = unique_folder(&dir, "foo");
        std::fs::create_dir_all(dir.join(&id1)).unwrap();
        let (id2, _) = unique_folder(&dir, "foo");
        assert_eq!(id1, "foo");
        assert_eq!(id2, "foo-2");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
