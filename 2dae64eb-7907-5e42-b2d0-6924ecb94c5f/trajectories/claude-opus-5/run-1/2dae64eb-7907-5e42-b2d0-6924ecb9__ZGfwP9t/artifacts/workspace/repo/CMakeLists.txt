# Copyright (C) 2019 Yaoyuan <ibireme@gmail.com>.
# Released under the MIT License:
# https://github.com/ibireme/yyjson/blob/master/LICENSE

# The library's implementation lives in `rust/` and is built with cargo. No
# compiled language is enabled here: `LANGUAGES NONE` keeps CMake from probing
# for a C (or any other) compiler, so this project configures and builds on a
# machine that has none. Linking still happens -- rustc drives it through
# `cc` -- but that is not compilation.
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
# Build Options for tests and docs
option(YYJSON_BUILD_DOC "Build documentation with doxygen" OFF)
option(YYJSON_INSTALL "Generate installation target" ON)



# ------------------------------------------------------------------------------
# Compilation options, see yyjson.h for more explanation.
#
# These are declared so that an existing cache or command line that sets them
# keeps working. The Rust implementation is built with the library's default
# configuration; a non-default value here is reported as an error rather than
# silently ignored.
set(YYJSON_OFF_OPTIONS
    YYJSON_DISABLE_READER
    YYJSON_DISABLE_WRITER
    YYJSON_DISABLE_INCR_READER
    YYJSON_DISABLE_FILE
    YYJSON_DISABLE_UTILS
    YYJSON_DISABLE_FAST_FP_CONV
    YYJSON_DISABLE_NON_STANDARD
    YYJSON_DISABLE_UTF8_VALIDATION
    YYJSON_DISABLE_UNALIGNED_MEMORY_ACCESS
    YYJSON_FREESTANDING)
foreach(opt ${YYJSON_OFF_OPTIONS})
    option(${opt} "Compile-time option, see yyjson.h" OFF)
    if(${opt})
        message(FATAL_ERROR "${opt} is not supported by this build")
    endif()
endforeach()
set(YYJSON_READER_DEPTH_LIMIT 0 CACHE STRING
    "Depth limit for reading nested objects/arrays, 0 for unlimited")
set(YYJSON_WRITER_DEPTH_LIMIT 0 CACHE STRING
    "Depth limit for writing nested objects/arrays, 0 for unlimited")
if(YYJSON_READER_DEPTH_LIMIT GREATER 0 OR YYJSON_WRITER_DEPTH_LIMIT GREATER 0)
    message(FATAL_ERROR "Depth limits are not supported by this build")
endif()



# ------------------------------------------------------------------------------
# Library
#
# `cargo` builds one artifact per configuration, matching what
# `add_library(yyjson ...)` used to produce:
#   BUILD_SHARED_LIBS=ON  -> cdylib    -> libyyjson.so.0.13.0 (+ 2 symlinks)
#   BUILD_SHARED_LIBS=OFF -> staticlib -> libyyjson.a
find_program(YYJSON_CARGO cargo
             DOC "Path to the cargo executable used to build the library")
if(NOT YYJSON_CARGO)
    message(FATAL_ERROR "cargo was not found; it is required to build yyjson")
endif()

