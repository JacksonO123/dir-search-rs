use std::{collections::HashMap, hash::Hash, string};

pub fn byte_slice_to_string(slice: &[u8]) -> Result<String, string::FromUtf8Error> {
    Ok(String::from_utf8(slice.to_vec())?)
}

pub fn exists_as_val_in_map<K, V, E>(
    map: &HashMap<K, V>,
    key: K,
    expected_val: V,
    err: E,
) -> Result<bool, E>
where
    K: Eq + Hash,
    V: Eq,
{
    match map.get(&key) {
        Some(value) => {
            if *value != expected_val {
                return Err(err);
            }

            Ok(true)
        }
        None => Ok(false),
    }
}
