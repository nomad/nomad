use ed::{backend, notify};
use indexmap::IndexMap;
use serde_json::Number;

/// TODO: docs.
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    List(Vec<Self>),
    Map(Map),
    Function(Box<dyn FnMut(Self) -> Self>),
}

#[derive(Default)]
pub struct Map {
    inner: IndexMap<String, Value>,
}

pub struct MapAccess<'a> {
    map: &'a mut Map,
    idx: Option<usize>,
}

pub struct MapAccessError {
    kind: &'static str,
}

impl Value {
    fn kind(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::List(_) => "list",
            Self::Map(_) => "map",
            Self::Function(_) => "function",
        }
    }
}

impl Map {
    pub(crate) fn contains_key(&mut self, key: impl AsRef<str>) -> bool {
        self.inner.contains_key(key.as_ref())
    }

    pub(crate) fn insert(
        &mut self,
        key: impl AsRef<str>,
        value: impl Into<Value>,
    ) {
        self.inner.insert(key.as_ref().to_owned(), value.into());
    }
}

impl backend::Value for Value {
    type MapAccess<'a> = MapAccess<'a>;
    type MapAccessError<'a> = MapAccessError;

    fn map_access(
        &mut self,
    ) -> Result<Self::MapAccess<'_>, Self::MapAccessError<'_>> {
        match self {
            Self::Map(map) => Ok(MapAccess { map, idx: None }),
            _ => Err(MapAccessError { kind: self.kind() }),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Null
    }
}

impl backend::MapAccess for MapAccess<'_> {
    type Key<'a>
        = &'a str
    where
        Self: 'a;
    type Value = Value;

    fn next_key(&mut self) -> Option<Self::Key<'_>> {
        let mut is_first_access = false;
        let idx = self.idx.get_or_insert_with(|| {
            is_first_access = true;
            0
        });
        let maybe_key = self.map.inner.get_index(*idx).map(|(key, _)| &**key);
        *idx += !is_first_access as usize;
        maybe_key
    }

    fn take_next_value(&mut self) -> Self::Value {
        let idx = self.idx.expect("already called next_key");
        let (_, value) =
            self.map.inner.swap_remove_index(idx).expect("not oob");
        value
    }
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(bool) => Self::Bool(bool),
            serde_json::Value::Number(number) => Self::Number(number),
            serde_json::Value::String(str) => Self::String(str),
            serde_json::Value::Array(vec) => {
                Self::List(vec.into_iter().map(Into::into).collect())
            },
            serde_json::Value::Object(map) => Self::Map(
                map.into_iter().map(|(k, v)| (k, v.into())).collect(),
            ),
        }
    }
}

impl TryFrom<Value> for serde_json::Value {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        use serde::de::Error;
        match value {
            Value::Null => Ok(serde_json::Value::Null),
            Value::Bool(bool) => Ok(serde_json::Value::Bool(bool)),
            Value::Number(number) => Ok(serde_json::Value::Number(number)),
            Value::String(string) => Ok(serde_json::Value::String(string)),
            Value::List(list) => Ok(serde_json::Value::Array(
                list.into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            )),
            Value::Map(map) => Ok(serde_json::Value::Object(
                map.into_iter()
                    .map(|(k, v)| v.try_into().map(|v| (k, v)))
                    .collect::<Result<_, _>>()?,
            )),
            Value::Function(_) => Err(serde_json::Error::custom(
                "cannot convert function to JSON value",
            )),
        }
    }
}

impl IntoIterator for Map {
    type Item = (String, Value);
    type IntoIter = indexmap::map::IntoIter<String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl FromIterator<(String, Value)> for Map {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        Self { inner: IndexMap::from_iter(iter) }
    }
}

impl notify::Error for MapAccessError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let msg = format!("expected a map, got {} instead", self.kind);
        (notify::Level::Error, notify::Message::from_str(msg))
    }
}
