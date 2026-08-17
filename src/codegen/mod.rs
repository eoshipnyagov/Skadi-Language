pub mod c;

pub use c::{
    CodegenOutput, CodegenSourceMapEntry, ensure_codegen_supported, transpile_program_to_c,
    transpile_program_to_c_with_map,
};
