use crate::lsif::Element;
use std::io::BufRead;

/// An iterator that streams LSIF elements from an NDJSON (JSONLines) reader.
pub struct LsifReader<R: BufRead> {
    reader: R,
}

impl<R: BufRead> LsifReader<R> {
    /// Create a new streaming LSIF reader from a `BufRead`.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: BufRead> Iterator for LsifReader<R> {
    type Item = Result<Element, serde_json::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        loop {
            line.clear();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    return Some(serde_json::from_str(trimmed));
                }
                Err(e) => {
                    // Convert io::Error to serde_json::Error to match interface
                    let io_err = serde_json::Error::io(e);
                    return Some(Err(io_err));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsif::{Id, Vertex};
    use std::io::Cursor;

    #[test]
    fn test_lsif_reader() {
        let data = "{\"label\":\"resultSet\",\"id\":1,\"type\":\"vertex\"}\n\n   \n{\"label\":\"resultSet\",\"id\":2,\"type\":\"vertex\"}\n";
        let cursor = Cursor::new(data);
        let mut reader = LsifReader::new(cursor);

        let el1 = reader.next().unwrap().unwrap();
        if let Element::Vertex(Vertex::ResultSet { id, type_: _ }) = el1 {
            assert_eq!(id, Id::Number(1));
        } else {
            panic!("Expected ResultSet");
        }

        let el2 = reader.next().unwrap().unwrap();
        if let Element::Vertex(Vertex::ResultSet { id, type_: _ }) = el2 {
            assert_eq!(id, Id::Number(2));
        } else {
            panic!("Expected ResultSet");
        }

        assert!(reader.next().is_none());
    }

    #[test]
    fn test_lsif_reader_invalid_json() {
        let data = "{\"label\":\"resultSet\",\"id\":1,\"type\":\"vertex\"}\n{invalid_json}";
        let cursor = Cursor::new(data);
        let mut reader = LsifReader::new(cursor);

        assert!(reader.next().unwrap().is_ok());
        assert!(reader.next().unwrap().is_err());
    }
}
