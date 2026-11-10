//! Replacing a file's contents atomically: written to a temporary file beside
//! it, synced, then renamed over it, so a crash leaves either the old file or
//! the new one, never half of either.
//!
//! Errors are the bare OS errors, with no temporary path in them, so a
//! failed keystore save reads as the reference crate's does:
//! `Couldn't save keystore to <path>: No such file or directory (os error 2)`.

use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Mode for a file this module creates. An existing file keeps its own.
#[cfg(unix)]
const NEW_FILE_MODE: u32 = 0o600;

/// Hops followed before a symlink chain is treated as a loop, as `ELOOP` does.
const MAX_SYMLINK_HOPS: usize = 40;

/// Names tried for a temporary file before giving up on collisions.
const TEMP_FILE_ATTEMPTS: u32 = 16;

/// Distinguishes the temporary files of concurrent writes within one process.
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) trait AtomicWrite {
    /// Replace the file at this path with `contents`. A symlinked path is
    /// followed to the file a plain write would reach and the link is kept;
    /// an existing file keeps its mode, and a new one is owner-only on unix.
    fn write_atomically(&self, contents: &[u8]) -> io::Result<()>;
}

impl AtomicWrite for Path {
    fn write_atomically(&self, contents: &[u8]) -> io::Result<()> {
        let target = self.resolve_symlinks()?;
        let dir = target.containing_dir();
        let existing_permissions = target.existing_permissions()?;

        let (temp_path, mut temp) = target.create_temp_sibling()?;
        let written = temp
            .write_synced(contents, existing_permissions)
            .and_then(|()| fs::rename(&temp_path, &target));
        if let Err(error) = written {
            return Err(temp_path.remove_after_failure(error));
        }
        return dir.sync_dir();
    }
}

/// The steps of [`AtomicWrite::write_atomically`].
trait WriteSteps {
    /// Follow any symlinks to the file a plain write would reach, so the
    /// rename replaces the target rather than the link. A dangling link
    /// resolves to its missing target, which the write then creates.
    fn resolve_symlinks(&self) -> io::Result<PathBuf>;

    /// The directory holding this path, `.` for a bare file name.
    fn containing_dir(&self) -> PathBuf;

    /// The mode of the file at this path, or `None` when there is none yet.
    fn existing_permissions(&self) -> io::Result<Option<fs::Permissions>>;

    /// A new, empty file beside this path, owner-only on unix until an
    /// existing file's mode is copied onto it.
    fn create_temp_sibling(&self) -> io::Result<(PathBuf, fs::File)>;

    /// Delete this temporary file after a failed write, returning the write's
    /// error; a failed delete is reported alongside it, not dropped.
    fn remove_after_failure(&self, error: io::Error) -> io::Error;

    /// Make a rename into this directory durable.
    fn sync_dir(&self) -> io::Result<()>;
}

impl WriteSteps for Path {
    fn resolve_symlinks(&self) -> io::Result<PathBuf> {
        let mut current = self.to_path_buf();
        for _ in 0..MAX_SYMLINK_HOPS {
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    let link = fs::read_link(&current)?;
                    current = match current.parent() {
                        Some(parent) => parent.join(link),
                        None => link,
                    };
                }
                Ok(_) => return Ok(current),
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(current),
                Err(e) => return Err(e),
            }
        }
        return Err(io::Error::other(format!(
            "too many levels of symbolic links: {}",
            self.display()
        )));
    }

    fn containing_dir(&self) -> PathBuf {
        match self.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => return parent.to_path_buf(),
            _ => return PathBuf::from("."),
        }
    }

    fn existing_permissions(&self) -> io::Result<Option<fs::Permissions>> {
        match fs::metadata(self) {
            Ok(metadata) => return Ok(Some(metadata.permissions())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        }
    }

    fn create_temp_sibling(&self) -> io::Result<(PathBuf, fs::File)> {
        let dir = self.containing_dir();
        let name = self
            .file_name()
            .map_or_else(|| "file".into(), |name| name.to_string_lossy());
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(NEW_FILE_MODE);
        }

        let mut attempt = 0;
        loop {
            let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let temp_path = dir.join(format!(".{name}.{}.{sequence}.tmp", std::process::id()));
            match options.open(&temp_path) {
                Ok(file) => return Ok((temp_path, file)),
                // a leftover from an earlier process with the same pid
                Err(e)
                    if e.kind() == io::ErrorKind::AlreadyExists && attempt < TEMP_FILE_ATTEMPTS =>
                {
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn remove_after_failure(&self, error: io::Error) -> io::Error {
        match fs::remove_file(self) {
            Ok(()) => return error,
            Err(cleanup) => {
                return io::Error::new(
                    error.kind(),
                    format!(
                        "{error}; the temporary file {} could not be removed: {cleanup}",
                        self.display()
                    ),
                );
            }
        }
    }

    // directories cannot be opened for syncing outside unix
    fn sync_dir(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            return fs::File::open(self)?.sync_all();
        }
        #[cfg(not(unix))]
        {
            return Ok(());
        }
    }
}

trait SyncedWrite {
    /// Write `contents`, take on `permissions` if given, and sync to disk.
    fn write_synced(
        &mut self,
        contents: &[u8],
        permissions: Option<fs::Permissions>,
    ) -> io::Result<()>;
}

impl SyncedWrite for fs::File {
    fn write_synced(
        &mut self,
        contents: &[u8],
        permissions: Option<fs::Permissions>,
    ) -> io::Result<()> {
        self.write_all(contents)?;
        if let Some(permissions) = permissions {
            self.set_permissions(permissions)?;
        }
        return self.sync_all();
    }
}
