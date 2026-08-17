pub mod c;

pub use c::{
    CodegenOptions, CodegenOutput, CodegenSourceMapEntry, IntWidth, ensure_codegen_supported,
    ensure_codegen_supported_with_options, transpile_program_to_c, transpile_program_to_c_with_map,
    transpile_program_to_c_with_options,
};
