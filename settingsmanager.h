//
// Created by Linus Genz on 24.01.2026.
// Copyright (c) 2026 Linus Genz. All rights reserved.
//

#ifndef KAPTIK_SETTINGSMANAGER_H
#define KAPTIK_SETTINGSMANAGER_H

#include <QSettings>
#include <qqml.h>

class SettingsManager : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

    QSettings m_settings {"LinusGenz", "Kaptik"};

public:
    Q_INVOKABLE bool loadDarkMode() const {
        return m_settings.value("darkMode", false).toBool();
    }

    Q_INVOKABLE void saveDarkMode(bool enabled) {
        m_settings.setValue("darkMode", enabled);
    }

    Q_INVOKABLE QString loadVideoPath() const {
        return m_settings.value("videoPath", "").toString();
    }

    Q_INVOKABLE void saveVideoPath(const QString &path) {
        m_settings.setValue("videoPath", path);
    }
};

#endif //KAPTIK_SETTINGSMANAGER_H
