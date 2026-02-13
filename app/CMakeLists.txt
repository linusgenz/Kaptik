cmake_minimum_required(VERSION 3.24)
project(Kaptik VERSION 0.1 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

enable_language(RC)

find_package(Qt6 REQUIRED COMPONENTS Quick QuickControls2 Svg Multimedia Core5Compat)

qt_standard_project_setup(REQUIRES 6.8)

if(WIN32)
    set(APP_ICON_RESOURCE "${CMAKE_CURRENT_SOURCE_DIR}/resources.rc")
endif()

set(RESOURCES
    resources.qrc
)

qt_add_executable(appKaptik
    WIN32
    main.cpp
    settingsmanager.h
    clipmodel.h
    clipmodel.cpp
    win/videometadata.cpp
    win/videometadata.h
    thumbnailprovider.h
    IpcClient.cpp
    IpcClient.h
    ApmLoader.cpp
    ApmLoader.h
    EventLoader.cpp
    EventLoader.h
    ${RESOURCES}
    ${APP_ICON_RESOURCE}
)


qt_add_qml_module(appKaptik
    URI Kaptik
    QML_FILES
        Main.qml
        GridPage.qml
        PlayerPage.qml
        SettingsPage.qml
        SettingsSection.qml
        SettingsRow.qml
        SettingsComboBox.qml
        SettingsSlider.qml
        BaseRoundButton.qml
        ApmGraphView.qml
        EventMarkers.qml
    RESOURCES
        resources.qrc
)

set(LIBS_DIR "G:/Kaptik/app/libs")
target_include_directories(appKaptik PRIVATE ${LIBS_DIR})

target_link_libraries(appKaptik PRIVATE
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::Svg
    Qt6::Multimedia
    Qt6::Core5Compat
    propsys
    ole32
    shell32
)

if(MINGW)
    target_link_libraries(appKaptik PRIVATE
        -lkernel32
        -luser32
        -lgdi32
        -lwinspool
        -lole32
        -loleaut32
        -luuid
        -lcomdlg32
        -ladvapi32
    )
endif()

set_target_properties(appKaptik PROPERTIES
    OUTPUT_NAME "kaptik-ui"
    WIN32_EXECUTABLE TRUE
)


include(GNUInstallDirs)
install(TARGETS appKaptik
    BUNDLE DESTINATION .
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

if(EXISTS "${CMAKE_SOURCE_DIR}/cmake/DeployQt.cmake")
    include("${CMAKE_SOURCE_DIR}/cmake/DeployQt.cmake")

elseif(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/cmake/DeployQt.cmake")
    include("${CMAKE_CURRENT_SOURCE_DIR}/cmake/DeployQt.cmake")

endif()

if(COMMAND deploy_qt)
    deploy_qt(appKaptik)
endif()


