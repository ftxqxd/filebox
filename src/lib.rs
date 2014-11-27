#![feature(unsafe_destructor)]

//! A box that writes to a file instead of memory.
//!
//! # Example
//!
//! ```rust
//! extern crate filebox;
//! 
//! use filebox::FileBox;
//!
//! fn main() {
//!     let path = Path::new("target/filebox.box");
//!     {
//!         let mut db = FileBox::open_new(&path, 15i).unwrap();
//!         // The number 15 is now stored in the file "target/filebox.box"
//!         // and can be retrieved later
//!         *db += 2;
//!     }
//!     let db: FileBox<int> = FileBox::open(&path).unwrap();
//!     println!("{}", *db);
//! }
//! ```

extern crate serialize;
extern crate bincode;

use std::default::Default;
use std::io::{mod, fs, File, IoError, IoResult, BufferedReader, MemWriter};
use std::io::fs::PathExtensions;
use std::fmt::{mod, Show, Formatter};
use serialize::{Decodable, Encodable};
use bincode::{DecoderReader, EncoderWriter};

/// A box that writes to a file when dropped, and reads from a file when created.
pub struct FileBox<T> {
    _file: File,
    _val: T,
}

impl<'a, T> FileBox<T> where T: Decodable<DecoderReader<'a, BufferedReader<File>>, IoError>
                              + Encodable<EncoderWriter<'a, MemWriter>, IoError> {
    /// Creates a new `FileBox` at the given path with the given value. If the file at the path is
    /// not empty, it will be overwritten.
    pub fn open_new(p: &Path, val: T) -> IoResult<FileBox<T>> {
        Ok(FileBox {
            _file: try!(File::open_mode(p, io::Truncate, io::Write)),
            _val: val,
        })
    }

    /// Opens a `FileBox` from a path, reading the data stored inside. This will fail if the file
    /// cannot be read or the file contains invalid data.
    pub fn open(p: &Path) -> IoResult<FileBox<T>> {
        let f = try!(File::open_mode(p, io::Open, io::Read));
        let val = try!(bincode::decode_from(&mut BufferedReader::new(f)));
        let f = try!(File::open_mode(p, io::Truncate, io::Write));
        Ok(FileBox {
            _file: f,
            _val: val,
        })
    }

    /// Deletes a `FileBox`, deleting the file it is stored in. Returns the result of deleting the
    /// file.
    pub fn delete(self) -> IoResult<()> {
        fs::unlink(self._file.path())
    }
}

impl<'a, T> FileBox<T> where T: Decodable<DecoderReader<'a, BufferedReader<File>>, IoError>
                              + Encodable<EncoderWriter<'a, MemWriter>, IoError>
                              + Default {
    /// Creates a new `FileBox` at the given path with its default value.
    pub fn new(p: &Path) -> IoResult<FileBox<T>> {
        FileBox::open_new(p, Default::default())
    }

    /// Opens a `FileBox` from a path, creating a new one with a default value if the file doesn’t
    /// exist.
    pub fn open_or_new(p: &Path) -> IoResult<FileBox<T>> {
        if p.exists() {
            FileBox::open(p)
        } else {
            FileBox::new(p)
        }
    }
}

impl<T> Deref<T> for FileBox<T> {
    fn deref(&self) -> &T {
        &self._val
    }
}

impl<T> DerefMut<T> for FileBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self._val
    }
}

#[unsafe_destructor]
impl<'a, T> Drop for FileBox<T> where T: Encodable<EncoderWriter<'a, MemWriter>, IoError> {
    fn drop(&mut self) {
        // TODO: decide what this should do if the file can’t be written to
        self._file.write(bincode::encode(&self._val).unwrap().as_slice())
            .ok().expect("could not write to file");
    }
}

impl<T> Show for FileBox<T> where T: Show {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self._val.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::FileBox;

    #[test]
    fn write_then_read() {
        let path = Path::new("target/write_then_read");
        {
            let mut x: FileBox<int> = FileBox::open_new(&path, 10i).unwrap();
            *x += 1i;
        }
        let x: FileBox<int> = FileBox::open(&path).unwrap();
        assert_eq!(*x, 11);
    }

    #[test]
    fn complex_type() {
        let path = Path::new("target/complex_type");
        #[deriving(Encodable, Decodable, Default, PartialEq, Show)]
        struct Foo {
            x: String,
            y: (int, f64),
        }
        {
            let mut x: FileBox<Foo> = FileBox::new(&path).unwrap();
            *x.y.mut0() += 13;
            *x.y.mut1() -= 3.2;
            x.x.push_str("foo bar");
        }
        let x: FileBox<Foo> = FileBox::open(&path).unwrap();
        assert_eq!(*x, Foo { x: "foo bar".to_string(), y: (13, -3.2) });
    }

    #[test]
    fn delete_box() {
        let path = Path::new("target/delete_box");
        let x: FileBox<int> = FileBox::new(&path).unwrap();
        x.delete().unwrap();
        match FileBox::<int>::open(&path) {
            Ok(_) => panic!("opened the file which should be deleted"),
            Err(_) => {},
        }
    }

    #[test]
    fn show() {
        let path = Path::new("target/show");
        let x: FileBox<int> = FileBox::open_new(&path, 1).unwrap();
        assert_eq!(format!("{}", x), "1".to_string());

        let x: FileBox<Box<Vec<int>>> = FileBox::open_new(&path, box vec![1, 2, 3]).unwrap();
        assert_eq!(format!("{}", x), "[1, 2, 3]".to_string());
    }
}
