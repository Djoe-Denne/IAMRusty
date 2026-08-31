/// Distinguishes “leave unchanged” from “assign this value” (including `None`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FieldUpdate<T> {
    #[default]
    Unchanged,
    Set(T),
}
