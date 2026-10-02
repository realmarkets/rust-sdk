//! JSON representation for the u128 ids carried by events and engine state.
//!
//! Order and conditional-order ids are u128, well past the 2^53 a JSON number
//! carries losslessly, so they cross the JSON boundary as decimal strings —
//! which is what clients have always received. Proto keeps them numeric
//! (`Uint128`); this module is only about the JSON side.
//!
//! An id past `u64::MAX` is not merely imprecise as a JSON number: `to_value`
//! refuses it outright. The engine state indexes ids in sets and maps — per
//! account, per price level, per expiry — and the operator snapshot renders
//! that state through `to_value`, so those containers go through
//! [`Ids`](crate::u128_str::Ids) and the [`ids`](crate::u128_str::ids) module,
//! one `with` attribute whatever the nesting.
//!
//! Serialization goes through `collect_str`, which serde_json writes straight
//! into the output buffer, so rendering an id costs no intermediate `String`.
//! Deserialization accepts a number as well as a string: a client that echoes
//! a small id back as JSON number is still readable.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    hash::{BuildHasher, Hash},
};

use serde::{
    Serialize, Serializer,
    de::{self, Deserialize, DeserializeOwned, Deserializer, Visitor},
};

pub fn serialize<S: Serializer>(id: &u128, s: S) -> Result<S::Ok, S::Error> {
    return s.collect_str(id);
}

pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u128, D::Error> {
    return d.deserialize_any(IdVisitor);
}

pub mod opt {
    use serde::{Deserialize, Deserializer};

    use super::{Id, Serializer};

    pub fn serialize<S: Serializer>(id: &Option<u128>, s: S) -> Result<S::Ok, S::Error> {
        return match id {
            Some(id) => s.collect_str(id),
            None => s.serialize_none(),
        };
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u128>, D::Error> {
        return Ok(Option::<Id>::deserialize(d)?.map(|id| id.0));
    }
}

pub mod set {
    use std::collections::BTreeSet;

    use serde::{Deserialize, Deserializer};

    use super::{Id, Serializer, Str};

    pub fn serialize<S: Serializer>(ids: &BTreeSet<u128>, s: S) -> Result<S::Ok, S::Error> {
        return s.collect_seq(ids.iter().map(|id| Str(*id)));
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeSet<u128>, D::Error> {
        return Ok(Vec::<Id>::deserialize(d)?
            .into_iter()
            .map(|id| id.0)
            .collect());
    }
}

pub mod vec {
    use serde::{Deserialize, Deserializer};

    use super::{Id, Serializer, Str};

    pub fn serialize<S: Serializer>(ids: &[u128], s: S) -> Result<S::Ok, S::Error> {
        return s.collect_seq(ids.iter().map(|id| Str(*id)));
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u128>, D::Error> {
        return Ok(Vec::<Id>::deserialize(d)?
            .into_iter()
            .map(|id| id.0)
            .collect());
    }
}

/// An index of ids whose every leaf renders as a decimal string: one id, a
/// set or pair of them, or a map whose values are themselves such an index.
pub trait Ids: Sized {
    fn write<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error>;
    fn read<'de, D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>;
}

impl Ids for u128 {
    fn write<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        return serialize(self, s);
    }

    fn read<'de, D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        return deserialize(d);
    }
}

impl Ids for [u128; 2] {
    fn write<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        return s.collect_seq(self.iter().map(|id| Str(*id)));
    }

    fn read<'de, D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let [a, b] = <[Id; 2]>::deserialize(d)?;
        return Ok([a.0, b.0]);
    }
}

impl Ids for BTreeSet<u128> {
    fn write<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        return set::serialize(self, s);
    }

    fn read<'de, D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        return set::deserialize(d);
    }
}

impl<K: Serialize + DeserializeOwned + Ord, V: Ids> Ids for BTreeMap<K, V> {
    fn write<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        return s.collect_map(self.iter().map(|(k, v)| (k, Borrowed(v))));
    }

    fn read<'de, D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        return Ok(BTreeMap::<K, Owned<V>>::deserialize(d)?
            .into_iter()
            .map(|(k, v)| (k, v.0))
            .collect());
    }
}

impl<K, V, H> Ids for HashMap<K, V, H>
where
    K: Serialize + DeserializeOwned + Eq + Hash,
    V: Ids,
    H: BuildHasher + Default,
{
    fn write<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        return s.collect_map(self.iter().map(|(k, v)| (k, Borrowed(v))));
    }

    fn read<'de, D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        return Ok(HashMap::<K, Owned<V>, H>::deserialize(d)?
            .into_iter()
            .map(|(k, v)| (k, v.0))
            .collect());
    }
}

/// `#[serde(with = "u128_str::ids")]` for any [`Ids`] field.
pub mod ids {
    use super::{Deserializer, Ids, Serializer};

