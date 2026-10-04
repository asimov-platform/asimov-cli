// This is free and unencumbered software released into the public domain.

use std::{fs::File, io, path::Path};

/// Writes a private temporary file beside the destination, then replaces it.
/// A failed write leaves the previous destination intact and removes the temp.
pub(crate) fn atomic_write(
    path: &Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    write(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn failed_writes_preserve_the_destination_and_remove_temporary_files() -> io::Result<()> {
        let root = temp_dir::TempDir::new()?;
        let path = root.child("value");
        std::fs::write(&path, b"old value")?;
        let result = atomic_write(&path, |file| {
            file.write_all(b"partial replacement")?;
            Err(io::Error::other("injected write failure"))
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path)?, b"old value");
        assert_eq!(std::fs::read_dir(root.path())?.count(), 1);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn temporary_and_final_files_are_private() -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let root = temp_dir::TempDir::new()?;
        let path = root.child("value");
        atomic_write(&path, |file| {
            assert_eq!(file.metadata()?.permissions().mode() & 0o777, 0o600);
            file.write_all(b"secret")
        })?;
        assert_eq!(
            std::fs::metadata(&path)?.permissions().mode() & 0o777,
            0o600
        );
        Ok(())
    }

    #[test]
    fn failed_replacement_cleans_up_without_modifying_the_destination() -> io::Result<()> {
        let root = temp_dir::TempDir::new()?;
        let destination = root.child("directory");
        std::fs::create_dir(&destination)?;
        std::fs::write(destination.join("keep"), b"old")?;
        assert!(atomic_write(&destination, |file| file.write_all(b"new")).is_err());
        assert_eq!(std::fs::read(destination.join("keep"))?, b"old");
        assert_eq!(std::fs::read_dir(root.path())?.count(), 1);
        Ok(())
    }
}
