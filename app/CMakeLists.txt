cmake_minimum_required(VERSION 3.16)
project(Kaptik VERSION 0.1 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

find_package(Qt6 REQUIRED COMPONENTS Quick QuickControls2 Svg Multimedia Core5Compat)

qt_standard_project_setup(REQUIRES 6.8)

set(RESOURCES
    resources.qrc
)

qt_add_executable(appKaptik
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
    ${RESOURCES}
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
        RESOURCES resources.qrc
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


include(GNUInstallDirs)
install(TARGETS appKaptik
    BUNDLE DESTINATION .
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)
