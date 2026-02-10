cmake --preset release
cmake --build --preset release
cmake --install build/release
cpack --config build/release/CPackConfig.cmake
