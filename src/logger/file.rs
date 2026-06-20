use std::{
    fs::{
        File,
        OpenOptions,
    },
    io::{
        Error as IoError,
        Result as IoResult,
        Write,
    },
};

use anyhow::{
    Result,
    bail,
};
use pathkit::{
    Path,
    SyncFsOps,
};

use super::options::LoggerFileRotationOptions;

// Structs
#[derive(Debug)]
pub(super) struct RotatingLogFile {
    backup_count: usize,
    file: Option<File>,
    max_bytes: Option<u64>,
    path: Path,
    written_bytes: u64,
}

impl Write for RotatingLogFile {
    fn flush(&mut self) -> IoResult<()> {
        self.file
            .as_mut()
            .ok_or_else(|| IoError::other("log file is closed"))?
            .flush()
    }

    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.rotate_if_needed(buf.len())?;
        let written_bytes = self
            .file
            .as_mut()
            .ok_or_else(|| IoError::other("log file is closed"))?
            .write(buf)?;

        self.written_bytes = self
            .written_bytes
            .saturating_add(u64::try_from(written_bytes).unwrap_or(u64::MAX));

        Ok(written_bytes)
    }
}

impl RotatingLogFile {
    pub(super) fn new(path: Path, rotation: Option<LoggerFileRotationOptions>) -> Result<Self> {
        path.create_parent_dir_all_sync()?;

        let (max_bytes, backup_count) = match rotation {
            Some(rotation) => {
                if rotation.max_bytes_per_file == 0 {
                    bail!("max_bytes_per_file must be greater than 0 when file log rotation is enabled");
                }

                (Some(rotation.max_bytes_per_file), rotation.backup_file_count)
            },
            None => (None, 0),
        };

        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let written_bytes = path.metadata_sync()?.len();

        Ok(Self {
            backup_count,
            file: Some(file),
            max_bytes,
            path,
            written_bytes,
        })
    }

    // Private methods
    fn remove_file_if_exists(path: &Path) -> Result<()> {
        if path.exists_sync()? {
            path.remove_file_sync()?;
        }

        Ok(())
    }

    fn rotate(&mut self) -> IoResult<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }

        let rotate_result = (|| -> Result<()> {
            if self.backup_count == 0 {
                Self::remove_file_if_exists(&self.path)?;
            } else {
                for index in (1..=self.backup_count).rev() {
                    let current_path = rotated_path(&self.path, index);
                    let next_path = rotated_path(&self.path, index + 1);

                    if current_path.exists_sync()? {
                        if index == self.backup_count {
                            Self::remove_file_if_exists(&current_path)?;
                        } else {
                            current_path.move_to_sync(next_path)?;
                        }
                    }
                }

                if self.path.exists_sync()? {
                    self.path.move_to_sync(rotated_path(&self.path, 1))?;
                }
            }

            Ok(())
        })();

        let file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        self.written_bytes = file.metadata()?.len();
        self.file = Some(file);

        rotate_result.map_err(IoError::other)?;
        self.written_bytes = 0;

        Ok(())
    }

    fn rotate_if_needed(&mut self, incoming_bytes: usize) -> IoResult<()> {
        let Some(max_bytes) = self.max_bytes else {
            return Ok(());
        };

        let incoming_bytes = u64::try_from(incoming_bytes).unwrap_or(u64::MAX);
        if self.written_bytes == 0 || self.written_bytes.saturating_add(incoming_bytes) <= max_bytes {
            return Ok(());
        }

        self.rotate()
    }
}

// Functions
fn rotated_path(path: &Path, index: usize) -> Path {
    path.with_added_extension(index.to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    use anyhow::Result;
    use pathkit::{
        Path,
        SyncFsOps,
        path,
    };

    use super::*;

    #[test]
    fn log_file_without_rotation_appends_and_flushes() -> Result<()> {
        let temp_dir = unique_temp_dir("no-rotation");
        let log_path = &temp_dir / "runtime.log";
        let mut log_file = RotatingLogFile::new(log_path.clone(), None)?;

        log_file.write_all(b"first")?;
        log_file.write_all(b"second")?;
        log_file.flush()?;

        assert_eq!(log_path.read_to_string_sync()?, "firstsecond");
        assert!(!rotated_path(&log_path, 1).exists_sync()?);

        temp_dir.remove_dir_all_sync()?;
        Ok(())
    }

    #[test]
    fn rotation_rejects_zero_max_bytes() {
        let temp_dir = unique_temp_dir("zero-max-bytes");
        let log_path = &temp_dir / "runtime.log";

        let result = RotatingLogFile::new(
            log_path,
            Some(LoggerFileRotationOptions {
                backup_file_count: 1,
                max_bytes_per_file: 0,
            }),
        );

        assert!(result.is_err());
        assert!(
            result
                .err()
                .map(|error| error.to_string().contains("max_bytes_per_file must be greater than 0"))
                .unwrap_or(false)
        );
    }

    #[test]
    fn rotation_without_backups_replaces_active_file() -> Result<()> {
        let temp_dir = unique_temp_dir("no-backups");
        let log_path = &temp_dir / "runtime.log";
        let mut log_file = RotatingLogFile::new(
            log_path.clone(),
            Some(LoggerFileRotationOptions {
                backup_file_count: 0,
                max_bytes_per_file: 4,
            }),
        )?;

        log_file.write_all(b"abcd")?;
        log_file.write_all(b"ef")?;
        log_file.flush()?;

        assert_eq!(log_path.read_to_string_sync()?, "ef");
        assert!(!rotated_path(&log_path, 1).exists_sync()?);

        temp_dir.remove_dir_all_sync()?;
        Ok(())
    }

    #[test]
    fn rotation_shifts_backups_and_caps_backup_count() -> Result<()> {
        let temp_dir = unique_temp_dir("with-backups");
        let log_path = &temp_dir / "runtime.log";
        let mut log_file = RotatingLogFile::new(
            log_path.clone(),
            Some(LoggerFileRotationOptions {
                backup_file_count: 2,
                max_bytes_per_file: 10,
            }),
        )?;

        log_file.write_all(b"1234567890")?;
        log_file.write_all(b"abc")?;
        log_file.write_all(b"defghij")?;
        log_file.write_all(b"klm")?;
        log_file.write_all(b"nopqrst")?;
        log_file.write_all(b"uv")?;
        log_file.flush()?;

        assert_eq!(log_path.read_to_string_sync()?, "uv");
        assert_eq!(rotated_path(&log_path, 1).read_to_string_sync()?, "klmnopqrst");
        assert_eq!(rotated_path(&log_path, 2).read_to_string_sync()?, "abcdefghij");
        assert!(!rotated_path(&log_path, 3).exists_sync()?);

        temp_dir.remove_dir_all_sync()?;
        Ok(())
    }

    fn unique_temp_dir(label: &str) -> Path {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();

        path!(temp_dir()) / format!("kikiutils-logger-file-{label}-{nanos}")
    }
}
