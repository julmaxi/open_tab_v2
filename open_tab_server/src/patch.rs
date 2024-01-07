use serde::{Deserialize, Serialize, Deserializer};

#[derive(Debug, Clone, Serialize)]
pub enum PatchValue<V> {
    NotSet,
    Set(V)
}

impl<T> Default for PatchValue<T> {
    fn default() -> Self {
        PatchValue::NotSet
    }
}

impl<'de, T> Deserialize<'de> for PatchValue<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|v| PatchValue::Set(v))
    }
}