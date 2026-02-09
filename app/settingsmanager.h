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
#include <QMediaDevices>
#include <QAudioDevice>

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
        if (!m_ipc.connectPipe()) {
            std::cerr << "IPC not available (recorder not running?)\n";
            return false;
        }
        m_ipc.startListening();

        QObject::connect(&m_ipc, &IpcClient::shutdownReceived,
                         this, &SettingsManager::recorderShutdown);

        return true;
    }

    enum Key {
        Key_DarkMode,
        Key_VideoPath,
        Key_Resolution,
        Key_FpsLimit,
        Key_GameAudio,
        Key_Microphone,
        Key_SystemSounds,
        Key_TonemapAlgorithm,
        Key_HdrNitsMode,
        Key_SelectedMicrophone,
        Key_SelectedOutput,
        Key_MicrophoneVolume,
        Key_OutputVolume
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

    enum TonemapAlgorithm {
        Reinhard,
        AcesSimple,
        AcesFitted,
        Uncharted2,
        HejlDawson
    };
    Q_ENUM(TonemapAlgorithm)

    enum HdrNitsMode {
        HdrNitsAuto,
        HdrNits1000,
        HdrNits2000,
        HdrNits4000,
        HdrNits10000
    };
    Q_ENUM(HdrNitsMode)

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

        switch (key) {
        case Key_Resolution:
            valueStr = enumToString(static_cast<Resolution>(value.toInt()));
            break;
        case Key_FpsLimit:
            valueStr = enumToString(static_cast<Fps>(value.toInt()));
            break;
        case Key_TonemapAlgorithm:
            valueStr = enumToString(static_cast<TonemapAlgorithm>(value.toInt()));
            break;
        case Key_HdrNitsMode:
            valueStr = enumToString(static_cast<HdrNitsMode>(value.toInt()));
            break;

        default:
            if (value.metaType().id() == QMetaType::Bool)
                valueStr = value.toBool() ? "true" : "false";
            else if (value.metaType().id() == QMetaType::Int)
                valueStr = QString::number(value.toInt());
            else
                valueStr = value.toString();
            break;
        }



        m_ipc.sendUpdateSetting(keyStr, valueStr);
        saveToDisk();
        emit settingChanged(key, value);
    }

    Q_INVOKABLE QVariantList availableMicrophones() const {
        QVariantList list;
        for (const QAudioDevice &dev : QMediaDevices::audioInputs()) {
            QVariantMap item;
            item["text"] = dev.description();
            item["value"] = QString::fromUtf8(dev.id());
            list.append(item);
        }
        return list;
    }

    Q_INVOKABLE QVariantList availableOutputs() const {
        QVariantList list;
        for (const QAudioDevice &dev : QMediaDevices::audioOutputs()) {
            QVariantMap item;
            item["text"] = dev.description();
            item["value"] = QString::fromUtf8(dev.id());
            list.append(item);
        }
        return list;
    }


