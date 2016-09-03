use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::Write;

mod onedrive;
pub use self::onedrive::OneDriveClient;

pub trait ObjectStore {
    type Reader: Read + Seek;
    type Writer: Write;
    type ObjectIterator: Iterator<Item = String>;

    fn open(&self, name: AsRef<str>) -> Result<Self::Reader>;
    fn create(&self, name: AsRef<str>) -> Result<Self::Writer>;
    fn remove(&self, name: AsRef<str>) -> Result<()>;
    fn list(&self, prefix: AsRef<str>) -> Result<Self::ObjectIterator>;
}
