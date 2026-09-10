#![forbid(unsafe_code)]

//! Bytes that are only bytes. The shape every Stream can take when nothing
//! else says what it is: one nameless part, the whole content with the media
//! type it came with, and no announced type. `binary` says yes to everything
//! and is asked last (ADR-0047).
//!
//! It refuses nothing. An empty Stream is one empty part, because a Message
//! with no content still has a Section to carry it.

use message::{Part, Shape, ShapeError, Shaped};
use stream::Stream;

/// The binary shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct Binary;

impl Shape for Binary {
    fn technology(&self) -> &'static str {
        "binary"
    }

    fn media_types(&self) -> &'static [&'static str] {
        &["application/octet-stream"]
    }

    fn recognises(&self, _: &[u8]) -> bool {
        true
    }

    fn shape(&self, stream: &Stream) -> Result<Shaped, ShapeError> {
        Ok(Shaped {
            parts: vec![Part::whole(stream)],
            message_type: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xcore::StreamId;

    fn stream(bytes: &[u8], media: Option<&str>) -> Stream {
        Stream::new(StreamId::new(1), bytes.to_vec(), media.map(str::to_string))
    }

    #[test]
    fn any_bytes_are_one_nameless_part_with_the_media_type_they_came_with() {
        let source = stream(b"\x00\xff\x01", Some("image/png"));
        let shaped = Binary.shape(&source).expect("binary refuses nothing");
        assert_eq!(shaped.parts.len(), 1);
        assert_eq!(shaped.parts[0].name, None);
        assert_eq!(shaped.parts[0].bytes, b"\x00\xff\x01");
        assert_eq!(shaped.parts[0].media_type.as_deref(), Some("image/png"));
        assert_eq!(shaped.message_type, None);
    }

    #[test]
    fn an_empty_stream_is_one_empty_part_rather_than_a_refusal() {
        let shaped = Binary
            .shape(&stream(b"", None))
            .expect("empty is still bytes");
        assert_eq!(shaped.parts.len(), 1);
        assert!(shaped.parts[0].bytes.is_empty());
        assert_eq!(shaped.parts[0].media_type, None);
    }

    #[test]
    fn the_shape_claims_octet_stream_and_recognises_everything() {
        assert_eq!(Binary.technology(), "binary");
        assert_eq!(Binary.media_types(), &["application/octet-stream"]);
        assert!(Binary.recognises(b""));
        assert!(Binary.recognises(b"{\"json\": true}"));
        assert!(Binary.recognises(&[0xff, 0xfe, 0x00]));
    }

    #[test]
    fn choose_falls_through_to_binary_when_nothing_before_it_claims_the_stream() {
        struct Never;
        impl Shape for Never {
            fn technology(&self) -> &'static str {
                "never"
            }
            fn media_types(&self) -> &'static [&'static str] {
                &["application/never"]
            }
            fn recognises(&self, _: &[u8]) -> bool {
                false
            }
            fn shape(&self, _: &Stream) -> Result<Shaped, ShapeError> {
                Err(ShapeError::new("never", "never shapes"))
            }
        }
        let shapes: [&dyn Shape; 2] = [&Never, &Binary];
        let chosen = message::choose(&shapes, &stream(b"anything", Some("image/gif")));
        assert_eq!(chosen.map(Shape::technology), Some("binary"));
        let by_media = message::choose(&shapes, &stream(b"", Some("application/octet-stream")));
        assert_eq!(by_media.map(Shape::technology), Some("binary"));
    }
}
