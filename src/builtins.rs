#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    Len,
    Contains,
    Find,
    Slice,
    FsList,
    FsIsDir,
    // Math track (reserved for 1.x activation):
    // Sin,
    // Cos,
    // Atan2,
    // Root,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinCategory {
    CoreCollectionText,
    CoreFilesystem,
    Math,
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinSpec {
    pub builtin: Builtin,
    pub name: &'static str,
    pub arity: usize,
    pub category: BuiltinCategory,
    pub enabled: bool,
}

const BUILTIN_SPECS: &[BuiltinSpec] = &[
    BuiltinSpec {
        builtin: Builtin::Len,
        name: "len",
        arity: 1,
        category: BuiltinCategory::CoreCollectionText,
        enabled: true,
    },
    BuiltinSpec {
        builtin: Builtin::Contains,
        name: "contains",
        arity: 2,
        category: BuiltinCategory::CoreCollectionText,
        enabled: true,
    },
    BuiltinSpec {
        builtin: Builtin::Find,
        name: "find",
        arity: 2,
        category: BuiltinCategory::CoreCollectionText,
        enabled: true,
    },
    BuiltinSpec {
        builtin: Builtin::Slice,
        name: "slice",
        arity: 3,
        category: BuiltinCategory::CoreCollectionText,
        enabled: true,
    },
    BuiltinSpec {
        builtin: Builtin::FsList,
        name: "fs.list",
        arity: 1,
        category: BuiltinCategory::CoreFilesystem,
        enabled: true,
    },
    BuiltinSpec {
        builtin: Builtin::FsIsDir,
        name: "fs.is_dir",
        arity: 1,
        category: BuiltinCategory::CoreFilesystem,
        enabled: true,
    },
    // Reserved (disabled) math builtins for 1.x:
    // BuiltinSpec {
    //     builtin: Builtin::Sin,
    //     name: "sin",
    //     arity: 1,
    //     category: BuiltinCategory::Math,
    //     enabled: false,
    // },
    // BuiltinSpec {
    //     builtin: Builtin::Cos,
    //     name: "cos",
    //     arity: 1,
    //     category: BuiltinCategory::Math,
    //     enabled: false,
    // },
    // BuiltinSpec {
    //     builtin: Builtin::Atan2,
    //     name: "atan2",
    //     arity: 2,
    //     category: BuiltinCategory::Math,
    //     enabled: false,
    // },
    // BuiltinSpec {
    //     builtin: Builtin::Root,
    //     name: "root",
    //     arity: 2,
    //     category: BuiltinCategory::Math,
    //     enabled: false,
    // },
];

fn spec_for_builtin(b: Builtin) -> Option<&'static BuiltinSpec> {
    BUILTIN_SPECS.iter().find(|s| s.builtin == b)
}

pub fn builtin_from_name(name: &str) -> Option<Builtin> {
    BUILTIN_SPECS
        .iter()
        .find(|s| s.enabled && s.name == name)
        .map(|s| s.builtin)
}

pub fn builtin_arity(b: Builtin) -> usize {
    spec_for_builtin(b).map(|s| s.arity).unwrap_or(0)
}

pub fn is_builtin(name: &str) -> bool {
    builtin_from_name(name).is_some()
}

pub fn builtin_category(b: Builtin) -> Option<BuiltinCategory> {
    spec_for_builtin(b).map(|s| s.category)
}
