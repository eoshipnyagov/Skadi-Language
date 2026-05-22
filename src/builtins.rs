#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    Len,
    Contains,
    Find,
    Slice,
}

pub fn builtin_from_name(name: &str) -> Option<Builtin> {
    match name {
        "len" => Some(Builtin::Len),
        "contains" => Some(Builtin::Contains),
        "find" => Some(Builtin::Find),
        "slice" => Some(Builtin::Slice),
        _ => None,
    }
}

pub fn builtin_arity(b: Builtin) -> usize {
    match b {
        Builtin::Len => 1,
        Builtin::Contains => 2,
        Builtin::Find => 2,
        Builtin::Slice => 3,
    }
}

pub fn is_builtin(name: &str) -> bool {
    builtin_from_name(name).is_some()
}
