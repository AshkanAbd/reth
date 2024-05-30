//! Wrapper for `std::fs` methods

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Result alias for [FsPathError].
pub type Result<T> = std::result::Result<T, FsPathError>;

/// Various error variants for `std::fs` operations that serve as an addition to the io::Error which
/// does not provide any information about the path.
#[derive(Debug, thiserror::Error)]
pub enum FsPathError {
    /// Error variant for failed write operation with additional path context.
    #[error("failed to write to {path:?}: {source}")]
    Write {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed read operation with additional path context.
    #[error("failed to read from {path:?}: {source}")]
    Read {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed read link operation with additional path context.
    #[error("failed to read from {path:?}: {source}")]
    ReadLink {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed file creation operation with additional path context.
    #[error("failed to create file {path:?}: {source}")]
    CreateFile {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed file removal operation with additional path context.
    #[error("failed to remove file {path:?}: {source}")]
    RemoveFile {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed directory creation operation with additional path context.
    #[error("failed to create dir {path:?}: {source}")]
    CreateDir {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed directory removal operation with additional path context.
    #[error("failed to remove dir {path:?}: {source}")]
    RemoveDir {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed directory read operation with additional path context.
    #[error("failed to read dir {path:?}: {source}")]
    ReadDir {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed file renaming operation with additional path context.
    #[error("failed to rename {from:?} to {to:?}: {source}")]
    Rename {
        /// The source `io::Error`.
        source: io::Error,
        /// The original path.
        from: PathBuf,
        /// The target path.
        to: PathBuf,
    },

    /// Error variant for failed file opening operation with additional path context.
    #[error("failed to open file {path:?}: {source}")]
    Open {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed file read as JSON operation with additional path context.
    #[error("failed to parse JSON file {path:?}: {source}")]
    ReadJson {
        /// The source `serde_json::Error`.
        source: serde_json::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed JSON write to file operation with additional path context.
    #[error("failed to write to JSON file {path:?}: {source}")]
    WriteJson {
        /// The source `serde_json::Error`.
        source: serde_json::Error,
        /// The path related to the operation.
        path: PathBuf,
    },

    /// Error variant for failed file metadata operation with additional path context.
    #[error("failed to get metadata for {path:?}: {source}")]
    Metadata {
        /// The source `io::Error`.
        source: io::Error,
        /// The path related to the operation.
        path: PathBuf,
    },
}

impl FsPathError {
    /// Returns the complementary error variant for [`std::fs::write`].
    pub fn write(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Write { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::read`].
    pub fn read(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Read { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::read_link`].
    pub fn read_link(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::ReadLink { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::File::create`].
    pub fn create_file(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::CreateFile { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::remove_file`].
    pub fn remove_file(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::RemoveFile { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::create_dir`].
    pub fn create_dir(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::CreateDir { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::remove_dir`].
    pub fn remove_dir(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::RemoveDir { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::read_dir`].
    pub fn read_dir(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::ReadDir { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::File::open`].
    pub fn open(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Open { source, path: path.into() }
    }

    /// Returns the complementary error variant for [`std::fs::rename`].
    pub fn rename(source: io::Error, from: impl Into<PathBuf>, to: impl Into<PathBuf>) -> Self {
        Self::Rename { source, from: from.into(), to: to.into() }
    }

    /// Returns the complementary error variant for [`std::fs::File::metadata`].
    pub fn metadata(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Metadata { source, path: path.into() }
    }
}

/// Wrapper for `std::fs::read_to_string`
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|err| FsPathError::read(err, path))
}

/// Read the entire contents of a file into a bytes vector.
///
/// Wrapper for `std::fs::read`
pub fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path = path.as_ref();
    fs::read(path).map_err(|err| FsPathError::read(err, path))
}

/// Wrapper for `std::fs::write`
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();
    fs::write(path, contents).map_err(|err| FsPathError::write(err, path))
}

/// Reads the JSON file and deserialize it into the provided type.
pub fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    // Read the file into a byte array first.
    // https://github.com/serde-rs/json/issues/160
    let b = read(path)?;
    serde_json::from_slice(&b).map_err(|source| FsPathError::ReadJson { source, path: path.into() })
}

/// Writes the object as a JSON object.
pub fn write_json_file<T: Serialize>(path: &Path, obj: &T) -> Result<()> {
    let file = create_file(path)?;
    let mut writer = io::BufWriter::new(file);
    serde_json::to_writer(&mut writer, obj)
        .map_err(|source| FsPathError::WriteJson { source, path: path.into() })?;
    writer.flush().map_err(|e| FsPathError::write(e, path))
}

/// Wrapper for [`File::create`].
pub fn create_file(path: impl AsRef<Path>) -> Result<fs::File> {
    let path = path.as_ref();
    fs::File::create(path).map_err(|err| FsPathError::create_file(err, path))
}

/// Wrapper for `std::fs::remove_file`
pub fn remove_file(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    fs::remove_file(path).map_err(|err| FsPathError::remove_file(err, path))
}

/// Wrapper for `std::fs::create_dir_all`
pub fn create_dir_all(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    fs::create_dir_all(path).map_err(|err| FsPathError::create_dir(err, path))
}

/// Wrapper for `std::fs::remove_dir_all`
pub fn remove_dir_all(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    fs::remove_dir_all(path).map_err(|err| FsPathError::remove_dir(err, path))
}

/// Wrapper for `std::fs::read_dir`
pub fn read_dir(path: impl AsRef<Path>) -> Result<fs::ReadDir> {
    let path = path.as_ref();
    fs::read_dir(path).map_err(|err| FsPathError::read_dir(err, path))
}

/// Wrapper for `std::fs::rename`
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    fs::rename(from, to).map_err(|err| FsPathError::rename(err, from, to))
}

/// Wrapper for `std::fs::metadata`
pub fn metadata(path: impl AsRef<Path>) -> Result<fs::Metadata> {
    let path = path.as_ref();
    fs::metadata(path).map_err(|err| FsPathError::metadata(err, path))
}
