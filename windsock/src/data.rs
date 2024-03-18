use std::path::PathBuf;
use std::sync::OnceLock;

static INITIALIZED_PATH: OnceLock<()> = OnceLock::new();

pub fn windsock_path() -> PathBuf {
    let path = windsock_path_inner();

    // Create the directory only the first time this function is called during program execution.
    if INITIALIZED_PATH.set(()).is_ok() {
        std::fs::create_dir_all(&path).unwrap();
    }

    path
}

fn windsock_path_inner() -> PathBuf {
    // If we are run via cargo (we are in a target directory) use the target directory for storage.
    // Otherwise just fallback to the current working directory.
    let mut path = std::env::current_exe().unwrap();
    while path.pop() {
        if path.file_name().map(|x| x == "target").unwrap_or(false) {
            return path.join("windsock_data");
        }
    }

    PathBuf::from("windsock_data")
}

pub fn cloud_resources_path() -> PathBuf {
    windsock_path().join("cloud_resources")
}
