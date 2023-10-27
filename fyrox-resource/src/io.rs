//! Provides an interface for IO operations that a resource loader will use, this facilliates
//! things such as loading assets within archive files

use fyrox_core::io::FileLoadError;
use std::future::{ready, Future};
use std::iter::empty;
use std::pin::Pin;
use std::{
    fmt::Debug,
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Trait for files readers ensuring they implement the required traits
#[cfg(target_arch = "wasm32")]
pub trait FileReader: Debug + Read + Seek + 'static {}

#[cfg(target_arch = "wasm32")]
impl<F> FileReader for F where F: Debug + Read + Seek + 'static {}

/// Trait for files readers ensuring they implement the required traits
#[cfg(not(target_arch = "wasm32"))]
pub trait FileReader: Debug + Send + Read + Seek + 'static {}

#[cfg(not(target_arch = "wasm32"))]
impl<F> FileReader for F where F: Debug + Send + Read + Seek + 'static {}

/// Interface wrapping IO operations for doing this like loading files
/// for resources
pub trait ResourceIo: Send + Sync + 'static {
    /// Attempts to load the file at the provided path returning
    /// the entire byte contents of the file or an error
    fn load_file<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Vec<u8>, FileLoadError>>;

    /// Provides an iterator over the paths present in the provided
    /// path, this should only provide paths immediately within the directory
    ///
    /// Default implementation is no-op returning an empty iterator
    fn read_directory<'a>(
        &'a self,
        #[allow(unused)] path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Box<dyn Iterator<Item = PathBuf> + Send>, FileLoadError>> {
        let iter: Box<dyn Iterator<Item = PathBuf> + Send> = Box::new(empty());
        Box::pin(ready(Ok(iter)))
    }

    /// Provides an iterator over the paths present in the provided
    /// path directory this implementation should walk the directory paths
    ///
    /// Default implementation is no-op returning an empty iterator
    fn walk_directory<'a>(
        &'a self,
        #[allow(unused)] path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Box<dyn Iterator<Item = PathBuf> + Send>, FileLoadError>> {
        let iter: Box<dyn Iterator<Item = PathBuf> + Send> = Box::new(empty());
        Box::pin(ready(Ok(iter)))
    }

    /// Attempts to open a file reader to the proivded path for
    /// reading its bytes
    ///
    /// Default implementation loads the entire file contents from `load_file`
    /// then uses a cursor as the reader
    fn file_reader<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Box<dyn FileReader>, FileLoadError>> {
        Box::pin(async move {
            let bytes = self.load_file(path).await?;
            let read: Box<dyn FileReader> = Box::new(Cursor::new(bytes));
            Ok(read)
        })
    }

    /// Used to check whether a path exists
    fn exists<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool>;

    /// Used to check whether a path is a file
    fn is_file<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool>;

    /// Used to check whether a path is a dir
    fn is_dir<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool>;
}

/// Standard resource IO provider that uses the file system to
/// load the file bytes
#[derive(Default)]
pub struct FsResourceIo;

/// Future for resource io loading
#[cfg(target_arch = "wasm32")]
pub type ResourceIoFuture<'a, V> = Pin<Box<dyn Future<Output = V> + 'a>>;
/// Future for resource io loading
#[cfg(not(target_arch = "wasm32"))]
pub type ResourceIoFuture<'a, V> = Pin<Box<dyn Future<Output = V> + Send + 'a>>;

/// Iterator of paths
#[cfg(target_arch = "wasm32")]
pub type PathIter = Box<dyn Iterator<Item = PathBuf>>;
/// Iterator of paths
#[cfg(not(target_arch = "wasm32"))]
pub type PathIter = Box<dyn Iterator<Item = PathBuf> + Send>;

impl ResourceIo for FsResourceIo {
    fn load_file<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Vec<u8>, FileLoadError>> {
        Box::pin(fyrox_core::io::load_file(path))
    }

    /// Android and wasm should fallback to the default no-op impl as im not sure if they
    /// can directly read a directory
    ///
    /// TODO: Needs an android implementation for reading a directory
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    fn read_directory<'a>(
        &'a self,
        #[allow(unused)] path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<PathIter, FileLoadError>> {
        Box::pin(async move {
            let iter = std::fs::read_dir(path)?.flatten().map(|entry| entry.path());
            let iter: PathIter = Box::new(iter);
            Ok(iter)
        })
    }

    /// Android and wasm should fallback to the default no-op impl as im not sure if they
    /// can be walked
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    fn walk_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<PathIter, FileLoadError>> {
        Box::pin(async move {
            let iter = WalkDir::new(path)
                .into_iter()
                .flatten()
                .map(|value| value.into_path());

            let iter: PathIter = Box::new(iter);

            Ok(iter)
        })
    }

    /// Only use file reader when not targetting android or wasm
    ///
    /// Note: Might be possible to using the android Asset for reading as
    /// long as its Send + Sync + 'static (It already implements Debug + Read + Seek)
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    fn file_reader<'a>(
        &'a self,
        path: &'a Path,
    ) -> ResourceIoFuture<'a, Result<Box<dyn FileReader>, FileLoadError>> {
        Box::pin(async move {
            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) => return Err(FileLoadError::Io(e)),
            };

            let read: Box<dyn FileReader> = Box::new(std::io::BufReader::new(file));
            Ok(read)
        })
    }

    fn exists<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool> {
        Box::pin(fyrox_core::io::exists(path))
    }

    fn is_file<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool> {
        Box::pin(fyrox_core::io::is_file(path))
    }

    fn is_dir<'a>(&'a self, path: &'a Path) -> ResourceIoFuture<'a, bool> {
        Box::pin(fyrox_core::io::is_dir(path))
    }
}
