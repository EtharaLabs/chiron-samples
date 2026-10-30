# Copyright (C) 2019 Yaoyuan <ibireme@gmail.com>.
# Released under the MIT License:
# https://github.com/ibireme/yyjson/blob/master/LICENSE
#
# The library's implementation lives in `rust/` and is built with cargo.
# No compiled language is enabled here: `project(... LANGUAGES NONE)` means
# CMake never looks for a C, C++, Objective-C or assembly compiler. The only
# toolchain this build needs is cargo/rustc, which links through `cc`.

cmake_minimum_required(VERSION 3.5...3.31)
project(yyjson VERSION 0.13.0 LANGUAGES NONE)
set(YYJSON_SOVERSION 0)



# ------------------------------------------------------------------------------
# Build Type
if (CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR)
    if(NOT CMAKE_BUILD_TYPE)
        message(STATUS "No build type selected, default to: Release")
        set(CMAKE_BUILD_TYPE Release)
    endif()
endif()



# ------------------------------------------------------------------------------
# Build Options
option(YYJSON_INSTALL "Generate installation target" ON)

# Compilation options, see yyjson.h for more explanation.
# They are kept so that existing cache entries and scripts keep working, but
# the Rust implementation always builds the full, default-configured library.
option(YYJSON_DISABLE_READER "Disable JSON reader" OFF)
option(YYJSON_DISABLE_WRITER "Disable JSON writer" OFF)
option(YYJSON_DISABLE_INCR_READER "Disable incremental reader" OFF)
option(YYJSON_DISABLE_FILE "Disable file/fp read and write APIs" OFF)
option(YYJSON_DISABLE_UTILS "Disable JSON Pointer, JSON Patch, JSON Merge Patch" OFF)
option(YYJSON_DISABLE_FAST_FP_CONV "Disable custom floating-point number conversion" OFF)
option(YYJSON_DISABLE_NON_STANDARD "Disable non-standard JSON support" OFF)
option(YYJSON_DISABLE_UTF8_VALIDATION "Disable UTF-8 validation" OFF)
option(YYJSON_DISABLE_UNALIGNED_MEMORY_ACCESS "Disable unaligned memory access explicit" OFF)
option(YYJSON_FREESTANDING "Build without libc (freestanding)" OFF)

foreach(_opt
        YYJSON_DISABLE_READER YYJSON_DISABLE_WRITER YYJSON_DISABLE_INCR_READER
        YYJSON_DISABLE_FILE YYJSON_DISABLE_UTILS YYJSON_DISABLE_FAST_FP_CONV
        YYJSON_DISABLE_NON_STANDARD YYJSON_DISABLE_UTF8_VALIDATION
        YYJSON_FREESTANDING)
    if(${_opt})
        message(WARNING "${_opt} is not supported by the Rust implementation "
                        "and is ignored; the full library is always built.")
    endif()
endforeach()



# ------------------------------------------------------------------------------
# Toolchain
find_program(CARGO_EXECUTABLE NAMES cargo)
if(NOT CARGO_EXECUTABLE)
    message(FATAL_ERROR "cargo was not found; it is required to build yyjson.")
endif()
find_program(RUSTC_EXECUTABLE NAMES rustc)
if(NOT RUSTC_EXECUTABLE)
    message(FATAL_ERROR "rustc was not found; it is required to build yyjson.")
endif()

# `GNUInstallDirs` picks lib/ vs lib64/ from `CMAKE_SIZEOF_VOID_P`, which is
# normally set by the compiler probe. No language is enabled here, so ask the
# Rust toolchain for the target's pointer width instead.
if(NOT CMAKE_SIZEOF_VOID_P)
    execute_process(COMMAND ${RUSTC_EXECUTABLE} --print cfg
                    OUTPUT_VARIABLE YYJSON_RUSTC_CFG
                    OUTPUT_STRIP_TRAILING_WHITESPACE)
    if(YYJSON_RUSTC_CFG MATCHES "target_pointer_width=\"([0-9]+)\"")
        math(EXPR CMAKE_SIZEOF_VOID_P "${CMAKE_MATCH_1} / 8")
    endif()
endif()



