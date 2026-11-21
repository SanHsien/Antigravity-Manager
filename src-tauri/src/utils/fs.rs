use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Platform-specific atomic file replacement
#[cfg(target_os = "windows")]
fn atomic_replace_file(src: &Path, dst: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    type Bool = i32;
    type Dword = u32;

    #[link(name = "Kernel32")]
    extern "system" {
        fn MoveFileExW(
            lp_existing_file_name: *const u16,
            lp_new_file_name: *const u16,
            dw_flags: Dword,
        ) -> Bool;
    }

    let src_wide: Vec<u16> = src
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let dst_wide: Vec<u16> = dst
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // MOVEFILE_REPLACE_EXISTING = 0x1
    // MOVEFILE_WRITE_THROUGH = 0x8
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;

    // Antivirus scanners, indexers and concurrent readers briefly hold the target
    // without FILE_SHARE_DELETE; MoveFileExW then fails with ERROR_ACCESS_DENIED (5)
    // or ERROR_SHARING_VIOLATION (32). Retry those with a bounded backoff
    // (5+10+20+40+80 ms) before giving up (upstream 90241980, #3881).
    const ERROR_ACCESS_DENIED: i32 = 5;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    const MAX_RETRIES: u32 = 5;

    let mut attempt: u32 = 0;
    loop {
        let result = unsafe { MoveFileExW(src_wide.as_ptr(), dst_wide.as_ptr(), flags) };
        if result != 0 {
            return Ok(());
        }

        let err = std::io::Error::last_os_error();
        let transient = matches!(
            err.raw_os_error(),
            Some(ERROR_ACCESS_DENIED) | Some(ERROR_SHARING_VIOLATION)
        );
        if transient && attempt < MAX_RETRIES {
            std::thread::sleep(std::time::Duration::from_millis(5u64 << attempt));
            attempt += 1;
            continue;
        }

        let _ = std::fs::remove_file(src);
        return Err(format!(
            "MoveFileExW failed after {} attempt(s): {}",
            attempt + 1,
            err
        ));
    }
}

/// Non-Windows: use standard atomic rename
#[cfg(not(target_os = "windows"))]
fn atomic_replace_file(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::rename(src, dst).map_err(|e| format!("rename failed: {}", e))
}

/// Atomically write bytes to a file at `target_path`.
///
/// Steps:
/// 1. Create a unique temporary file in the same directory (`<file>.tmp.<uuid>`).
/// 2. Write content and call `sync_all()` to flush buffers to physical disk.
/// 3. Replace target file via atomic rename (`MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH` on Windows, `fs::rename` on POSIX).
/// 4. If any step fails, remove the temporary file.
pub fn write_atomic<P: AsRef<Path>>(target_path: P, content: &[u8]) -> Result<(), String> {
    let target = target_path.as_ref();
    let parent_dir = target
        .parent()
        .ok_or_else(|| "Target path has no parent directory".to_string())?;

    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let temp_filename = format!("{}.tmp.{}", file_name, Uuid::new_v4());
    let temp_path: PathBuf = parent_dir.join(temp_filename);

    let mut file = File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file {:?}: {}", temp_path, e))?;

    if let Err(e) = file.write_all(content) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!(
            "Failed to write to temporary file {:?}: {}",
            temp_path, e
        ));
    }

    if let Err(e) = file.sync_all() {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!(
            "Failed to fsync temporary file {:?}: {}",
            temp_path, e
        ));
    }

    // Explicitly drop file handle before rename to release Windows lock
    drop(file);

    if let Err(e) = atomic_replace_file(&temp_path, target) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Failed to atomically replace {:?}: {}", target, e));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_write_atomic_basic() {
        let temp_dir = std::env::temp_dir().join(format!("test_atomic_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let target_file = temp_dir.join("config.json");
        let initial_data = b"{\"key\":\"initial_value\"}";
        write_atomic(&target_file, initial_data).expect("First write should succeed");

        assert_eq!(fs::read(&target_file).unwrap(), initial_data);

        let updated_data = b"{\"key\":\"updated_value\"}";
        write_atomic(&target_file, updated_data).expect("Overwrite should succeed");

        assert_eq!(fs::read(&target_file).unwrap(), updated_data);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    /// Open `path` with no sharing, the way an antivirus scanner or indexer briefly
    /// holds a freshly written file; MoveFileExW then fails with
    /// ERROR_SHARING_VIOLATION (32) or ERROR_ACCESS_DENIED (5).
    #[cfg(target_os = "windows")]
    fn hold_exclusively(path: &Path) -> File {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .expect("test must be able to lock the target")
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_write_atomic_retries_through_a_transient_windows_lock() {
        // Upstream 90241980 (#3881): a short-lived lock on the target must not
        // turn a config/account save into a hard failure.
        let temp_dir = std::env::temp_dir().join(format!("test_atomic_lock_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("config.json");
        write_atomic(&target_file, b"old").unwrap();

        // The holder takes the lock, reports that it holds it, then releases it
        // well inside the ~155 ms retry window.
        let (held_tx, held_rx) = std::sync::mpsc::channel();
        let lock_path = target_file.clone();
        let releaser = std::thread::spawn(move || {
            let lock = hold_exclusively(&lock_path);
            held_tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
            drop(lock);
        });
        held_rx.recv().unwrap();

        let result = write_atomic(&target_file, b"new");
        releaser.join().unwrap();
        assert!(
            result.is_ok(),
            "transient lock should be retried: {:?}",
            result
        );
        assert_eq!(fs::read(&target_file).unwrap(), b"new");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_write_atomic_gives_up_on_a_persistent_lock_and_leaves_no_temp_file() {
        let temp_dir = std::env::temp_dir().join(format!("test_atomic_stuck_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("config.json");
        write_atomic(&target_file, b"old").unwrap();

        let lock = hold_exclusively(&target_file);
        let started = std::time::Instant::now();
        let result = write_atomic(&target_file, b"new");
        let elapsed = started.elapsed();
        drop(lock);

        assert!(result.is_err(), "a lock that never clears must still fail");
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "retries must be bounded"
        );
        assert_eq!(fs::read(&target_file).unwrap(), b"old");
        // An antivirus scan can delay the delete of the temp file for a moment;
        // allow a short grace period before calling it a leftover.
        let list_leftovers = || -> Vec<_> {
            fs::read_dir(&temp_dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
                .map(|e| e.file_name())
                .collect()
        };
        let mut leftovers = list_leftovers();
        for _ in 0..25 {
            if leftovers.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
            leftovers = list_leftovers();
        }
        assert!(
            leftovers.is_empty(),
            "temp file left behind: {:?}",
            leftovers
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_write_atomic_survives_partial_tmp() {
        let temp_dir = std::env::temp_dir().join(format!("test_atomic_partial_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let target_file = temp_dir.join("accounts.json");
        let good_data = b"{\"valid\":true}";
        write_atomic(&target_file, good_data).expect("Initial write should succeed");

        // Simulate leftover half-written temp file from a simulated crash / power loss
        let leftover_tmp = temp_dir.join("accounts.json.tmp.aborted");
        fs::write(&leftover_tmp, b"{\"valid\":false, truncated...").unwrap();

        // Target file should remain intact and valid
        assert_eq!(fs::read(&target_file).unwrap(), good_data);

        // Next write should succeed cleanly
        let next_data = b"{\"valid\":true,\"generation\":2}";
        write_atomic(&target_file, next_data).expect("Subsequent write should succeed");
        assert_eq!(fs::read(&target_file).unwrap(), next_data);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
