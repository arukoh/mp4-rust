use std::io::{Read, Seek};

use serde::Serialize;

use crate::mp4box::meta::MetaBox;
use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct UserDefinedBox {
    pub name: String,
    pub size: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct UdtaBox {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaBox>,
    pub children: Vec<UserDefinedBox>,
}

impl UdtaBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::UdtaBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        if let Some(meta) = &self.meta {
            size += meta.box_size();
        }
        size
    }

    pub fn get_children(&self) -> &Vec<UserDefinedBox> {
        &self.children
    }
}

impl Mp4Box for UdtaBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        Ok(String::new())
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for UdtaBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let mut meta = None;
        let mut children = Vec::new();

        let mut current = reader.stream_position()?;
        let end = start + size;
        while current < end {
            // Get box header.
            let header = BoxHeader::read(reader)?;
            let BoxHeader { name, size: s } = header;
            if s > size {
                return Err(Error::InvalidData(
                    "udta box contains a box with a larger size than it",
                ));
            }

            match name {
                BoxType::MetaBox => {
                    meta = Some(MetaBox::read_box(reader, s)?);
                }
                _ => {
                    // XXX warn!()
                    let mut data = vec![0; (s - 8) as usize];
                    reader.read_exact(&mut data)?;
                    children.push(UserDefinedBox { name: name.to_string(), size: s, data });
                }
            }

            current = reader.stream_position()?;
        }

        skip_bytes_to(reader, start + size)?;

        Ok(UdtaBox { meta, children })
    }
}

impl<W: Write> WriteBox<&mut W> for UdtaBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        if let Some(meta) = &self.meta {
            meta.write_box(writer)?;
        }
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;
    use std::io::Cursor;

    #[test]
    fn test_udta_empty() {
        let src_box = UdtaBox { meta: None, children: Vec::new() };

        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::UdtaBox);
        assert_eq!(header.size, src_box.box_size());

        let dst_box = UdtaBox::read_box(&mut reader, header.size).unwrap();
        assert_eq!(dst_box, src_box);
    }

    #[test]
    fn test_udta() {
        let src_box = UdtaBox {
            meta: Some(MetaBox::default()),
            children: Vec::new(),
        };

        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::UdtaBox);
        assert_eq!(header.size, src_box.box_size());

        let dst_box = UdtaBox::read_box(&mut reader, header.size).unwrap();
        assert_eq!(dst_box, src_box);
    }
}
