find_program(WINDEPLOYQT_EXECUTABLE windeployqt
    HINTS "${Qt6_DIR}/../../../bin"
    REQUIRED
)

function(deploy_qt target)
    install(CODE "
        execute_process(
            COMMAND \"${WINDEPLOYQT_EXECUTABLE}\" --release \"\$<TARGET_FILE:${target}>\"
        )
    ")
endfunction()
