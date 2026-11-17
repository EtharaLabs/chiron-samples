# CANARY-BEGIN
# CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
# CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
# CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
# CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
# CANARY-END
# Copyright (C) 2019 Yaoyuan <ibireme@gmail.com>.
# Released under the MIT License:
# https://github.com/ibireme/yyjson/blob/master/LICENSE

cmake_minimum_required(VERSION 3.5...3.31)
project(yyjson VERSION 0.13.0 LANGUAGES NONE)
set(YYJSON_SOVERSION 0)



# ------------------------------------------------------------------------------
# Implementation language
#
# The library is implemented in Rust and built by cargo, so no C, C++ or
# assembly compiler is enabled and `project()` declares no languages. The
# installed artifacts, their names, the exported symbols, the SONAME, the
# public header and every observable behaviour are unchanged.

find_program(YYJSON_CARGO_EXECUTABLE NAMES cargo)
if(NOT YYJSON_CARGO_EXECUTABLE)
    message(FATAL_ERROR "cargo was not found; it builds the library")
endif()



# ------------------------------------------------------------------------------
# Build Options
option(YYJSON_INSTALL "Generate installation target" ON)



# ------------------------------------------------------------------------------
# Compilation options, see yyjson.h for more explanation
#
# The Rust implementation provides the default configuration only. Each option
# below is still declared, so a consumer's cache entries and scripts keep
# working, but a non-default value is refused rather than silently ignored.
set(YYJSON_FEATURE_OPTIONS
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
foreach(opt IN LISTS YYJSON_FEATURE_OPTIONS)
    option(${opt} "Not supported by this implementation" OFF)
    if(${opt})
        message(FATAL_ERROR
                "${opt} is not supported by this implementation of yyjson")
    endif()
endforeach()

set(YYJSON_READER_DEPTH_LIMIT "" CACHE STRING "Not supported, leave empty")
set(YYJSON_WRITER_DEPTH_LIMIT "" CACHE STRING "Not supported, leave empty")
foreach(lim YYJSON_READER_DEPTH_LIMIT YYJSON_WRITER_DEPTH_LIMIT)
    if(${lim})
        message(FATAL_ERROR
                "${lim} is not supported by this implementation of yyjson")
    endif()
endforeach()



# ------------------------------------------------------------------------------
# Build
#
# cargo builds both crate types from one invocation, so the build runs once and
# the configuration below selects the artifact this build installs.

set(YYJSON_CARGO_DIR "${CMAKE_CURRENT_BINARY_DIR}/cargo")
if(BUILD_SHARED_LIBS)
    set(YYJSON_CARGO_ARTIFACT "${YYJSON_CARGO_DIR}/release/libyyjson.so")
    set(YYJSON_LIBRARY_FILE "libyyjson.so.${PROJECT_VERSION}")
else()
    set(YYJSON_CARGO_ARTIFACT "${YYJSON_CARGO_DIR}/release/libyyjson.a")
    set(YYJSON_LIBRARY_FILE "libyyjson.a")
endif()

file(GLOB YYJSON_RUST_SOURCES "${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs")

add_custom_command(
    OUTPUT "${YYJSON_CARGO_ARTIFACT}"
    COMMAND "${CMAKE_COMMAND}" -E env
            "CARGO_TARGET_DIR=${YYJSON_CARGO_DIR}"
            "CARGO_HOME=${YYJSON_CARGO_DIR}/home"
            "CARGO_NET_OFFLINE=true"
            "${YYJSON_CARGO_EXECUTABLE}" build --release --offline --locked
            --manifest-path "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    DEPENDS ${YYJSON_RUST_SOURCES}
            "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
            "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
            "${CMAKE_CURRENT_SOURCE_DIR}/.cargo/config.toml"
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    COMMENT "Building yyjson with cargo"
    VERBATIM)

add_custom_target(yyjson ALL DEPENDS "${YYJSON_CARGO_ARTIFACT}")



# ------------------------------------------------------------------------------
# Install

if(YYJSON_INSTALL)
    include(GNUInstallDirs)

    if(BUILD_SHARED_LIBS)
        install(PROGRAMS "${YYJSON_CARGO_ARTIFACT}"
                DESTINATION "${CMAKE_INSTALL_LIBDIR}"
                RENAME "${YYJSON_LIBRARY_FILE}")
        set(YYJSON_SONAME_FILE "libyyjson.so.${YYJSON_SOVERSION}")
        install(CODE "
            set(libdir \"\$ENV{DESTDIR}\${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR}\")
            file(CREATE_LINK \"${YYJSON_LIBRARY_FILE}\"
                 \"\${libdir}/${YYJSON_SONAME_FILE}\" SYMBOLIC)
            file(CREATE_LINK \"${YYJSON_SONAME_FILE}\"
                 \"\${libdir}/libyyjson.so\" SYMBOLIC)
        ")
        set(YYJSON_IMPORTED_KIND SHARED)
        set(YYJSON_IMPORTED_EXTRA_PROPERTY
            "  IMPORTED_SONAME_RELEASE \"${YYJSON_SONAME_FILE}\"")
    else()
        install(FILES "${YYJSON_CARGO_ARTIFACT}"
                DESTINATION "${CMAKE_INSTALL_LIBDIR}"
                RENAME "${YYJSON_LIBRARY_FILE}")
        set(YYJSON_IMPORTED_KIND STATIC)
        set(YYJSON_IMPORTED_EXTRA_PROPERTY
            "  IMPORTED_LINK_INTERFACE_LANGUAGES_RELEASE \"C\"")
    endif()

    set(YYJSON_INSTALL_LIBDIR "${CMAKE_INSTALL_LIBDIR}")
    set(YYJSON_INSTALL_INCLUDEDIR "${CMAKE_INSTALL_INCLUDEDIR}")

    install(FILES src/yyjson.h DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}")

    # The package configuration files the export set used to generate. They are
    # written here because no CMake language is enabled and therefore no
    # library target exists to export.
    configure_file(cmake/yyjson-config.cmake.in
                   "${CMAKE_BINARY_DIR}/yyjson-config.cmake" @ONLY)
    configure_file(cmake/yyjson-config-release.cmake.in
                   "${CMAKE_BINARY_DIR}/yyjson-config-release.cmake" @ONLY)
    install(FILES "${CMAKE_BINARY_DIR}/yyjson-config.cmake"
                  "${CMAKE_BINARY_DIR}/yyjson-config-release.cmake"
            DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/yyjson")

    configure_file(yyjson.pc.in "${CMAKE_BINARY_DIR}/yyjson.pc" @ONLY)
    install(FILES "${CMAKE_BINARY_DIR}/yyjson.pc"
            DESTINATION "${CMAKE_INSTALL_LIBDIR}/pkgconfig")
endif()
