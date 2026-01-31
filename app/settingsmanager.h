//
// Created by Linus Genz on 24.01.2026.
// Copyright (c) 2026 Linus Genz. All rights reserved.
//

#ifndef KAPTIK_SETTINGSMANAGER_H
#define KAPTIK_SETTINGSMANAGER_H

#include <QObject>
#include <QVariant>
#include <QFile>
#include <QDir>
#include <qqml.h>
#include <iostream>
#include <map>
#include "libs/toml.hpp"
#include "IpcClient.h"

class SettingsManager : public QObject {
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

public:
    explicit SettingsManager(QObject *parent = nullptr)
        : QObject(parent)
    {
        loadFromDisk();
    }

    ~SettingsManager() {
        m_ipc.disconnectPipe();
    }

    Q_INVOKABLE bool connectToRecorder() {
        std::cout << "connectToRecorder\n";
        if (!m_ipc.connectPipe()) {
            std::cerr << "IPC not available (recorder not running?)\n";
            return false;
        }
        return true;
    }

    enum Key {
        Key_DarkMode,
        Key_VideoPath,
        Key_Resolution,
        Key_FpsLimit,
        Key_GameAudio,
        Key_Microphone,
        Key_SystemSounds
    };
    Q_ENUM(Key)

    enum Resolution {
        Resolution720p,
        Resolution1080p,
        Resolution1440p,
        Resolution4K,
        ResolutionSource
    };
    Q_ENUM(Resolution)

    enum Fps {
        Fps30,
        Fps60,
        Fps120
    };
    Q_ENUM(Fps)

    Q_INVOKABLE QVariant value(Key key) const {
        auto it = m_data.find(key);
        if (it != m_data.end())
            return it->second;
        return defaultValue(key);
    }

    Q_INVOKABLE void setValue(Key key, const QVariant &value) {
        m_data[key] = value;

        QString keyStr = keyToString(key);
        QString valueStr;

        if (value.metaType().id() == QMetaType::Bool)
            valueStr = value.toBool() ? "true" : "false";
        else if (value.metaType().id() == QMetaType::Int)
            valueStr = QString::number(value.toInt());
        else
            valueStr = value.toString();

        m_ipc.sendUpdateSetting(keyStr, valueStr);

        saveToDisk();
        emit settingChanged(key, value);
    }


signals:
    void settingChanged(Key key, QVariant value);

private:
    IpcClient m_ipc;
    std::map<Key, QVariant> m_data;
    QString m_filePath;

    QString keyToString(Key key) const {
        switch (key) {
        case Key_DarkMode:     return "dark_mode";
        case Key_VideoPath:    return "video_path";
        case Key_Resolution:   return "resolution";
        case Key_FpsLimit:     return "fps_limit";
        case Key_GameAudio:    return "game_audio";
        case Key_Microphone:   return "microphone";
        case Key_SystemSounds: return "system_sounds";
        }
        return {};
    }

    QVariant defaultValue(Key key) const {
        switch (key) {
        case Key_DarkMode:     return false;
        case Key_VideoPath:    return "";
        case Key_Resolution:   return Resolution1080p;
        case Key_FpsLimit:     return Fps60;
        case Key_GameAudio:    return true;
        case Key_Microphone:   return true;
        case Key_SystemSounds: return false;
        }
        return {};
    }

    void loadFromDisk() {
        QDir dir(QDir::homePath() + "/AppData/Roaming/Kaptik");
        if (!dir.exists())
            dir.mkpath(".");

        m_filePath = dir.filePath("settings.toml");

        if (!QFile::exists(m_filePath))
            return;

        try {
            auto tbl = toml::parse_file(m_filePath.toStdString());

            for (int k = Key_DarkMode; k <= Key_SystemSounds; ++k) {
                auto key = keyToString(static_cast<Key>(k));
                if (tbl.contains(key.toStdString())) {
                    const auto& val = tbl[key.toStdString()];
                    if (val.is_boolean())
                        m_data[static_cast<Key>(k)] = val.value_or(false);
                    else if (val.is_integer())
                        m_data[static_cast<Key>(k)] = static_cast<int>(val.value_or(0));
                    else if (val.is_string())
                        m_data[static_cast<Key>(k)] = QString::fromStdString(val.value_or(""));
                }
            }
        } catch (const toml::parse_error& err) {
            std::cerr << "Failed to parse settings.toml: " << err.description() << std::endl;
        }
    }

    void saveToDisk() {
        toml::table tbl;

        for (const auto& [k, v] : m_data) {
            auto keyQString = keyToString(k);
            std::string key = keyQString.toStdString();

            if (v.metaType().id() == QMetaType::Bool)
                tbl.insert(key, v.toBool());
            else if (v.metaType().id() == QMetaType::Int)
                tbl.insert(key, v.toInt());
            else if (v.metaType().id() == QMetaType::QString)
                tbl.insert(key, v.toString().toStdString());
        }

        std::ofstream ofs(m_filePath.toStdString());
        ofs << tbl;
    }
};

#endif // KAPTIK_SETTINGSMANAGER_H
