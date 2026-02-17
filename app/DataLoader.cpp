#include "DataLoader.h"
#include <QDebug>
#include <QDateTime>
#include <fstream>
#include <vector>

DataLoader::DataLoader(QObject *parent)
    : QObject(parent)
{
}

QVariantMap DataLoader::loadRecordingData(const QString &filePath)
{
    QVariantMap result;
    result["apm"] = QVariantList();
    result["events"] = QVariantList();
    result["metadata"] = QVariantMap();

    try {
        std::ifstream ifs(filePath.toStdString(), std::ios::binary);
        if (!ifs.is_open()) {
            qWarning() << "Could not open recording file:" << filePath;
            return result;
        }

        std::vector<char> buffer(
            (std::istreambuf_iterator<char>(ifs)),
            std::istreambuf_iterator<char>()
            );
        ifs.close();

        if (buffer.empty()) {
            qWarning() << "Recording file is empty:" << filePath;
            return result;
        }

        msgpack::object_handle oh = msgpack::unpack(buffer.data(), buffer.size());
        msgpack::object obj = oh.get();

        // Root = ARRAY [metadata, apm, events]
        if (obj.type != msgpack::type::ARRAY) {
            qWarning() << "Invalid root format — expected ARRAY, got:" << (int)obj.type;
            return result;
        }

        const auto* root_arr = &obj.via.array;
        qDebug() << "Root array size:" << root_arr->size;

        if (root_arr->size < 3) {
            qWarning() << "Invalid root array size:" << root_arr->size;
            return result;
        }

        // Index 0 = metadata array
        result["metadata"] = parseMetadata(root_arr->ptr[0]);

        // Index 1 = apm data array
        result["apm"] = parseApmData(root_arr->ptr[1]);

        // Index 2 = events array
        result["events"] = parseEvents(root_arr->ptr[2]);

        qDebug() << "Loaded recording data:"
                 << result["apm"].toList().size() << "APM points,"
                 << result["events"].toList().size() << "events";

    } catch (const std::exception& e) {
        qWarning() << "Error loading recording data:" << e.what();
    }

    return result;
}

QVariantMap DataLoader::loadRecordingMetadata(const QString &filePath)
{
    QVariantMap metadata;

    try {
        std::ifstream ifs(filePath.toStdString(), std::ios::binary);
        if (!ifs.is_open()) {
            qWarning() << "Could not open recording file:" << filePath;
            return metadata;
        }

        std::vector<char> buffer(
            (std::istreambuf_iterator<char>(ifs)),
            std::istreambuf_iterator<char>()
            );
        ifs.close();

        if (buffer.empty()) {
            qWarning() << "Recording file is empty:" << filePath;
            return metadata;
        }

        msgpack::object_handle oh = msgpack::unpack(buffer.data(), buffer.size());
        msgpack::object obj = oh.get();

        // Root muss ARRAY sein
        if (obj.type != msgpack::type::ARRAY) {
            qWarning() << "Invalid root format — expected ARRAY, got:" << (int)obj.type;
            return metadata;
        }

        const auto* root_arr = &obj.via.array;

        if (root_arr->size < 1) {
            qWarning() << "Root array has no metadata";
            return metadata;
        }

        metadata = parseMetadata(root_arr->ptr[0]);

        qDebug() << "Loaded metadata for game:"
                 << metadata["game_name"]
                 << "character:" << metadata["character_name"]
                << "kda:" << metadata["kda"];

    } catch (const std::exception& e) {
        qWarning() << "Error loading recording metadata:" << e.what();
    }

    return metadata;
}


