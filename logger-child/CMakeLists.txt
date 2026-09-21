# Optional; install.ps1 compiles src/main.cpp with cl.exe directly.
cmake_minimum_required(VERSION 3.15)
project(logger-child CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_executable(logger-child src/main.cpp)

if(MSVC)
    target_compile_options(logger-child PRIVATE /W4 /permissive-)
    # Ask for a UTF-8 source and execution charset so the log lines are not
    # mangled on non-English systems.
    target_compile_options(logger-child PRIVATE /utf-8)
else()
    target_compile_options(logger-child PRIVATE -Wall -Wextra -Wpedantic)
endif()

# GCC before 9 needs an explicit link for std::filesystem.
if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU" AND CMAKE_CXX_COMPILER_VERSION VERSION_LESS 9.0)
    target_link_libraries(logger-child PRIVATE stdc++fs)
endif()
