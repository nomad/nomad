use core::fmt;

use serde::de;

/// TODO: docs.
#[derive(Default, Clone, Copy)]
pub struct Empty;

impl<'de> de::Deserialize<'de> for Empty {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct EmptyVisitor;

        impl<'de> serde::de::Visitor<'de> for EmptyVisitor {
            type Value = Empty;

            #[inline]
            fn expecting(
                &self,
                formatter: &mut fmt::Formatter,
            ) -> fmt::Result {
                formatter.write_str("null or {}")
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Empty)
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                map.next_key::<de::IgnoredAny>()
                    .expect("never fails")
                    .is_none()
                    .then_some(Empty)
                    .ok_or_else(|| {
                        de::Error::invalid_value(
                            de::Unexpected::Map,
                            &"an empty map",
                        )
                    })
            }

            #[inline]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                seq.next_element::<de::IgnoredAny>()
                    .expect("never fails")
                    .is_none()
                    .then_some(Empty)
                    .ok_or_else(|| {
                        de::Error::invalid_value(
                            de::Unexpected::Seq,
                            &"an empty sequence",
                        )
                    })
            }
        }

        deserializer.deserialize_any(EmptyVisitor)
    }
}
