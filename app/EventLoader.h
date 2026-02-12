#ifndef EVENTLOADER_H
#define EVENTLOADER_H

#include <QObject>
#include <QVariantList>
#include <QVariantMap>
#include <QString>
#include <msgpack.hpp>

class EventLoader : public QObject
{
    Q_OBJECT

public:
    explicit EventLoader(QObject *parent = nullptr);

    Q_INVOKABLE QVariantList loadEvents(const QString &filePath);

private:
    QString parseEventType(const msgpack::object& eventTypeObj);
    QString parseOptionalString(const msgpack::object& obj);

    QVariantMap parseEventAsMap(const msgpack::object& event_obj);
    QVariantMap parseEventAsArray(const msgpack::object& event_obj);

    void parseEventData(const msgpack::object& data_obj, QVariantMap& event);
    void parseEventDataAsArray(const msgpack::object& data_obj, QVariantMap& event);
};

#endif // EVENTLOADER_H
