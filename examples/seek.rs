use std::io::{self, Read, Seek, SeekFrom};

use multi_readers::{join_seek_readers, SliceReader};


fn main() -> io::Result<()> {
    let f = SliceReader::new(b"Hello Rust!");
    let f2 = SliceReader::new(b"MultiSeekReaders!");
    let mut reader = join_seek_readers!(f, f2).unwrap();
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    assert_eq!("Hello Rust!MultiSeekReaders!", &buf);
    reader.seek(SeekFrom::Start(11))?;
    buf.clear();
    reader.read_to_string(&mut buf)?;
    assert_eq!("MultiSeekReaders!", &buf);
    Ok(())
}