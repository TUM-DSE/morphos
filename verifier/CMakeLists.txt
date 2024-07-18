cmake_minimum_required(VERSION 3.22)
project(ubpf_verifier)

set(CMAKE_CXX_STANDARD 20)

find_package(Boost 1.40 COMPONENTS program_options REQUIRED)
find_package(OpenSSL REQUIRED)

add_subdirectory(ebpf-verifier)

include_directories(.)
include_directories(./ebpf-verifier/src)
include_directories(${Boost_INCLUDE_DIR})

add_executable(ubpf_verifier main.cpp
        platform/click_platform.cpp platform/click_platform.hpp
        platform/gpl/spec_prototypes.cpp platform/gpl/spec_type_descriptors.hpp
)

target_link_libraries(ubpf_verifier ${Boost_LIBRARIES} ebpfverifier OpenSSL::SSL OpenSSL::Crypto)