    pub fn serialize<S: Serializer, T: Ids>(ids: &T, s: S) -> Result<S::Ok, S::Error> {
        return ids.write(s);
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: Ids>(d: D) -> Result<T, D::Error> {
        return T::read(d);
    }
}

// a map value, borrowed for rendering and owned for parsing, so the map impls
// can hand it to serde's own map machinery
struct Borrowed<'a, T: Ids>(&'a T);

impl<T: Ids> Serialize for Borrowed<'_, T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        return self.0.write(s);
    }
}

struct Owned<T: Ids>(T);

impl<'de, T: Ids> Deserialize<'de> for Owned<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        return T::read(d).map(Owned);
    }
}

struct IdVisitor;

impl<'de> Visitor<'de> for IdVisitor {
    type Value = u128;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return f.write_str("a u128 id as a decimal string or a number");
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<u128, E> {
        return v.parse::<u128>().map_err(E::custom);
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<u128, E> {
        return Ok(v as u128);
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<u128, E> {
        return Ok(v);
    }
}

// one id, wrapped so the `opt` and `vec` forms reuse the scalar impls
struct Id(u128);

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        return Ok(Id(deserialize(d)?));
    }
}

struct Str(u128);

impl Serialize for Str {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        return s.collect_str(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet, HashMap};

    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Holder {
        #[serde(with = "super")]
        id: u128,
        #[serde(with = "super::opt")]
        maybe: Option<u128>,
        #[serde(with = "super::vec")]
        many: Vec<u128>,
    }

    #[test]
    fn ids_serialize_as_decimal_strings() {
        let holder = Holder {
            id: u128::MAX,
            maybe: Some(0),
            many: vec![1, 340_282_366_920_938_463_463_374_607_431_768_211_455],
        };
        assert_eq!(
            crate::to_string(&holder).unwrap(),
            r#"{"id":"340282366920938463463374607431768211455","maybe":"0","many":["1","340282366920938463463374607431768211455"]}"#
        );
        assert_eq!(
            crate::from_str::<Holder>(&crate::to_string(&holder).unwrap()).unwrap(),
            holder
        );
    }

    #[test]
    fn absent_optional_id_is_null() {
        let holder = Holder {
            id: 7,
            maybe: None,
            many: vec![],
        };
        assert_eq!(
            crate::to_string(&holder).unwrap(),
            r#"{"id":"7","maybe":null,"many":[]}"#
        );
        assert_eq!(
            crate::from_str::<Holder>(r#"{"id":"7","maybe":null,"many":[]}"#).unwrap(),
            holder
        );
    }

    // a client that echoes an id back as a JSON number stays readable
    #[test]
    fn numbers_deserialize_as_ids() {
        assert_eq!(
            crate::from_str::<Holder>(r#"{"id":7,"maybe":8,"many":[9]}"#).unwrap(),
            Holder {
                id: 7,
                maybe: Some(8),
                many: vec![9],
            }
        );
    }

    #[test]
    fn a_non_numeric_id_is_an_error() {
        let err = crate::from_str::<Holder>(r#"{"id":"nope","maybe":null,"many":[]}"#)
            .expect_err("a non-numeric id must not parse");
        assert!(err.to_string().contains("invalid digit"), "{err}");
    }

    // the shapes the engine state indexes ids in: sets per key, a nested map of
    // ids, and a pair of totals
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Index {
        #[serde(with = "super::ids")]
        sets: BTreeMap<String, BTreeSet<u128>>,
        #[serde(with = "super::ids")]
        nested: BTreeMap<String, HashMap<String, u128>>,
        #[serde(with = "super::ids")]
        pairs: BTreeMap<String, [u128; 2]>,
    }

    fn index() -> Index {
        return Index {
            sets: BTreeMap::from([("a".to_string(), BTreeSet::from([u128::MAX, 1]))]),
            nested: BTreeMap::from([(
                "a".to_string(),
                HashMap::from([("c".to_string(), u128::MAX)]),
            )]),
            pairs: BTreeMap::from([("a".to_string(), [u128::MAX, 0])]),
        };
    }

    #[test]
    fn nested_indexes_render_every_id_as_a_decimal_string() {
        let rendered = crate::to_string(&index()).unwrap();
        assert_eq!(
            rendered,
            r#"{"sets":{"a":["1","340282366920938463463374607431768211455"]},"nested":{"a":{"c":"340282366920938463463374607431768211455"}},"pairs":{"a":["340282366920938463463374607431768211455","0"]}}"#
        );
        assert_eq!(crate::from_str::<Index>(&rendered).unwrap(), index());
    }

    // `to_value` is the path that rejects a bare u128 past u64::MAX outright
    #[test]
    fn nested_indexes_survive_to_value() {
        let value = crate::to_value(&index()).expect("every id renders as a string");
        assert_eq!(crate::from_value::<Index>(value).unwrap(), index());
    }
}
