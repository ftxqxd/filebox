#![feature(unsafe_destructor)]

//! A magic box that writes to a file instead of memory.
//!
//! Example:
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
use std::io;
use std::io::{File, IoError};
use serialize::{Decoder, Decodable, Encoder, Encodable};
use serialize::json;

pub struct FileBox<T> {
    f: File,
    _val: T,
}

impl<T> FileBox<T> where T: Decodable<json::Decoder, json::DecoderError> {
    pub fn open_new(p: &Path, val: T) -> FileBox<T> {
        FileBox {
            f: File::open_mode(p, io::Truncate, io::Write).unwrap(),
            _val: val,
        }
    }

    pub fn open(p: &Path) -> FileBox<T> {
        let mut f = File::open_mode(p, io::Open, io::Read).unwrap();
        let val = json::decode(str::from_utf8(f.read_to_end().unwrap().as_slice()).unwrap()).unwrap();
        let f = File::open_mode(p, io::Truncate, io::Write).unwrap();
        FileBox {
            f: f,
            _val: val,
        }
    }
}

impl<T> FileBox<T> where T: Decodable<json::Decoder, json::DecoderError> + Default {
    pub fn new(p: &Path) -> FileBox<T> {
        FileBox::open_new(p, Default::default())
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
        // Err... should this *abort* when the file isn’t there?
        self.f.write(json::encode(&self._val).as_bytes()).ok().expect("could not write to file");
    }
}

#[cfg(test)]
mod tests {
    use super::FileBox;

    #[test]
    fn write_then_read() {
        {
            let mut x: FileBox<int> = FileBox::open_new(&Path::new("target/write_then_read"), 10i);
            *x += 1i;
        }
        let x: FileBox<int> = FileBox::open(&Path::new("target/write_then_read"));
        assert_eq!(*x, 11);
    }

    #[test]
    fn complex_type() {
        #[deriving(Encodable, Decodable, Default, PartialEq, Show)]
        struct Foo {
            x: String,
            y: (int, f64),
        }
        {
            let mut x: FileBox<Foo> = FileBox::new(&Path::new("target/complex_type"));
            *x.y.mut0() += 13;
            *x.y.mut1() -= 3.2;
            x.x.push_str("foo bar");
        }
        let x: FileBox<Foo> = FileBox::open(&Path::new("target/complex_type"));
        assert_eq!(*x, Foo { x: "foo bar".to_string(), y: (13, -3.2) });
    }
}
