idf_component_register(SRCS "skadi_program.c" "native_0_board.c" INCLUDE_DIRS "." REQUIRES esp_driver_gpio esp_driver_gptimer esp_timer)
target_compile_options(${COMPONENT_LIB} PRIVATE -Wno-unused-function -Wno-unused-variable)
