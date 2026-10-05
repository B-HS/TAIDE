use std::{
    fs::File,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::path::Component;

use taide_model::error::{AppError, AppResult};

#[derive(Debug)]
pub(crate) struct Anchor {
    pub path: PathBuf,
    #[cfg(unix)]
    directory: File,
    #[cfg(unix)]
    identity: (u64, u64),
}

impl Anchor {
    #[cfg(unix)]
    pub fn open(path: PathBuf) -> AppResult<Self> {
        use rustix::fs::{Mode, OFlags};
        use std::os::unix::fs::MetadataExt;

        let directory = File::from(
            rustix::fs::open(
                &path,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(std::io::Error::from)?,
        );
        let metadata = directory.metadata()?;
        let identity = (metadata.dev(), metadata.ino());
        let anchor = Self {
            path,
            directory,
            identity,
        };
        anchor.check()?;
        Ok(anchor)
    }

    #[cfg(not(unix))]
    pub fn open(_path: PathBuf) -> AppResult<Self> {
        Err(AppError::Forbidden(
            "platform resource anchor is not implemented".into(),
        ))
    }

    #[cfg(unix)]
    pub fn check(&self) -> AppResult<()> {
        use std::os::unix::fs::MetadataExt;
        let current = std::fs::metadata(&self.path)?;
        if !current.is_dir() || (current.dev(), current.ino()) != self.identity {
            return Err(AppError::Forbidden(
                "preview resource root was replaced".into(),
            ));
        }
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn check(&self) -> AppResult<()> {
        Err(AppError::Forbidden(
            "platform resource anchor is not implemented".into(),
        ))
    }

    #[cfg(unix)]
    pub fn file(&self, canonical: &Path) -> AppResult<File> {
        use rustix::fs::{Mode, OFlags};
        self.check()?;
        let relative = canonical
            .strip_prefix(&self.path)
            .map_err(|_| AppError::Forbidden("resource is outside its source root".into()))?;
        let mut components = relative.components().peekable();
        let mut current = self.directory.try_clone()?;
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(AppError::Forbidden(
                    "resource path contains non-normal components".into(),
                ));
            };
            let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
            if components.peek().is_some() {
                flags |= OFlags::DIRECTORY;
            }
            current = File::from(
                rustix::fs::openat(&current, name, flags, Mode::empty())
                    .map_err(std::io::Error::from)?,
            );
        }
        if !current.metadata()?.is_file() {
            return Err(AppError::Forbidden(
                "resource must be a regular file".into(),
            ));
        }
        self.check()?;
        Ok(current)
    }

    #[cfg(not(unix))]
    pub fn file(&self, _canonical: &Path) -> AppResult<File> {
        Err(AppError::Forbidden(
            "platform resource anchor is not implemented".into(),
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stamp {
    length: u64,
    modified: std::time::SystemTime,
    #[cfg(unix)]
    identity: (u64, u64),
    #[cfg(unix)]
    changed: (i64, i64),
}

impl Stamp {
    pub fn read(file: &File) -> AppResult<Self> {
        let metadata = file.metadata()?;
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        if !metadata.is_file() {
            return Err(AppError::Forbidden(
                "resource must be a regular file".into(),
            ));
        }
        Ok(Self {
            length: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(unix)]
            identity: (metadata.dev(), metadata.ino()),
            #[cfg(unix)]
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn web_anchor는_root_component_교체_symlink와_fifo를_거절한다() {
        let fixture = Fixture(std::env::temp_dir().join(format!(
            "taide-web-anchor-{}",
            taide_model::ids::ProjectId::new()
        )));
        let root = fixture.0.join("root");
        let sub = root.join("sub");
        let outside = fixture.0.join("outside");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let source = sub.join("asset.png");
        std::fs::write(&source, b"approved synthetic bytes").unwrap();
        std::fs::write(outside.join("asset.png"), b"outside synthetic bytes").unwrap();
        let anchor = Anchor::open(root.clone()).unwrap();
        assert!(anchor.file(&source).is_ok());
        std::fs::rename(&sub, root.join("old-sub")).unwrap();
        std::os::unix::fs::symlink(&outside, &sub).unwrap();
        assert!(anchor.file(&source).is_err());
        assert!(
            std::process::Command::new("/usr/bin/mkfifo")
                .arg(root.join("pipe.png"))
                .status()
                .unwrap()
                .success()
        );
        assert!(anchor.file(&root.join("pipe.png")).is_err());
        std::fs::rename(&root, fixture.0.join("old-root")).unwrap();
        std::os::unix::fs::symlink(&outside, &root).unwrap();
        assert!(anchor.file(&root.join("asset.png")).is_err());
        assert!(anchor.check().is_err());
    }
}
