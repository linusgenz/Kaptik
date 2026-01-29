//
// Created by Linus Genz on 28.01.2026.
// Copyright (c) 2026 Linus Genz. All rights reserved.
//

#include "IpcClient.h"
#include <iostream>

IpcClient::IpcClient(QObject* parent) : QObject(parent) {}

bool IpcClient::connectPipe() {
    if (m_pipe != INVALID_HANDLE_VALUE)
        return true;

    while (true) {
        m_pipe = CreateFileA(
            R"(\\.\pipe\kaptik_pipe)",
            GENERIC_WRITE,
            0,
            nullptr,
            OPEN_EXISTING,
            0,
            nullptr
            );

        if (m_pipe != INVALID_HANDLE_VALUE) {
            std::cout << "Connected to recorder pipe.\n";
            return true;
        }

        DWORD err = GetLastError();

        if (err == ERROR_PIPE_BUSY) {
            if (!WaitNamedPipeA(R"(\\.\pipe\kaptik_pipe)", 3000)) {
                std::cerr << "Pipe busy timeout.\n";
                return false;
            }
        }
        else if (err == ERROR_FILE_NOT_FOUND) {
            std::cerr << "Recorder not running (pipe not found).\n";
            return false;
        }
        else {
            std::cerr << "CreateFile failed with error: " << err << "\n";
            return false;
        }
    }
}

void IpcClient::disconnectPipe() {
    if (m_pipe != INVALID_HANDLE_VALUE) {
        CloseHandle(m_pipe);
        m_pipe = INVALID_HANDLE_VALUE;
    }
}

bool IpcClient::writeMessage(const QByteArray& payload) {
    if (m_pipe == INVALID_HANDLE_VALUE)
        return false;

    uint32_t size = payload.size();

    QByteArray packet;
    packet.append(reinterpret_cast<const char*>(&size), sizeof(uint32_t));
    packet.append(payload);

    DWORD written = 0;
    BOOL ok = WriteFile(m_pipe, packet.constData(), DWORD(packet.size()), &written, nullptr);

    if (!ok || written != packet.size()) {
        std::cerr << "Pipe write failed. Disconnecting...\n";
        disconnectPipe();
        return false;
    }

    return true;
}

void IpcClient::sendStartRecording() {
    Command cmd;
    cmd.type = CommandType::StartRecording;

    msgpack::sbuffer buffer;
    msgpack::pack(buffer, cmd);
    writeMessage(QByteArray(buffer.data(), int(buffer.size())));
}

void IpcClient::sendStopRecording() {
    Command cmd;
    cmd.type = CommandType::StopRecording;

    msgpack::sbuffer buffer;
    msgpack::pack(buffer, cmd);
    writeMessage(QByteArray(buffer.data(), int(buffer.size())));
}

void IpcClient::sendUpdateSetting(const QString& key, const QString& value) {
    Command cmd;
    cmd.type = CommandType::UpdateSetting;
    cmd.update = UpdateSetting{ key.toStdString(), value.toStdString() };

    msgpack::sbuffer buffer;
    msgpack::pack(buffer, cmd);
    writeMessage(QByteArray(buffer.data(), int(buffer.size())));
}