signals:
    void settingChanged(Key key, QVariant value);
    void recorderShutdown();

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
        case Key_TonemapAlgorithm: return "tonemap_algorithm";
        case Key_HdrNitsMode:      return "hdr_nits_mode";
        case Key_SelectedMicrophone: return "input_device";
        case Key_SelectedOutput: return "output_device";
        case Key_MicrophoneVolume: return "microphone_volume";
        case Key_OutputVolume: return "output_volume";
        }
        return {};
    }

    QVariant defaultValue(Key key) const {
        switch (key) {
        case Key_DarkMode:     return false;
        case Key_VideoPath:    return QString("");
        case Key_Resolution:   return Resolution1080p;
        case Key_FpsLimit:     return Fps60;
        case Key_GameAudio:    return true;
        case Key_Microphone:   return true;
        case Key_SystemSounds: return false;
        case Key_TonemapAlgorithm: return AcesFitted;
        case Key_HdrNitsMode:      return HdrNitsAuto;
        case Key_SelectedMicrophone: return QString("");
        case Key_SelectedOutput: return QString("");
        case Key_MicrophoneVolume: return 50;
        case Key_OutputVolume: return 50;
        }
        return {};
    }

    template<typename Enum>
    QString enumToString(Enum value) const {
        const QMetaObject &mo = staticMetaObject;
        int index = mo.indexOfEnumerator(QMetaEnum::fromType<Enum>().name());
        QMetaEnum me = mo.enumerator(index);
        return QString::fromLatin1(me.valueToKey(static_cast<int>(value)));
    }

    template<typename Enum>
    Enum stringToEnum(const QString &key, Enum defaultValue) const {
        const QMetaObject &mo = staticMetaObject;
        int index = mo.indexOfEnumerator(QMetaEnum::fromType<Enum>().name());
        QMetaEnum me = mo.enumerator(index);

        int val = me.keyToValue(key.toLatin1().constData());
        if (val == -1)
            return defaultValue;

        return static_cast<Enum>(val);
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

            for (int k = Key_DarkMode; k <= Key_OutputVolume; ++k) {
                Key keyEnum = static_cast<Key>(k);
                QString key = keyToString(keyEnum);

                if (!tbl.contains(key.toStdString()))
                    continue;

                const auto& val = tbl[key.toStdString()];

                switch (keyEnum) {
                case Key_Resolution:
                    if (val.is_string())
                        m_data[keyEnum] = static_cast<int>(
                            stringToEnum<Resolution>(
                                QString::fromStdString(val.value_or("Resolution1080p")),
                                Resolution1080p));
                    break;

                case Key_FpsLimit:
                    if (val.is_string())
                        m_data[keyEnum] = static_cast<int>(
                            stringToEnum<Fps>(
                                QString::fromStdString(val.value_or("Fps60")),
                                Fps60));
                    break;

                case Key_TonemapAlgorithm:
                    if (val.is_string())
                        m_data[keyEnum] = static_cast<int>(
                            stringToEnum<TonemapAlgorithm>(
                                QString::fromStdString(val.value_or("AcesFitted")),
                                AcesFitted));
                    break;

                case Key_HdrNitsMode:
                    if (val.is_string())
                        m_data[keyEnum] = static_cast<int>(
                            stringToEnum<HdrNitsMode>(
                                QString::fromStdString(val.value_or("HdrNits_Auto")),
                                HdrNitsAuto));
                    break;

                default:
                    if (val.is_boolean())
                        m_data[keyEnum] = val.value_or(false);
                    else if (val.is_integer())
                        m_data[keyEnum] = static_cast<int>(val.value_or(0));
                    else if (val.is_string())
                        m_data[keyEnum] = QString::fromStdString(val.value_or(""));
                    break;
                }
            }

        } catch (const toml::parse_error& err) {
            std::cerr << "Failed to parse settings.toml: " << err.description() << std::endl;
        }
    }

    void saveToDisk() {
        toml::table tbl;

        for (const auto& [k, v] : m_data) {
            std::string key = keyToString(k).toStdString();

            switch (k) {
            case Key_Resolution:
                tbl.insert(key, enumToString(static_cast<Resolution>(v.toInt())).toStdString());
                break;
            case Key_FpsLimit:
                tbl.insert(key, enumToString(static_cast<Fps>(v.toInt())).toStdString());
                break;
            case Key_TonemapAlgorithm:
                tbl.insert(key, enumToString(static_cast<TonemapAlgorithm>(v.toInt())).toStdString());
                break;
            case Key_HdrNitsMode:
                tbl.insert(key, enumToString(static_cast<HdrNitsMode>(v.toInt())).toStdString());
                break;

            default:
                if (v.metaType().id() == QMetaType::Bool)
                    tbl.insert(key, v.toBool());
                else if (v.metaType().id() == QMetaType::QString)
                    tbl.insert(key, toml::value(v.toString().toStdString()));
                else if (v.metaType().id() == QMetaType::Int)
                    tbl.insert(key, v.toInt());
                break;
            }
        }

        std::ofstream ofs(m_filePath.toStdString());
        ofs << tbl;
    }
};

#endif // KAPTIK_SETTINGSMANAGER_H