set(YYJSON_CRATE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/rust")
set(YYJSON_CARGO_DIR "${CMAKE_CURRENT_BINARY_DIR}/cargo")

# The C build defaulted to a Release build; mirror the build type onto the
# cargo profile so that a Debug configuration is not silently optimized.
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(YYJSON_CARGO_PROFILE_ARGS "")
    set(YYJSON_CARGO_PROFILE_DIR debug)
else()
    set(YYJSON_CARGO_PROFILE_ARGS --release)
    set(YYJSON_CARGO_PROFILE_DIR release)
endif()

if(BUILD_SHARED_LIBS)
    set(YYJSON_CRATE_TYPE cdylib)
    set(YYJSON_CARGO_OUTPUT "${YYJSON_CARGO_DIR}/${YYJSON_CARGO_PROFILE_DIR}/libyyjson.so")
    # `rust_eh_personality` must not be exported from the shared library;
    # see rust/build.rs and rust/Cargo.toml.
    set(YYJSON_CARGO_FEATURE_ARGS --no-default-features)
    set(YYJSON_LIB_REAL "libyyjson.so.${PROJECT_VERSION}")
    set(YYJSON_LIB_SOVER "libyyjson.so.${YYJSON_SOVERSION}")
    set(YYJSON_LIB_LINK "libyyjson.so")
    set(YYJSON_ARTIFACTS
        "${CMAKE_CURRENT_BINARY_DIR}/${YYJSON_LIB_REAL}"
        "${CMAKE_CURRENT_BINARY_DIR}/${YYJSON_LIB_SOVER}"
        "${CMAKE_CURRENT_BINARY_DIR}/${YYJSON_LIB_LINK}")
    # The two version symlinks `set_target_properties(... SOVERSION)` used to
    # create next to the shared library.
    set(YYJSON_SYMLINK_COMMANDS
        COMMAND "${CMAKE_COMMAND}" -E create_symlink
                "${YYJSON_LIB_REAL}" "${YYJSON_LIB_SOVER}"
        COMMAND "${CMAKE_COMMAND}" -E create_symlink
                "${YYJSON_LIB_SOVER}" "${YYJSON_LIB_LINK}")
else()
    set(YYJSON_CRATE_TYPE staticlib)
    set(YYJSON_CARGO_OUTPUT "${YYJSON_CARGO_DIR}/${YYJSON_CARGO_PROFILE_DIR}/libyyjson.a")
    set(YYJSON_CARGO_FEATURE_ARGS "")
    set(YYJSON_LIB_REAL "libyyjson.a")
    set(YYJSON_ARTIFACTS "${CMAKE_CURRENT_BINARY_DIR}/${YYJSON_LIB_REAL}")
endif()

file(GLOB YYJSON_RUST_SOURCES "${YYJSON_CRATE_DIR}/src/*.rs")

# `--offline` and `--locked` keep the build from touching the network; the
# crate has no dependencies at all, so there is nothing to fetch.
add_custom_command(
    OUTPUT ${YYJSON_ARTIFACTS}
    COMMAND "${CMAKE_COMMAND}" -E env
            CARGO_TARGET_DIR=${YYJSON_CARGO_DIR}
            "${YYJSON_CARGO}" rustc
            --offline
            --locked
            --manifest-path "${YYJSON_CRATE_DIR}/Cargo.toml"
            --crate-type ${YYJSON_CRATE_TYPE}
            ${YYJSON_CARGO_PROFILE_ARGS}
            ${YYJSON_CARGO_FEATURE_ARGS}
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
            "${YYJSON_CARGO_OUTPUT}"
            "${CMAKE_CURRENT_BINARY_DIR}/${YYJSON_LIB_REAL}"
    ${YYJSON_SYMLINK_COMMANDS}
    WORKING_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}"
    DEPENDS ${YYJSON_RUST_SOURCES}
            "${YYJSON_CRATE_DIR}/Cargo.toml"
            "${YYJSON_CRATE_DIR}/Cargo.lock"
            "${YYJSON_CRATE_DIR}/build.rs"
            "${CMAKE_CURRENT_SOURCE_DIR}/src/yyjson.h"
    COMMENT "Building yyjson (${YYJSON_CRATE_TYPE}) with cargo"
    VERBATIM)

add_custom_target(yyjson-build ALL DEPENDS ${YYJSON_ARTIFACTS})

# A target for in-tree consumers (`add_subdirectory`), equivalent to what the
# old `yyjson` library target offered them.
add_library(yyjson INTERFACE)
add_dependencies(yyjson yyjson-build)
target_include_directories(yyjson INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/src>)
target_link_libraries(yyjson INTERFACE
    "${CMAKE_CURRENT_BINARY_DIR}/${YYJSON_LIB_REAL}")
add_library(yyjson::yyjson ALIAS yyjson)



