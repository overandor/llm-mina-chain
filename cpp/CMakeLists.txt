cmake_minimum_required(VERSION 3.14)
project(llm-mina-chain CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find OpenSSL for SHA256
find_package(OpenSSL REQUIRED)

# Include directories
include_directories(${OPENSSL_INCLUDE_DIR})
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)

# Source files
set(SOURCES
    src/core.cpp
    src/llm_layer.cpp
    src/node.cpp
)

# Create library
add_library(llm_mina_chain STATIC
    src/core.cpp
    src/llm_layer.cpp
)

target_link_libraries(llm_mina_chain ${OPENSSL_LIBRARIES})

# Create executable
add_executable(llm-mina-node src/node.cpp)
target_link_libraries(llm-mina-node llm_mina_chain ${OPENSSL_LIBRARIES})
