use std::fs;
use std::path::{Path, PathBuf};

/// Represents a game found by scanning a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedGame {
    /// The display name of the game (folder name).
    pub name: String,
    /// The full path to the main executable (largest .exe in the folder).
    pub path: PathBuf,
    /// The size of the executable in bytes.
    pub size_bytes: u64,
    /// The Steam app ID generated from the game.
    pub app_id: String,
}

/// Recursively scans a folder for game directories and finds the largest .exe in each.
///
/// This approach is similar to the Python script - it looks for game folders
/// and finds the largest executable in each folder (usually the main game).
///
/// # Arguments
/// * `path` - The root folder to scan.
///
/// # Returns
/// Vector of ScannedGame entries with name, path, size, and generated app ID.
pub fn scan_folder(path: &Path) -> Vec<ScannedGame> {
    let mut games = Vec::new();
    
    // Get all directories in the root path
    let read_dir = match fs::read_dir(path) {
        Ok(rd) => rd,
        Err(e) => {
            eprintln!("[scan_folder] Could not read dir {}: {}", path.display(), e);
            return games;
        }
    };
    
    for entry in read_dir.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            // Skip system folders and redistributables
            let folder_name = entry_path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            let skip_dirs = ["redist", "directx", "dotnet", "vcredist", "_commonredist", 
                           "system32", "windows", "program files", "program files (x86)"];
            
            if skip_dirs.iter().any(|d| folder_name.contains(d)) {
                continue;
            }
            
            // Find the largest .exe file in this game folder
            if let Some(largest_exe) = find_largest_exe(&entry_path) {
                if let Ok(metadata) = fs::metadata(&largest_exe) {
                    let name = entry_path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let app_id = generate_app_id(&name, &largest_exe);
                    
                    games.push(ScannedGame {
                        name,
                        path: largest_exe,
                        size_bytes: metadata.len(),
                        app_id,
                    });
                }
            }
        }
    }
    
    games
}

/// Find the largest .exe file in a directory (recursively).
fn find_largest_exe(dir: &Path) -> Option<PathBuf> {
    let mut largest_file = None;
    let mut largest_size = 0u64;
    
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recursively search subdirectories
                if let Some(sub_largest) = find_largest_exe(&path) {
                    if let Ok(metadata) = fs::metadata(&sub_largest) {
                        if metadata.len() > largest_size {
                            largest_size = metadata.len();
                            largest_file = Some(sub_largest);
                        }
                    }
                }
            } else if is_executable(&path) {
                if let Ok(metadata) = fs::metadata(&path) {
                    // Only consider files larger than 1MB to skip launchers/updaters
                    if metadata.len() > 1024 * 1024 && metadata.len() > largest_size {
                        largest_size = metadata.len();
                        largest_file = Some(path);
                    }
                }
            }
        }
    }
    
    largest_file
}

/// Generate a Steam app ID from game name and executable path (similar to Python script).
fn generate_app_id(game_name: &str, exe_path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let unique_string = format!("{}{}", exe_path.display(), game_name);
    let mut hasher = DefaultHasher::new();
    unique_string.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Convert to signed 32-bit and set the high bit (like Steam does)
    let legacy_id = (hash as u32) | 0x80000000u32;
    legacy_id.to_string()
}

#[cfg(target_os = "windows")]
fn is_executable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn is_executable(path: &Path) -> bool {
    use std::fs::File;
    use std::io::Read;
    if let Ok(mut file) = File::open(path) {
        let mut magic = [0u8; 4];
        if file.read_exact(&mut magic).is_ok() {
            // ELF magic bytes: 0x7F 'E' 'L' 'F'
            return magic == [0x7F, b'E', b'L', b'F'];
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_scan_folder_empty() {
        let dir = tempdir().unwrap();
        let games = scan_folder(dir.path());
        assert!(games.is_empty());
    }

    #[test]
    fn test_scan_folder_with_game_dirs() {
        let dir = tempdir().unwrap();
        
        // Create a game directory with multiple .exe files
        let game_dir = dir.path().join("TestGame");
        fs::create_dir(&game_dir).unwrap();
        
        // Create a small launcher .exe
        let launcher = game_dir.join("launcher.exe");
        File::create(&launcher).unwrap().write_all(b"small exe").unwrap();
        
        // Create a larger main game .exe
        let main_game = game_dir.join("game.exe");
        let mut main_file = File::create(&main_game).unwrap();
        main_file.write_all(&vec![0u8; 1024 * 1024]).unwrap(); // 1MB file
        
        let games = scan_folder(dir.path());
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "TestGame");
        assert_eq!(games[0].path, main_game);
    }
}