# ------------------------------------------------------------------------------
# Build & Install
if (YYJSON_INSTALL)
    # GNUInstallDirs derives lib/lib64 from the pointer size, which CMake
    # normally learns from a compiler probe. Take it from the Rust toolchain
    # instead, so that no language has to be enabled.
    if(NOT CMAKE_SIZEOF_VOID_P)
        find_program(YYJSON_RUSTC rustc)
        if(YYJSON_RUSTC)
            execute_process(COMMAND "${YYJSON_RUSTC}" --print cfg
                            OUTPUT_VARIABLE YYJSON_RUSTC_CFG
                            ERROR_QUIET)
            if(YYJSON_RUSTC_CFG MATCHES "target_pointer_width=\"([0-9]+)\"")
                math(EXPR CMAKE_SIZEOF_VOID_P "${CMAKE_MATCH_1} / 8")
            endif()
        endif()
    endif()
    include(GNUInstallDirs)

    install(FILES "${CMAKE_CURRENT_BINARY_DIR}/${YYJSON_LIB_REAL}"
            DESTINATION "${CMAKE_INSTALL_LIBDIR}")
    if(BUILD_SHARED_LIBS)
        install(CODE "
            set(_libdir \"\$ENV{DESTDIR}\${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR}\")
            file(CREATE_LINK \"${YYJSON_LIB_REAL}\"
                 \"\${_libdir}/${YYJSON_LIB_SOVER}\" SYMBOLIC)
            file(CREATE_LINK \"${YYJSON_LIB_SOVER}\"
                 \"\${_libdir}/${YYJSON_LIB_LINK}\" SYMBOLIC)
        ")
    endif()
    install(FILES src/yyjson.h DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}")

    # The CMake package configuration, byte-for-byte equivalent to what
    # `install(EXPORT yyjson-targets FILE yyjson-config.cmake NAMESPACE
    # yyjson::)` generated for the C target.
    if(BUILD_SHARED_LIBS)
        set(YYJSON_IMPORTED_TYPE SHARED)
        set(YYJSON_IMPORTED_FILE "${YYJSON_LIB_REAL}")
        set(YYJSON_IMPORTED_PROPERTIES
"  IMPORTED_LOCATION_@YYJSON_CONFIG_UPPER@ \"\${_IMPORT_PREFIX}/${CMAKE_INSTALL_LIBDIR}/${YYJSON_LIB_REAL}\"
  IMPORTED_SONAME_@YYJSON_CONFIG_UPPER@ \"${YYJSON_LIB_SOVER}\"
")
    else()
        set(YYJSON_IMPORTED_TYPE STATIC)
        set(YYJSON_IMPORTED_FILE "${YYJSON_LIB_REAL}")
        set(YYJSON_IMPORTED_PROPERTIES
"  IMPORTED_LINK_INTERFACE_LANGUAGES_@YYJSON_CONFIG_UPPER@ \"C\"
  IMPORTED_LOCATION_@YYJSON_CONFIG_UPPER@ \"\${_IMPORT_PREFIX}/${CMAKE_INSTALL_LIBDIR}/${YYJSON_LIB_REAL}\"
")
    endif()

    if(CMAKE_BUILD_TYPE)
        set(YYJSON_CONFIG_NAME "${CMAKE_BUILD_TYPE}")
    else()
        set(YYJSON_CONFIG_NAME "noconfig")
    endif()
    string(TOUPPER "${YYJSON_CONFIG_NAME}" YYJSON_CONFIG_UPPER)
    string(TOLOWER "${YYJSON_CONFIG_NAME}" YYJSON_CONFIG_LOWER)
    string(CONFIGURE "${YYJSON_IMPORTED_PROPERTIES}"
           YYJSON_IMPORTED_PROPERTIES @ONLY)

    configure_file(cmake/yyjson-config.cmake.in
                   "${CMAKE_CURRENT_BINARY_DIR}/yyjson-config.cmake" @ONLY)
    configure_file(cmake/yyjson-config-cfg.cmake.in
                   "${CMAKE_CURRENT_BINARY_DIR}/yyjson-config-${YYJSON_CONFIG_LOWER}.cmake"
                   @ONLY)
    install(FILES
            "${CMAKE_CURRENT_BINARY_DIR}/yyjson-config.cmake"
            "${CMAKE_CURRENT_BINARY_DIR}/yyjson-config-${YYJSON_CONFIG_LOWER}.cmake"
            DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/yyjson")

    configure_file(yyjson.pc.in ${CMAKE_BINARY_DIR}/yyjson.pc @ONLY)
    install(FILES ${CMAKE_BINARY_DIR}/yyjson.pc DESTINATION ${CMAKE_INSTALL_LIBDIR}/pkgconfig)
endif()



# ------------------------------------------------------------------------------
# Doxygen
if(YYJSON_BUILD_DOC)
    find_package(Doxygen)
    if(DOXYGEN_FOUND)
        message(STATUS "Doxygen found")

        set(DOXYGEN_IN "${CMAKE_CURRENT_SOURCE_DIR}/doc/doxygen/Doxyfile.in")
        set(DOXYGEN_OUT "${CMAKE_CURRENT_BINARY_DIR}/Doxyfile")
        configure_file(${DOXYGEN_IN} ${DOXYGEN_OUT} @ONLY)
        message(STATUS "Doxygen will output to ${CMAKE_CURRENT_BINARY_DIR}/doxygen/html")

        add_custom_target(yyjson_doc ALL
            COMMAND ${DOXYGEN_EXECUTABLE} "${CMAKE_CURRENT_BINARY_DIR}/Doxyfile"
            WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
            COMMENT "Generating documentation with Doxygen"
            VERBATIM)
    else()
        message(WARNING "Doxygen not found")
    endif()
endif()
