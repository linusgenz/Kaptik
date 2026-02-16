#ifndef DATALOADER_H
#define DATALOADER_H

#include <QObject>
#include <QVariantList>
#include <QVariantMap>
#include <msgpack.hpp>

class DataLoader : public QObject
{
    Q_OBJECT
public:
    explicit DataLoader(QObject *parent = nullptr);

    Q_INVOKABLE QVariantMap loadRecordingData(const QString &filePath);

private:
    QVariantMap parseMetadata(const msgpack::object& obj);
    QVariantList parseApmData(const msgpack::object& obj);
    QVariantList parseEvents(const msgpack::object& obj);

    QString parseEventType(const msgpack::object& eventTypeObj);
    QString parseOptionalString(const msgpack::object& obj);
    QVariantMap parseEventAsMap(const msgpack::object& event_obj);
    QVariantMap parseEventAsArray(const msgpack::object& event_obj);
    void parseEventData(const msgpack::object& data_obj, QVariantMap& event);
    void parseEventDataAsArray(const msgpack::object& data_obj, QVariantMap& event);
};

#endif // DATALOADER_H