# ------------------------------------------------------------------------------
# Library
set(YYJSON_RUST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/rust")
set(YYJSON_CARGO_TARGET_DIR "${CMAKE_CURRENT_BINARY_DIR}/cargo")
set(YYJSON_CARGO_OUT_DIR "${YYJSON_CARGO_TARGET_DIR}/release")

file(GLOB YYJSON_RUST_SOURCES "${YYJSON_RUST_DIR}/src/*.rs")
list(APPEND YYJSON_RUST_SOURCES
     "${YYJSON_RUST_DIR}/Cargo.toml"
     "${YYJSON_RUST_DIR}/build.rs")

if(BUILD_SHARED_LIBS)
    set(YYJSON_CARGO_ARTIFACT "${YYJSON_CARGO_OUT_DIR}/libyyjson.so")
else()
    set(YYJSON_CARGO_ARTIFACT "${YYJSON_CARGO_OUT_DIR}/libyyjson.a")
endif()

# `--offline` keeps cargo from touching the network; the crate has no
# dependencies, so there is nothing to fetch anyway.
add_custom_command(
    OUTPUT "${YYJSON_CARGO_ARTIFACT}"
    COMMAND ${CMAKE_COMMAND} -E env
            "CARGO_TARGET_DIR=${YYJSON_CARGO_TARGET_DIR}"
            ${CARGO_EXECUTABLE} build --release --offline
            --manifest-path "${YYJSON_RUST_DIR}/Cargo.toml"
    DEPENDS ${YYJSON_RUST_SOURCES}
    WORKING_DIRECTORY "${YYJSON_RUST_DIR}"
    COMMENT "Building yyjson with cargo"
    VERBATIM)

add_custom_target(yyjson ALL DEPENDS "${YYJSON_CARGO_ARTIFACT}")



# ------------------------------------------------------------------------------
# Install
if (YYJSON_INSTALL)
    include(GNUInstallDirs)

    set(YYJSON_LIB_FULL_NAME "libyyjson.so.${PROJECT_VERSION}")
    set(YYJSON_LIB_SONAME "libyyjson.so.${YYJSON_SOVERSION}")

    if(CMAKE_BUILD_TYPE)
        string(TOUPPER "${CMAKE_BUILD_TYPE}" YYJSON_CONFIG_UPPER)
        string(TOLOWER "${CMAKE_BUILD_TYPE}" YYJSON_CONFIG_LOWER)
    else()
        set(YYJSON_CONFIG_UPPER "NOCONFIG")
        set(YYJSON_CONFIG_LOWER "noconfig")
    endif()

    if(BUILD_SHARED_LIBS)
        set(YYJSON_IMPORTED_TYPE "SHARED")
        set(YYJSON_IMPORTED_FILE "\${_IMPORT_PREFIX}/${CMAKE_INSTALL_LIBDIR}/${YYJSON_LIB_FULL_NAME}")
        set(YYJSON_IMPORTED_PROPS "  IMPORTED_LOCATION_${YYJSON_CONFIG_UPPER} \"${YYJSON_IMPORTED_FILE}\"
  IMPORTED_SONAME_${YYJSON_CONFIG_UPPER} \"${YYJSON_LIB_SONAME}\"")
        install(FILES "${YYJSON_CARGO_ARTIFACT}"
                DESTINATION "${CMAKE_INSTALL_LIBDIR}"
                RENAME "${YYJSON_LIB_FULL_NAME}"
                PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE
                            GROUP_READ GROUP_EXECUTE
                            WORLD_READ WORLD_EXECUTE)
        install(CODE "
            set(_libdir \"\$ENV{DESTDIR}\${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR}\")
            message(STATUS \"Installing: \${_libdir}/${YYJSON_LIB_SONAME}\")
            execute_process(COMMAND \"\${CMAKE_COMMAND}\" -E create_symlink
                            \"${YYJSON_LIB_FULL_NAME}\" \"\${_libdir}/${YYJSON_LIB_SONAME}\")
            message(STATUS \"Installing: \${_libdir}/libyyjson.so\")
            execute_process(COMMAND \"\${CMAKE_COMMAND}\" -E create_symlink
                            \"${YYJSON_LIB_SONAME}\" \"\${_libdir}/libyyjson.so\")
        ")
    else()
        set(YYJSON_IMPORTED_TYPE "STATIC")
        set(YYJSON_IMPORTED_FILE "\${_IMPORT_PREFIX}/${CMAKE_INSTALL_LIBDIR}/libyyjson.a")
        set(YYJSON_IMPORTED_PROPS "  IMPORTED_LINK_INTERFACE_LANGUAGES_${YYJSON_CONFIG_UPPER} \"C\"
  IMPORTED_LOCATION_${YYJSON_CONFIG_UPPER} \"${YYJSON_IMPORTED_FILE}\"")
        install(FILES "${YYJSON_CARGO_ARTIFACT}"
                DESTINATION "${CMAKE_INSTALL_LIBDIR}")
    endif()

    install(FILES src/yyjson.h DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}")

    configure_file(cmake/yyjson-config.cmake.in
                   "${CMAKE_BINARY_DIR}/yyjson-config.cmake" @ONLY)
    configure_file(cmake/yyjson-config-build-type.cmake.in
                   "${CMAKE_BINARY_DIR}/yyjson-config-${YYJSON_CONFIG_LOWER}.cmake" @ONLY)
    install(FILES "${CMAKE_BINARY_DIR}/yyjson-config.cmake"
                  "${CMAKE_BINARY_DIR}/yyjson-config-${YYJSON_CONFIG_LOWER}.cmake"
            DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/yyjson")

    configure_file(yyjson.pc.in "${CMAKE_BINARY_DIR}/yyjson.pc" @ONLY)
    install(FILES "${CMAKE_BINARY_DIR}/yyjson.pc"
            DESTINATION "${CMAKE_INSTALL_LIBDIR}/pkgconfig")
endif()
