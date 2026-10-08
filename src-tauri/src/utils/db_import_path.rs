pub fn resolve_custom_db_import_path(
    import_dir: &std::path::Path,
    filename: &str,
) -> Result<std::path::PathBuf, String> {
    let requested = std::path::Path::new(filename);
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || requested.file_name() != Some(std::ffi::OsStr::new(filename))
    {
        return Err("Custom database import accepts a filename only".to_string());
    }

    // Return the directory entry's path, never a path constructed from request data.
    // file_type excludes symlinks, so an entry cannot redirect outside this directory.
    for entry in std::fs::read_dir(import_dir)
        .map_err(|_| "Custom database import directory is unavailable".to_string())?
    {
        let entry =
            entry.map_err(|_| "Cannot read custom database import directory".to_string())?;
        if entry.file_name() == requested.as_os_str()
            && entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
        {
            return Ok(entry.path());
        }
    }
    Err("Custom database file is not available in the import directory".to_string())
}

#[cfg(test)]
mod custom_db_import_path_tests {
    use super::resolve_custom_db_import_path;

    #[test]
    fn custom_db_import_path_accepts_only_existing_files_in_configured_directory() {
        let root = tempfile::tempdir().unwrap();
        let selected = root.path().join("state.vscdb");
        std::fs::write(&selected, b"fixture").unwrap();
        std::fs::create_dir(root.path().join("directory.vscdb")).unwrap();
        assert_eq!(
            resolve_custom_db_import_path(root.path(), "state.vscdb").unwrap(),
            selected
        );
        for invalid in [
            "",
            ".",
            "..",
            "../state.vscdb",
            "..\\state.vscdb",
            "/state.vscdb",
            "C:\\state.vscdb",
            "missing.vscdb",
            "directory.vscdb",
        ] {
            assert!(resolve_custom_db_import_path(root.path(), invalid).is_err());
        }
        assert!(resolve_custom_db_import_path(root.path(), selected.to_str().unwrap()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn custom_db_import_path_rejects_symlink_to_outside_directory() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("linked.vscdb")).unwrap();
        assert!(resolve_custom_db_import_path(root.path(), "linked.vscdb").is_err());
    }
}
