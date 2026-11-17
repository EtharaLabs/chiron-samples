find_program(HARNESS_MAKE NAMES gmake make REQUIRED)
add_custom_target(docs-prep
  COMMAND ${Python3_EXECUTABLE} "${PROJECT_SOURCE_DIR}/cmake/Docs.py" "${PROJECT_SOURCE_DIR}" "${PROJECT_BINARY_DIR}" "${YOSYS_PROGRAM_PREFIX}"
  COMMAND ${HARNESS_MAKE} -C "${PROJECT_SOURCE_DIR}/docs" gen
  DEPENDS yosys share VERBATIM)
add_custom_target(coverage COMMAND ${Python3_EXECUTABLE} "${PROJECT_SOURCE_DIR}/cmake/Coverage.py" "${PROJECT_SOURCE_DIR}" "${PROJECT_BINARY_DIR}" collect DEPENDS yosys VERBATIM)
add_custom_target(clean-coverage COMMAND ${Python3_EXECUTABLE} "${PROJECT_SOURCE_DIR}/cmake/Coverage.py" "${PROJECT_SOURCE_DIR}" "${PROJECT_BINARY_DIR}" clean VERBATIM)
