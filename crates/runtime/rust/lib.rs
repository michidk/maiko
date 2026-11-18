//! Host services shared by the statically translated Redguard executable.
pub mod audio;
pub mod crash;
pub mod dialog;
pub mod files;
pub mod guest;
pub mod host;
pub mod input;
pub mod install;
pub mod installed_media;
pub mod llasm;
pub mod mixer;
pub use maiko_launcher::locate;
pub mod logging;
pub mod media;
pub mod memory;
pub mod movies;
mod os;
pub mod platform;
pub mod redbook;
pub mod storage;
pub mod stream;
pub mod timing;

/// Lock a mutex, recovering the data if a panicking thread poisoned it: the runtime's state
/// stays usable for shutdown and error reporting after a failed service.
pub(crate) trait Locked<T> {
    fn locked(&self) -> std::sync::MutexGuard<'_, T>;
}
impl<T> Locked<T> for std::sync::Mutex<T> {
    fn locked(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
