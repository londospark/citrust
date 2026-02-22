use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum KeyDbError {
    #[error("key file not found at {0}")]
    FileNotFound(PathBuf),
    #[error("line {line}: {reason}")]
    ParseError { line: usize, reason: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A database of 128-bit AES keys parsed from a Citra-compatible `aes_keys.txt` file.
#[derive(Debug, Clone)]
pub struct KeyDatabase {
    keys: HashMap<String, u128>,
}

impl KeyDatabase {
    /// Parse key database from any buffered reader.
    pub fn from_reader(reader: impl BufRead) -> Result<Self, KeyDbError> {
        let mut keys = HashMap::new();
        let mut first_line = true;

        for (line_idx, line_result) in reader.lines().enumerate() {
            let line_num = line_idx + 1;
            let mut line = line_result.map_err(KeyDbError::Io)?;

            // Strip BOM from very first line
            if first_line {
                if let Some(stripped) = line.strip_prefix('\u{FEFF}') {
                    line = stripped.to_string();
                }
                first_line = false;
            }

            let trimmed = line.trim();

            // Skip blanks and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Split on first '=' only
            let Some((name_raw, value_raw)) = trimmed.split_once('=') else {
                return Err(KeyDbError::ParseError {
                    line: line_num,
                    reason: "missing '=' delimiter".to_string(),
                });
            };

            let name = name_raw.trim().to_lowercase();
            let value = value_raw.trim();

            // Validate hex length
            if value.len() != 32 {
                return Err(KeyDbError::ParseError {
                    line: line_num,
                    reason: format!(
                        "expected 32 hex characters, got {} (\"{}\")",
                        value.len(),
                        value
                    ),
                });
            }

            // Validate hex characters
            for (i, ch) in value.chars().enumerate() {
                if !ch.is_ascii_hexdigit() {
                    return Err(KeyDbError::ParseError {
                        line: line_num,
                        reason: format!("invalid hex character '{}' at position {}", ch, i + 1),
                    });
                }
            }

            let parsed = u128::from_str_radix(value, 16).map_err(|e| KeyDbError::ParseError {
                line: line_num,
                reason: format!("hex parse error: {e}"),
            })?;

            if keys.contains_key(&name) {
                eprintln!(
                    "warning: duplicate key '{}' on line {}, overwriting",
                    name, line_num
                );
            }

            keys.insert(name, parsed);
        }

        Ok(KeyDatabase { keys })
    }

    /// Parse key database from a file path.
    pub fn from_file(path: &Path) -> Result<Self, KeyDbError> {
        if !path.exists() {
            return Err(KeyDbError::FileNotFound(path.to_path_buf()));
        }
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Search default locations for an `aes_keys.txt` file. Returns the first found.
    pub fn search_default_locations() -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = vec![PathBuf::from("aes_keys.txt")];

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = std::env::var_os("HOME") {
                let home = PathBuf::from(home);
                candidates.push(home.join(".config/citrust/aes_keys.txt"));
                candidates.push(home.join(".local/share/citra-emu/sysdata/aes_keys.txt"));
                candidates.push(home.join(".local/share/azahar-emu/sysdata/aes_keys.txt"));
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                let appdata = PathBuf::from(appdata);
                candidates.push(appdata.join("citrust\\aes_keys.txt"));
                candidates.push(appdata.join("Citra\\sysdata\\aes_keys.txt"));
            }
        }

        candidates.into_iter().find(|p| p.exists())
    }

    /// Get the generator constant.
    pub fn generator(&self) -> Option<u128> {
        self.keys.get("generator").copied()
    }

    /// Get KeyX for a given slot number.
    pub fn get_key_x(&self, slot: u8) -> Option<u128> {
        self.keys.get(&format!("slot0x{:02x}keyx", slot)).copied()
    }

    /// Get KeyY for a given slot number.
    pub fn get_key_y(&self, slot: u8) -> Option<u128> {
        self.keys.get(&format!("slot0x{:02x}keyy", slot)).copied()
    }

    /// Get Normal key for a given slot number.
    pub fn get_key_n(&self, slot: u8) -> Option<u128> {
        self.keys.get(&format!("slot0x{:02x}keyn", slot)).copied()
    }

    /// Get common key by index.
    pub fn get_common(&self, idx: u8) -> Option<u128> {
        self.keys.get(&format!("common{}", idx)).copied()
    }

    /// Get common normal key by index.
    pub fn get_common_n(&self, idx: u8) -> Option<u128> {
        self.keys.get(&format!("common{}n", idx)).copied()
    }

    /// Raw key lookup by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<u128> {
        self.keys.get(&name.to_lowercase()).copied()
    }

    /// Number of keys loaded.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the database is empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse(input: &str) -> Result<KeyDatabase, KeyDbError> {
        KeyDatabase::from_reader(Cursor::new(input))
    }

    #[test]
    fn test_parse_valid_multiline() {
        let input = "\
generator=1FF9E9AAC5FE0408024591DC5D52768A
slot0x2CKeyX=B98E95CECA3E4D171F76A94DE934C053
slot0x25KeyX=CEE7D8AB30C00DAE850EF5E382AC5AF3
";
        let db = parse(input).unwrap();
        assert_eq!(db.len(), 3);
        assert_eq!(db.generator(), Some(0x1FF9E9AAC5FE0408024591DC5D52768Au128));
        assert_eq!(
            db.get_key_x(0x2C),
            Some(0xB98E95CECA3E4D171F76A94DE934C053u128)
        );
        assert_eq!(
            db.get_key_x(0x25),
            Some(0xCEE7D8AB30C00DAE850EF5E382AC5AF3u128)
        );
    }

    #[test]
    fn test_parse_comments_and_blank_lines() {
        let input = "\
# This is a comment
  
generator=1FF9E9AAC5FE0408024591DC5D52768A

# Another comment
slot0x2CKeyX=B98E95CECA3E4D171F76A94DE934C053
";
        let db = parse(input).unwrap();
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn test_parse_with_bom() {
        let input = "\u{FEFF}generator=1FF9E9AAC5FE0408024591DC5D52768A\n";
        let db = parse(input).unwrap();
        assert_eq!(db.len(), 1);
        assert!(db.generator().is_some());
    }

    #[test]
    fn test_error_invalid_hex() {
        let input = "generator=1FF9E9AAC5FE0408024591DC5D52ZZZZ\n";
        let err = parse(input).unwrap_err();
        match err {
            KeyDbError::ParseError { line, reason } => {
                assert_eq!(line, 1);
                assert!(reason.contains("invalid hex character"), "got: {reason}");
            }
            _ => panic!("expected ParseError, got {err:?}"),
        }
    }

    #[test]
    fn test_error_wrong_length() {
        let input = "generator=1FF9E9AAC5FE04\n";
        let err = parse(input).unwrap_err();
        match err {
            KeyDbError::ParseError { line, reason } => {
                assert_eq!(line, 1);
                assert!(
                    reason.contains("expected 32 hex characters"),
                    "got: {reason}"
                );
            }
            _ => panic!("expected ParseError, got {err:?}"),
        }
    }

    #[test]
    fn test_error_no_equals() {
        let input = "this line has no equals sign\n";
        let err = parse(input).unwrap_err();
        match err {
            KeyDbError::ParseError { line, reason } => {
                assert_eq!(line, 1);
                assert!(reason.contains("missing '='"), "got: {reason}");
            }
            _ => panic!("expected ParseError, got {err:?}"),
        }
    }

    #[test]
    fn test_lookup_methods() {
        let input = "\
slot0x2CKeyX=B98E95CECA3E4D171F76A94DE934C053
slot0x18KeyY=00000000000000000000000000000001
slot0x0CKeyN=E7C9FF9D4F5B6F4DC5E2F50E856F0AB2
common0=D07B337F9CA4385932A2E25723232EB9
common0N=64C5FD55DD3AD988325BAAEC5243DB98
";
        let db = parse(input).unwrap();
        assert_eq!(
            db.get_key_x(0x2C),
            Some(0xB98E95CECA3E4D171F76A94DE934C053u128)
        );
        assert_eq!(db.get_key_y(0x18), Some(1u128));
        assert_eq!(
            db.get_key_n(0x0C),
            Some(0xE7C9FF9D4F5B6F4DC5E2F50E856F0AB2u128)
        );
        assert_eq!(
            db.get_common(0),
            Some(0xD07B337F9CA4385932A2E25723232EB9u128)
        );
        assert_eq!(
            db.get_common_n(0),
            Some(0x64C5FD55DD3AD988325BAAEC5243DB98u128)
        );
    }

    #[test]
    fn test_missing_key_returns_none() {
        let input = "generator=1FF9E9AAC5FE0408024591DC5D52768A\n";
        let db = parse(input).unwrap();
        assert_eq!(db.get_key_x(0xFF), None);
        assert_eq!(db.get_key_y(0x00), None);
        assert_eq!(db.get_common(9), None);
    }

    #[test]
    fn test_case_insensitive_key_names() {
        let input = "Generator=1FF9E9AAC5FE0408024591DC5D52768A\n";
        let db = parse(input).unwrap();
        assert!(db.generator().is_some());
        assert!(db.get("GENERATOR").is_some());
        assert!(db.get("generator").is_some());
    }

    #[test]
    fn test_duplicate_key_last_wins() {
        let input = "\
generator=1FF9E9AAC5FE0408024591DC5D52768A
generator=00000000000000000000000000000001
";
        let db = parse(input).unwrap();
        assert_eq!(db.generator(), Some(1u128));
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_empty_file() {
        let db = parse("").unwrap();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
        assert_eq!(db.generator(), None);
    }

    #[test]
    fn test_search_default_locations_returns_none() {
        // In a test environment without any key file present in default locations,
        // this should return None (we can't guarantee any of the paths exist).
        // This test mainly verifies the function doesn't panic.
        let _result = KeyDatabase::search_default_locations();
    }

    #[test]
    fn test_mixed_case_hex_values() {
        let input = "generator=1ff9E9aAc5Fe0408024591Dc5d52768a\n";
        let db = parse(input).unwrap();
        assert_eq!(db.generator(), Some(0x1FF9E9AAC5FE0408024591DC5D52768Au128));
    }

    #[test]
    fn test_whitespace_trimming() {
        let input = "  generator  =  1FF9E9AAC5FE0408024591DC5D52768A  \n";
        let db = parse(input).unwrap();
        assert!(db.generator().is_some());
    }

    #[test]
    fn test_windows_line_endings() {
        let input = "generator=1FF9E9AAC5FE0408024591DC5D52768A\r\nslot0x2CKeyX=B98E95CECA3E4D171F76A94DE934C053\r\n";
        let db = parse(input).unwrap();
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn test_file_not_found() {
        let err = KeyDatabase::from_file(Path::new("nonexistent_keys_file.txt")).unwrap_err();
        assert!(matches!(err, KeyDbError::FileNotFound(_)));
    }

    #[test]
    fn test_split_on_first_equals_only() {
        // Value can't contain '=', but if it did by some odd format, we split on first only.
        // Here the value after first '=' is "B98E=5CECA3E4D171F76A94DE934C053" which is 33 chars → error
        let input = "slot0x2CKeyX=B98E=5CECA3E4D171F76A94DE934C053\n";
        let err = parse(input).unwrap_err();
        assert!(matches!(err, KeyDbError::ParseError { .. }));
    }
}
