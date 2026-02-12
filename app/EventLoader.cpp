#include "EventLoader.h"
#include <msgpack.hpp>
#include <QDebug>
#include <fstream>
#include <vector>

EventLoader::EventLoader(QObject *parent)
    : QObject(parent)
{
}

QVariantList EventLoader::loadEvents(const QString &filePath)
{
    QVariantList events;

    try {
        std::ifstream ifs(filePath.toStdString(), std::ios::binary);
        if (!ifs.is_open()) {
            qWarning() << "Could not open events file:" << filePath;
            return events;
        }

        std::vector<char> buffer(
            (std::istreambuf_iterator<char>(ifs)),
            std::istreambuf_iterator<char>()
            );
        ifs.close();

        if (buffer.empty()) {
            qWarning() << "Events file is empty:" << filePath;
            return events;
        }

        msgpack::object_handle oh = msgpack::unpack(buffer.data(), buffer.size());
        msgpack::object obj = oh.get();

        // Root = ARRAY [game_name, recording_id, timestamp, events]
        if (obj.type != msgpack::type::ARRAY) {
            qWarning() << "Invalid root format — expected ARRAY, got:" << (int)obj.type;
            return events;
        }

        const auto* root_arr = &obj.via.array;

        if (root_arr->size < 4) {
            qWarning() << "Invalid root array size:" << root_arr->size;
            return events;
        }

        // index 3 = events array
        const msgpack::object& events_obj = root_arr->ptr[3];

        if (events_obj.type != msgpack::type::ARRAY) {
            qWarning() << "Events field is not an array";
            return events;
        }

        const auto* events_arr = &events_obj.via.array;
        qDebug() << "Found" << events_arr->size << "events in file";

        for (uint32_t j = 0; j < events_arr->size; ++j) {
            const msgpack::object& event_obj = events_arr->ptr[j];

            QVariantMap event;

            // Try MAP format first (struct with field names)
            if (event_obj.type == msgpack::type::MAP) {
                event = parseEventAsMap(event_obj);
            }
            // Try ARRAY format (tuple struct)
            else if (event_obj.type == msgpack::type::ARRAY) {
                event = parseEventAsArray(event_obj);
            }
            else {
                qWarning() << "Event" << j << "is neither MAP nor ARRAY, type:" << (int)event_obj.type;
                continue;
            }

            if (event.contains("timestamp") && event.contains("event_type")) {

                qDebug() << "Event:"
                         << event["event_type"].toString()
                         << "| Name:" << event.value("name").toString()
                         << "| Timestamp:" << event["timestamp"];

                events.append(event);
            }
             else {
                qDebug() << "Event" << j << "missing required fields";
            }
        }

        qDebug() << "Loaded" << events.size() << "events from" << filePath;

        if (!events.isEmpty()) {
            QVariantMap first = events.first().toMap();
            QVariantMap last = events.last().toMap();

            qDebug() << "First event:" << first["event_type"]
                     << "at" << first["timestamp"].toDouble() << "ms";
            qDebug() << "Last event:" << last["event_type"]
                     << "at" << last["timestamp"].toDouble() << "ms";
        }

    }
    catch (const std::exception& e) {
        qWarning() << "Error loading events:" << e.what();
    }

    return events;
}

QVariantMap EventLoader::parseEventAsMap(const msgpack::object& event_obj)
{
    QVariantMap event;
    const auto* event_map = &event_obj.via.map;

    for (uint32_t k = 0; k < event_map->size; ++k) {
        std::string field_key;
        event_map->ptr[k].key.convert(field_key);
        const msgpack::object& field_val = event_map->ptr[k].val;

        if (field_key == "event_id") {
            uint32_t event_id;
            field_val.convert(event_id);
            event["event_id"] = static_cast<int>(event_id);
        }
        else if (field_key == "timestamp") {
            double timestampSeconds;
            field_val.convert(timestampSeconds);
            qint64 timestampMs = static_cast<qint64>(timestampSeconds * 1000.0);
            event["timestamp"] = timestampMs;
        }
        else if (field_key == "event_type") {
            event["event_type"] = parseEventType(field_val);
        }
        else if (field_key == "data") {
            if (field_val.type == msgpack::type::MAP) {
                parseEventData(field_val, event);
            } else if (field_val.type == msgpack::type::ARRAY) {
                parseEventDataAsArray(field_val, event);
            }
        }
    }

    return event;
}

