cmake_minimum_required(VERSION 3.24)
project(KaptikSuite VERSION 0.3 LANGUAGES CXX)

add_subdirectory(app)

include(ExternalProject)

# --- kaptik-core ---
ExternalProject_Add(build_core
    SOURCE_DIR ${CMAKE_SOURCE_DIR}/kaptik-core
    CONFIGURE_COMMAND ""
    BUILD_COMMAND cargo build --release
    BUILD_IN_SOURCE 1
    INSTALL_COMMAND ""
    BUILD_BYPRODUCTS ${CMAKE_SOURCE_DIR}/kaptik-core/target/release/kaptik-core.exe
)

add_dependencies(appKaptik build_core)

install(PROGRAMS
    ${CMAKE_SOURCE_DIR}/kaptik-core/target/release/kaptik-core.exe
    DESTINATION ${CMAKE_INSTALL_BINDIR}
)
install(FILES ${CMAKE_SOURCE_DIR}/assets/kaptik_logo.ico
        DESTINATION ${CMAKE_INSTALL_BINDIR}/assets)

include(InstallRequiredSystemLibraries)

set(CPACK_PACKAGE_NAME "Kaptik")
set(CPACK_PACKAGE_VENDOR "Linus Genz")
set(CPACK_PACKAGE_VERSION ${PROJECT_VERSION})

set(CPACK_PACKAGE_INSTALL_DIRECTORY "Kaptik")

set(CPACK_GENERATOR "NSIS")

set(CPACK_NSIS_DISPLAY_NAME "Kaptik")
set(CPACK_NSIS_PACKAGE_NAME "Kaptik")
set(CPACK_NSIS_CONTACT "linuslinuxgenz@gmail.com")
set(CPACK_NSIS_ENABLE_UNINSTALL_BEFORE_INSTALL ON)

set(CPACK_NSIS_CREATE_ICONS_EXTRA
  "CreateDirectory '$SMPROGRAMS\\\\Kaptik'
   CreateShortCut '$SMPROGRAMS\\\\Kaptik\\\\Kaptik.lnk' '$INSTDIR\\\\bin\\\\kaptik-core.exe'"
)

set(CPACK_NSIS_DELETE_ICONS_EXTRA
  "Delete '$SMPROGRAMS\\\\Kaptik\\\\Kaptik.lnk'
   RMDir '$SMPROGRAMS\\\\Kaptik'"
)

set(CPACK_NSIS_URL_INFO_ABOUT "https://github.com/linusgenz/Kaptik")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_SOURCE_DIR}/LICENSE")

set(CPACK_NSIS_MUI_ICON "${CMAKE_SOURCE_DIR}/assets/kaptik_logo.ico")
set(CPACK_NSIS_MUI_UNIICON "${CMAKE_SOURCE_DIR}/assets/kaptik_logo.ico")

set(CPACK_OUTPUT_FILE_PREFIX "${CMAKE_BINARY_DIR}")

set(CPACK_CREATE_DESKTOP_LINKS kaptik)
set(CPACK_NSIS_MODIFY_PATH ON)

include(CPack)