QVariantMap DataLoader::parseMetadata(const msgpack::object& obj)
{
    QVariantMap metadata;

    if (obj.type != msgpack::type::ARRAY) {
        qWarning() << "Metadata is not an ARRAY";
        return metadata;
    }

    const auto* meta_arr = &obj.via.array;

    // [recording_id, game_name, character_name, map_name, round_number, timestamp, recording_start, duration]
    if (meta_arr->size < 8) {
        qWarning() << "Metadata array too small:" << meta_arr->size;
        return metadata;
    }

    // Index 0: recording_id (Binary UUID)
    if (meta_arr->ptr[0].type == msgpack::type::BIN) {
        const auto& bin = meta_arr->ptr[0].via.bin;
        QByteArray uuid_bytes(bin.ptr, bin.size);
        // Convert to UUID string format
        if (uuid_bytes.size() == 16) {
            QString uuid = QString("%1%2%3%4-%5%6-%7%8-%9%10-%11%12%13%14%15%16")
            .arg((quint8)uuid_bytes[0], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[1], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[2], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[3], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[4], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[5], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[6], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[7], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[8], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[9], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[10], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[11], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[12], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[13], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[14], 2, 16, QChar('0'))
                .arg((quint8)uuid_bytes[15], 2, 16, QChar('0'));
            metadata["recording_id"] = uuid;
        }
    }

    // Index 1: game_name
    if (meta_arr->ptr[1].type == msgpack::type::STR) {
        std::string game;
        meta_arr->ptr[1].convert(game);
        metadata["game_name"] = QString::fromStdString(game);
    }

    // Index 2: character_name (Option<String>)
    metadata["character_name"] = parseOptionalString(meta_arr->ptr[2]);

    // Index 3: kda (Option<KDA>)
    if (meta_arr->ptr[3].type != msgpack::type::NIL) {
        const auto& kda_obj = meta_arr->ptr[3];

        if (kda_obj.type == msgpack::type::ARRAY && kda_obj.via.array.size == 3) {
            const auto* kda_arr = &kda_obj.via.array;

            try {
                uint64_t kills64 = 0, deaths64 = 0, assists64 = 0;

                kda_arr->ptr[0].convert(kills64);
                kda_arr->ptr[1].convert(deaths64);
                kda_arr->ptr[2].convert(assists64);

                QVariantMap kdaMap;
                kdaMap["kills"] = static_cast<int>(kills64);
                kdaMap["deaths"] = static_cast<int>(deaths64);
                kdaMap["assists"] = static_cast<int>(assists64);

                metadata["kda"] = kdaMap;
            }
            catch (const std::exception& e) {
                qWarning() << "Failed to parse KDA:" << e.what();
            }
        }
        else {
            qWarning() << "Invalid KDA format, type:" << (int)kda_obj.type
                       << "size:" << (kda_obj.type == msgpack::type::ARRAY ? kda_obj.via.array.size : 0);
        }
    }

    // Index 4: map_name (Option<String>)
    metadata["map_name"] = parseOptionalString(meta_arr->ptr[4]);

    // Index 5: round_number (Option<u32>)
    if (meta_arr->ptr[5].type != msgpack::type::NIL) {
        uint32_t round;
        meta_arr->ptr[5].convert(round);
        metadata["round_number"] = static_cast<int>(round);
    }

    // Index 6: timestamp (DateTime string)
    if (meta_arr->ptr[6].type == msgpack::type::STR) {
        std::string timestamp;
        meta_arr->ptr[6].convert(timestamp);
        metadata["timestamp"] = QString::fromStdString(timestamp);
    }

    // Index 7: recording_start (u64)
    if (meta_arr->ptr[7].type == msgpack::type::POSITIVE_INTEGER) {
        uint64_t start;
        meta_arr->ptr[7].convert(start);
        metadata["recording_start"] = static_cast<qint64>(start);
    }

    // Index 8: duration_seconds (Option<f64>)
    if (meta_arr->ptr[8].type != msgpack::type::NIL) {
        double duration;
        meta_arr->ptr[8].convert(duration);
        metadata["duration_seconds"] = duration;
    }

    qDebug() << "Parsed metadata: game=" << metadata["game_name"]
             << "character=" << metadata["character_name"]
             << "duration=" << metadata["duration_seconds"];

    return metadata;
}

