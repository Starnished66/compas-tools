//! JSON where the first of repeated keys wins, as in the player's cJSON
//! (`serde_json::Value` keeps the last).

use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

pub(super) struct FirstWins(pub Value);

impl<'de> Deserialize<'de> for FirstWins {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(FirstWinsVisitor)
    }
}

struct FirstWinsVisitor;

impl<'de> Visitor<'de> for FirstWinsVisitor {
    type Value = FirstWins;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<FirstWins, E> {
        Ok(FirstWins(v.into()))
    }

    fn visit_i64<E>(self, v: i64) -> Result<FirstWins, E> {
        Ok(FirstWins(v.into()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<FirstWins, E> {
        Ok(FirstWins(v.into()))
    }

    fn visit_f64<E>(self, v: f64) -> Result<FirstWins, E> {
        Ok(FirstWins(Value::from(v)))
    }

    fn visit_str<E>(self, v: &str) -> Result<FirstWins, E> {
        Ok(FirstWins(v.into()))
    }

    fn visit_unit<E>(self) -> Result<FirstWins, E> {
        Ok(FirstWins(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<FirstWins, A::Error> {
        let mut items = Vec::new();
        while let Some(FirstWins(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(FirstWins(Value::Array(items)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<FirstWins, A::Error> {
        let mut object = Map::new();
        while let Some((key, FirstWins(value))) = map.next_entry::<String, FirstWins>()? {
            object.entry(key).or_insert(value);
        }
        Ok(FirstWins(Value::Object(object)))
    }
}

pub(super) fn parse(bytes: &[u8]) -> serde_json::Result<Value> {
    serde_json::from_slice::<FirstWins>(bytes).map(|v| v.0)
}

#[cfg(test)]
mod tests;
