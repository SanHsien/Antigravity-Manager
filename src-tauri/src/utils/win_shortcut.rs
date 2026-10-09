use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Foundation::RPC_E_CHANGED_MODE,
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED, STGM_READWRITE,
        },
        UI::Shell::{IShellLinkW, SHChangeNotify, ShellLink, SHCNE_ASSOCCHANGED, SHCNF_IDLIST},
    },
};

struct ComApartment {
    owns_initialization: bool,
    // COM initialization and its balancing uninitialization belong to this thread.
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl ComApartment {
    fn initialize() -> Option<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if result.is_err() && result != RPC_E_CHANGED_MODE {
            return None;
        }
        Some(Self {
            owns_initialization: result.is_ok(),
            _thread: std::marker::PhantomData,
        })
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.owns_initialization {
            unsafe { CoUninitialize() };
        }
    }
}

/// Repair shortcut icons with typed native COM interfaces, without external processes.
pub fn heal_shortcuts_native() {
    let Some(_apartment) = ComApartment::initialize() else {
        return;
    };
    let mut target_dirs = Vec::new();
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        target_dirs.push(PathBuf::from(&userprofile).join("Desktop"));
    }
    if let Ok(public) = std::env::var("PUBLIC") {
        target_dirs.push(PathBuf::from(&public).join("Desktop"));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        target_dirs.push(PathBuf::from(&appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(programdata) = std::env::var("ProgramData") {
        target_dirs
            .push(PathBuf::from(&programdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }

    let mut changed = false;
    for base_dir in target_dirs {
        if base_dir.exists() {
            scan_and_heal_dir(&base_dir, &mut changed);
        }
    }
    if changed {
        unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
    }
}

fn scan_and_heal_dir(dir: &Path, changed: &mut bool) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_and_heal_dir(&path, changed);
            } else if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.to_lowercase().contains("antigravity")
                        && file_name.to_lowercase().ends_with(".lnk")
                        && heal_single_shortcut(&path)
                    {
                        *changed = true;
                    }
                }
            }
        }
    }
}

fn wide_path(path: &OsStr) -> Vec<u16> {
    path.encode_wide().chain(Some(0)).collect()
}

fn wide_buffer_string(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end]).trim().to_string()
}

fn heal_single_shortcut(lnk_path: &Path) -> bool {
    // Wide strings remain alive and NUL-terminated for their calls. The interfaces
    // own COM references; there are no raw vtables or manual releases.
    unsafe {
        let shell_link: IShellLinkW = match CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
        {
            Ok(link) => link,
            Err(_) => return false,
        };
        let persist_file: IPersistFile = match shell_link.cast() {
            Ok(file) => file,
            Err(_) => return false,
        };
        let lnk_path_wide = wide_path(lnk_path.as_os_str());
        if persist_file
            .Load(PCWSTR(lnk_path_wide.as_ptr()), STGM_READWRITE)
            .is_err()
        {
            return false;
        }

        let mut target_buf = [0u16; 512];
        let mut icon_buf = [0u16; 512];
        let mut icon_index = 0;
        if shell_link
            .GetPath(&mut target_buf, std::ptr::null_mut(), 0)
            .is_err()
            || shell_link
                .GetIconLocation(&mut icon_buf, &mut icon_index)
                .is_err()
        {
            return false;
        }
        let target = wide_buffer_string(&target_buf);
        let icon = wide_buffer_string(&icon_buf);
        if !(icon.is_empty() || icon == ",0") || !target.to_lowercase().ends_with(".exe") {
            return false;
        }

        let target_wide = wide_path(OsStr::new(&target));
        shell_link
            .SetIconLocation(PCWSTR(target_wide.as_ptr()), 0)
            .is_ok()
            && persist_file
                .Save(PCWSTR(lnk_path_wide.as_ptr()), true)
                .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_only_missing_icon_in_temporary_shortcut() {
        let _apartment = ComApartment::initialize().expect("COM initialization");
        let directory = tempfile::tempdir().unwrap();
        let shortcut = directory.path().join("Antigravity-fixture.lnk");
        let executable = std::env::current_exe().unwrap();
        let shortcut_wide = wide_path(shortcut.as_os_str());
        let executable_wide = wide_path(executable.as_os_str());
        unsafe {
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(PCWSTR(executable_wide.as_ptr())).unwrap();
            let empty = [0u16];
            link.SetIconLocation(PCWSTR(empty.as_ptr()), 0).unwrap();
            let file: IPersistFile = link.cast().unwrap();
            file.Save(PCWSTR(shortcut_wide.as_ptr()), true).unwrap();
        }
        assert!(heal_single_shortcut(&shortcut));
        assert!(!heal_single_shortcut(&shortcut));
        assert!(!heal_single_shortcut(&directory.path().join("missing.lnk")));
        unsafe {
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            let file: IPersistFile = link.cast().unwrap();
            file.Load(PCWSTR(shortcut_wide.as_ptr()), STGM_READWRITE)
                .unwrap();
            let mut icon = [0u16; 512];
            let mut index = -1;
            link.GetIconLocation(&mut icon, &mut index).unwrap();
            assert_eq!(wide_buffer_string(&icon), executable.to_string_lossy());
            assert_eq!(index, 0);
        }
    }
}