QVariantMap EventLoader::parseEventAsArray(const msgpack::object& event_obj)
{
    // GameEvent as tuple: [event_id, event_type, timestamp, data]
    QVariantMap event;
    const auto* event_arr = &event_obj.via.array;

    if (event_arr->size < 4) {
        qWarning() << "Event array too small:" << event_arr->size;
        return event;
    }

    // event_id
    uint32_t event_id;
    event_arr->ptr[0].convert(event_id);
    event["event_id"] = static_cast<int>(event_id);

    // event_type
    event["event_type"] = parseEventType(event_arr->ptr[1]);

    // timestamp
    double timestampSeconds;
    event_arr->ptr[2].convert(timestampSeconds);

    qint64 timestampMs = static_cast<qint64>(timestampSeconds * 1000.0);
    event["timestamp"] = timestampMs;

    // data
    const msgpack::object& data_obj = event_arr->ptr[3];
    if (data_obj.type == msgpack::type::MAP) {
        parseEventData(data_obj, event);
    } else if (data_obj.type == msgpack::type::ARRAY) {
        parseEventDataAsArray(data_obj, event);
    }

    return event;
}

void EventLoader::parseEventData(const msgpack::object& data_obj, QVariantMap& event)
{
    const auto* data_map = &data_obj.via.map;

    for (uint32_t m = 0; m < data_map->size; ++m) {
        std::string data_key;
        data_map->ptr[m].key.convert(data_key);
        const msgpack::object& data_val = data_map->ptr[m].val;

        if (data_key == "name") {
            std::string name;
            data_val.convert(name);
            event["name"] = QString::fromStdString(name);
        }
        else if (data_key == "actor") {
            event["actor"] = parseOptionalString(data_val);
        }
        else if (data_key == "target") {
            event["target"] = parseOptionalString(data_val);
        }
        else if (data_key == "metadata") {
            if (data_val.type == msgpack::type::MAP) {
                const auto* meta_map = &data_val.via.map;

                for (uint32_t n = 0; n < meta_map->size; ++n) {
                    std::string meta_key;
                    meta_map->ptr[n].key.convert(meta_key);

                    if (meta_key == "is_highlight") {
                        bool is_highlight;
                        meta_map->ptr[n].val.convert(is_highlight);
                        event["is_highlight"] = is_highlight;
                    }
                }
            }
        }
    }
}

void EventLoader::parseEventDataAsArray(const msgpack::object& data_obj, QVariantMap& event)
{
    // EventData as tuple: [name, actor, target, participants, metadata]
    const auto* data_arr = &data_obj.via.array;

    if (data_arr->size < 5) return;

    // name
    std::string name;
    data_arr->ptr[0].convert(name);
    event["name"] = QString::fromStdString(name);

    // actor
    event["actor"] = parseOptionalString(data_arr->ptr[1]);

    // target
    event["target"] = parseOptionalString(data_arr->ptr[2]);

    // metadata (index 4)
    const msgpack::object& meta_obj = data_arr->ptr[4];

    if (meta_obj.type == msgpack::type::MAP) {
        const auto* meta_map = &meta_obj.via.map;
        for (uint32_t n = 0; n < meta_map->size; ++n) {
            std::string meta_key;
            meta_map->ptr[n].key.convert(meta_key);

            if (meta_key == "is_highlight") {
                bool is_highlight;
                meta_map->ptr[n].val.convert(is_highlight);
                event["is_highlight"] = is_highlight;
            }
        }
    }
    else if (meta_obj.type == msgpack::type::ARRAY) {
        // EventMetadata as tuple - just get is_highlight for now
        const auto* meta_arr = &meta_obj.via.array;
        if (meta_arr->size > 3) {
            bool is_highlight;
            meta_arr->ptr[3].convert(is_highlight);
            event["is_highlight"] = is_highlight;
        }
    }
}

QString EventLoader::parseEventType(const msgpack::object& eventTypeObj)
{
    if (eventTypeObj.type == msgpack::type::STR) {
        std::string type_str;
        eventTypeObj.convert(type_str);
        return QString::fromStdString(type_str);
    }
    else if (eventTypeObj.type == msgpack::type::MAP) {
        const msgpack::object_map* type_map = &eventTypeObj.via.map;

        if (type_map->size > 0) {
            std::string variant_name;
            type_map->ptr[0].key.convert(variant_name);

            if (variant_name == "Custom") {
                std::string custom_value;
                type_map->ptr[0].val.convert(custom_value);
                return QString::fromStdString(custom_value);
            }

            return QString::fromStdString(variant_name);
        }
    }

    return "Unknown";
}

QString EventLoader::parseOptionalString(const msgpack::object& obj)
{
    if (obj.type == msgpack::type::NIL) {
        return "";
    }
    else if (obj.type == msgpack::type::STR) {
        std::string str;
        obj.convert(str);
        return QString::fromStdString(str);
    }

    return "";
}