QVariantList DataLoader::parseApmData(const msgpack::object& obj)
{
    QVariantList apmList;

    if (obj.type != msgpack::type::ARRAY) {
        qWarning() << "APM data is not an ARRAY";
        return apmList;
    }

    const auto* apm_arr = &obj.via.array;

    // [series, average_apm, peak_apm]
    if (apm_arr->size < 3) {
        qWarning() << "APM array too small:" << apm_arr->size;
        return apmList;
    }

    // Index 0: series array
    const msgpack::object& series_obj = apm_arr->ptr[0];

    if (series_obj.type != msgpack::type::ARRAY) {
        qWarning() << "APM series is not an array";
        return apmList;
    }

    const auto* series_arr = &series_obj.via.array;
    qDebug() << "APM series size:" << series_arr->size;

    for (uint32_t j = 0; j < series_arr->size; ++j) {
        const msgpack::object& tuple = series_arr->ptr[j];

        if (tuple.type == msgpack::type::ARRAY && tuple.via.array.size == 2) {
            double second;
            uint32_t apm;
            tuple.via.array.ptr[0].convert(second);
            tuple.via.array.ptr[1].convert(apm);

            QVariantMap point;
            point["timestamp"] = static_cast<qint64>(second * 1000); // seconds -> ms
            point["apm"] = static_cast<int>(apm);
            apmList.append(point);
        }
    }

    if (!apmList.isEmpty()) {
        QVariantMap first = apmList.first().toMap();
        QVariantMap last = apmList.last().toMap();
        qDebug() << "First APM point: timestamp=" << first["timestamp"] << "ms, apm=" << first["apm"];
        qDebug() << "Last APM point: timestamp=" << last["timestamp"] << "ms, apm=" << last["apm"];
    }

    // Index 1: average_apm (Option<f64>)
    if (apm_arr->ptr[1].type != msgpack::type::NIL) {
        double avg;
        apm_arr->ptr[1].convert(avg);
        qDebug() << "Average APM:" << avg;
    }

    // Index 2: peak_apm (Option<u32>)
    if (apm_arr->ptr[2].type != msgpack::type::NIL) {
        uint32_t peak;
        apm_arr->ptr[2].convert(peak);
        qDebug() << "Peak APM:" << peak;
    }

    return apmList;
}

QVariantList DataLoader::parseEvents(const msgpack::object& obj)
{
    QVariantList events;

    if (obj.type != msgpack::type::ARRAY) {
        qWarning() << "Events field is not an array";
        return events;
    }

    const auto* events_arr = &obj.via.array;
    qDebug() << "Found" << events_arr->size << "events";

    for (uint32_t j = 0; j < events_arr->size; ++j) {
        const msgpack::object& event_obj = events_arr->ptr[j];

        QVariantMap event;

        if (event_obj.type == msgpack::type::MAP) {
            event = parseEventAsMap(event_obj);
        }
        else if (event_obj.type == msgpack::type::ARRAY) {
            event = parseEventAsArray(event_obj);
        }
        else {
            qWarning() << "Event" << j << "is neither MAP nor ARRAY";
            continue;
        }

        if (event.contains("timestamp") && event.contains("event_type")) {
            qDebug() << "Event:"
                     << event["event_type"].toString()
                     << "| Name:" << event.value("name").toString()
                     << "| Timestamp:" << event["timestamp"];
            events.append(event);
        }
    }

    return events;
}

QVariantMap DataLoader::parseEventAsMap(const msgpack::object& event_obj)
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

QVariantMap DataLoader::parseEventAsArray(const msgpack::object& event_obj)
{
    QVariantMap event;
    const auto* event_arr = &event_obj.via.array;

    if (event_arr->size < 4) {
        qWarning() << "Event array too small:" << event_arr->size;
        return event;
    }

    // Index 0: event_id
    uint32_t event_id;
    event_arr->ptr[0].convert(event_id);
    event["event_id"] = static_cast<int>(event_id);

    // Index 1: event_type
    event["event_type"] = parseEventType(event_arr->ptr[1]);

    // Index 2: timestamp
    double timestampSeconds;
    event_arr->ptr[2].convert(timestampSeconds);
    qint64 timestampMs = static_cast<qint64>(timestampSeconds * 1000.0);
    event["timestamp"] = timestampMs;

    // Index 3: data
    const msgpack::object& data_obj = event_arr->ptr[3];
    if (data_obj.type == msgpack::type::MAP) {
        parseEventData(data_obj, event);
    } else if (data_obj.type == msgpack::type::ARRAY) {
        parseEventDataAsArray(data_obj, event);
    }

    return event;
}

void DataLoader::parseEventData(const msgpack::object& data_obj, QVariantMap& event)
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
    }
}

void DataLoader::parseEventDataAsArray(const msgpack::object& data_obj, QVariantMap& event)
{
    const auto* data_arr = &data_obj.via.array;

    if (data_arr->size < 5) return;

    // Index 0: name
    std::string name;
    data_arr->ptr[0].convert(name);
    event["name"] = QString::fromStdString(name);

    // Index 1: actor
    event["actor"] = parseOptionalString(data_arr->ptr[1]);

    // Index 2: target
    event["target"] = parseOptionalString(data_arr->ptr[2]);
}

QString DataLoader::parseEventType(const msgpack::object& eventTypeObj)
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

QString DataLoader::parseOptionalString(const msgpack::object& obj)
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
