use std::path::{Path};
use std::fs;
use std::io;
use tokio::fs::{File};
use tokio::io::{Error, ErrorKind};

pub async fn ensure_dir(dirname: &Path) -> io::Result<()> {
    fs::create_dir_all(dirname)
}

pub async fn file_exists(filepath: &Path) -> bool {
    Path::new(filepath).exists()
}

#[cfg(unix)]
pub async fn open_writeable_file(
    filename: impl AsRef<Path>,
) -> Result<File, tokio::io::Error> {
    // Ensure if the file is created it's only readable and writable by the
    // current user.
    use std::os::unix::fs::OpenOptionsExt;
    let opts: tokio::fs::OpenOptions = {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).mode(0o600);
        opts.into()
    };
    let dirname = match Path::new(filename.as_ref()).parent() {
        Some(dir_parent) => dir_parent,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Unable to determine parent directory path for {}",
                    Path::new(filename.as_ref()).to_str().unwrap(),
                )
            ));
        }
    };

    match fs::create_dir_all(dirname) {
        Ok(_) => {
            opts.open(filename).await
        }
        Err(err) => {
            Err(err)
        }
    }
}

#[cfg(not(unix))]
pub async fn open_writeable_file(
    filename: impl AsRef<Path>,
) -> Result<tokio::fs::File, tokio::io::Error> {
    // I don't have knowledge of windows or other platforms to know how to
    // create a file that's only readable by the current user.
    tokio::fs::File::create(filename).await
}