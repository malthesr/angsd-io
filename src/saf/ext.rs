//! Conventional file name extensions for SAF files.

/// Conventional index file extension.
pub const INDEX_EXT: &str = "saf.idx";

/// Conventional positions file extension.
pub const POSITIONS_FILE_EXT: &str = "saf.pos.gz";

/// Conventional values file extension.
pub const VALUES_FILE_EXT: &str = "saf.gz";

const EXTS: [&str; 3] = [INDEX_EXT, POSITIONS_FILE_EXT, VALUES_FILE_EXT];

/// Returns the shared prefix of SAF file member paths given any one of them.
pub(super) fn prefix_from_member_path(s: &str) -> Option<&str> {
    EXTS.into_iter()
        .find(|ext| s.ends_with(ext))
        .and_then(|ext| s.strip_suffix(ext))
        .and_then(|s_stem| s_stem.strip_suffix('.'))
}

/// Returns the all three SAF file member paths given their shared prefix.
pub(super) fn member_paths_from_prefix(prefix: &str) -> [String; 3] {
    let create_path = |ext| format!("{prefix}.{ext}");
    let index_path = create_path(INDEX_EXT);
    let position_path = create_path(POSITIONS_FILE_EXT);
    let value_path = create_path(VALUES_FILE_EXT);

    [index_path, position_path, value_path]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_from_member_path() {
        assert_eq!(prefix_from_member_path(&"foo.saf.idx"), Some("foo"));
        assert_eq!(prefix_from_member_path(&"foo.bar.saf.idx"), Some("foo.bar"));
        assert_eq!(
            prefix_from_member_path(&"dir/bar.saf.pos.gz"),
            Some("dir/bar")
        );
        assert_eq!(
            prefix_from_member_path(&"/home/dir/baz.saf.gz"),
            Some("/home/dir/baz"),
        );
    }

    #[test]
    fn test_prefix_from_non_member_path_invalid() {
        assert_eq!(prefix_from_member_path(&"foo.saf.gz.idx"), None);
    }

    #[test]
    fn test_member_paths_from_prefix() {
        let [index_path, position_path, value_path] = member_paths_from_prefix("foo");
        assert_eq!(index_path, "foo.saf.idx");
        assert_eq!(position_path, "foo.saf.pos.gz");
        assert_eq!(value_path, "foo.saf.gz");
    }

    #[test]
    fn test_member_paths_from_prefix_with_extra_prefix() {
        let [index_path, position_path, value_path] = member_paths_from_prefix("foo.bar");
        assert_eq!(index_path, "foo.bar.saf.idx");
        assert_eq!(position_path, "foo.bar.saf.pos.gz");
        assert_eq!(value_path, "foo.bar.saf.gz");
    }
}
