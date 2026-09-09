/// Distinguishes “leave unchanged” from “assign this value” (including `None`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FieldUpdate<T> {
    #[default]
    Unchanged,
    Set(T),
}

impl<T> FieldUpdate<T> {
    #[must_use]
    pub const fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }
}

impl<T> serde::Serialize for FieldUpdate<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Set(value) => value.serialize(serializer),
            Self::Unchanged => serializer.serialize_none(),
        }
    }
}

impl<'de, T> serde::Deserialize<'de> for FieldUpdate<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::Set)
    }
}
