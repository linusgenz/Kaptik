//
// Created by Linus Genz on 28.01.2026.
// Copyright (c) 2026 Linus Genz. All rights reserved.
//

#ifndef IPCCLIENT_H
#define IPCCLIENT_H

#include <QObject>
#include <QByteArray>
#include <windows.h>
#include <msgpack.hpp>

enum class CommandType {
    StartRecording,
    StopRecording,
    UpdateSetting
};
MSGPACK_ADD_ENUM(CommandType);

struct StartRecording { MSGPACK_DEFINE(); };
struct StopRecording { MSGPACK_DEFINE(); };
struct UpdateSetting {
    std::string key;
    std::string value;

    MSGPACK_DEFINE(key, value);
};

struct Command {
    CommandType type;
    std::optional<UpdateSetting> update;

    MSGPACK_DEFINE(type, update);
};


class IpcClient : public QObject {
    Q_OBJECT
public:
    explicit IpcClient(QObject* parent = nullptr);

    bool connectPipe();
    void disconnectPipe();

    void sendStartRecording();
    void sendStopRecording();
    void sendUpdateSetting(const QString& key, const QString& value);

private:
    HANDLE m_pipe = INVALID_HANDLE_VALUE;
    bool writeMessage(const QByteArray& msg);
};

#endif // IPCCLIENT_H
