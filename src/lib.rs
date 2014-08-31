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
//!     let path = Path::new("target/filebox.json");
//!     {
//!         let mut db = FileBox::open_new(&path, 15i);
//!         // The number 15 is now stored in the file "target/filebox.json"
//!         // and can be retrieved later
//!         *db += 2;
//!     }
//!     let db: FileBox<int> = FileBox::open(&path);
//!     println!("{}", *db);
//! }
//! ```

extern crate serialize;

use std::default::Default;
use std::str;
use std::io::{mod, fs, File, IoError, IoResult};
use serialize::{json, Decoder, Decodable, Encoder, Encodable};

/// A box that writes to a file when dropped, and reads from a file when created.
pub struct FileBox<T> {
    f: File,
    _val: T,
}

impl<T> FileBox<T> where T: Decodable<json::Decoder, json::DecoderError> {
    /// Creates a new `FileBox` at the given path with the given value. If the file at the path is
    /// not empty, it will be overwritten.
    pub fn open_new(p: &Path, val: T) -> FileBox<T> {
        FileBox {
            f: File::open_mode(p, io::Truncate, io::Write).unwrap(),
            _val: val,
        }
    }

    /// Opens a `FileBox` from a path, reading the data stored inside. This will fail if the file
    /// cannot be read or the file contains invalid data.
    pub fn open(p: &Path) -> FileBox<T> {
        let mut f = File::open_mode(p, io::Open, io::Read).unwrap();
        let val = json::decode(str::from_utf8(f.read_to_end().unwrap().as_slice()).unwrap())
                    .unwrap();
        let f = File::open_mode(p, io::Truncate, io::Write).unwrap();
        FileBox {
            f: f,
            _val: val,
        }
    }

    /// Deletes a `FileBox`, deleting the file it is stored in. Returns the result of deleting the
    /// file.
    pub fn delete(self) -> IoResult<()> {
        fs::unlink(self.f.path())
    }
}

impl<T> FileBox<T> where T: Decodable<json::Decoder, json::DecoderError> + Default {
    /// Creates a new `FileBox` at the given path with its default value.
    pub fn new(p: &Path) -> FileBox<T> {
        FileBox::open_new(p, Default::default())
    }

    /// Opens a `FileBox` from a path, creating a new one with a default value if the file doesn’t
    /// exist.
    pub fn open_or_new(p: &Path) -> FileBox<T> {
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
impl<'a, T> Drop for FileBox<T> where T: Encodable<json::Encoder<'a>, IoError> {
    fn drop(&mut self) {
        // TODO: decide what this should do if the file can’t be written to
        self.f.write(json::encode(&self._val).as_bytes()).ok().expect("could not write to file");
    }
}

#[cfg(test)]
mod tests {
    use super::FileBox;

    #[test]
    fn write_then_read() {
        let path = Path::new("target/write_then_read");
        {
            let mut x: FileBox<int> = FileBox::open_new(&path, 10i);
            *x += 1i;
        }
        let x: FileBox<int> = FileBox::open(&path);
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
            let mut x: FileBox<Foo> = FileBox::new(&path);
            *x.y.mut0() += 13;
            *x.y.mut1() -= 3.2;
            x.x.push_str("foo bar");
        }
        let x: FileBox<Foo> = FileBox::open(&path);
        assert_eq!(*x, Foo { x: "foo bar".to_string(), y: (13, -3.2) });
    }

    #[test]
    #[should_fail]
    fn delete_box() {
        let path = Path::new("target/delete_box");
        let x: FileBox<int> = FileBox::new(&path);
        match x.delete() {
            Ok(_) => {
                // Here it should fail
                let _: FileBox<int> = FileBox::open(&path);
            },
            // We want to do nothing if deleting the file fails, so that the test fails (or rather,
            // doesn’t)
            Err(_) => {},
        }
    }
}
