// ApmLoader.cpp
#include "ApmLoader.h"
#include <msgpack.hpp>
#include <fstream>
#include <vector>

ApmLoader::ApmLoader(QObject *parent)
    : QObject(parent)
{
}

QVariantList ApmLoader::loadApmData(const QString& filePath)
{
    QVariantList result;

    try {
        std::ifstream ifs(filePath.toStdString(), std::ios::binary);
        if (!ifs.is_open()) {
            qWarning() << "Could not open APM file:" << filePath;
            return result;
        }

        std::vector<char> buffer((std::istreambuf_iterator<char>(ifs)),
                                 std::istreambuf_iterator<char>());

        if (buffer.empty()) {
            qWarning() << "APM file is empty:" << filePath;
            return result;
        }

        msgpack::object_handle oh = msgpack::unpack(buffer.data(), buffer.size());
        msgpack::object obj = oh.get();

        qDebug() << "MessagePack type:" << (int)obj.type;

        if (obj.type == msgpack::type::ARRAY) {
            msgpack::object_array* outer_arr = &obj.via.array;
            qDebug() << "Outer array size:" << outer_arr->size;

            if (outer_arr->size > 0) {
                // Das erste (und einzige) Element ist das series Array
                msgpack::object& series_obj = outer_arr->ptr[0];

                if (series_obj.type == msgpack::type::ARRAY) {
                    msgpack::object_array* series_arr = &series_obj.via.array;
                    qDebug() << "Series array size:" << series_arr->size;

                    // Jedes Element ist ein Tupel (second, apm)
                    for (uint32_t i = 0; i < series_arr->size; ++i) {
                        msgpack::object& tuple = series_arr->ptr[i];

                        if (tuple.type == msgpack::type::ARRAY && tuple.via.array.size == 2) {
                            double second;
                            uint32_t apm;

                            tuple.via.array.ptr[0].convert(second);
                            tuple.via.array.ptr[1].convert(apm);

                            QVariantMap point;
                            point["timestamp"] = static_cast<qint64>(second * 1000); // Sekunden -> Millisekunden
                            point["apm"] = static_cast<int>(apm);

                            result.append(point);
                        } else {
                            qWarning() << "Invalid tuple at index" << i << "- type:" << (int)tuple.type;
                        }
                    }
                } else {
                    qWarning() << "Expected series to be array, got type:" << (int)series_obj.type;
                }
            }
        } else {
            qWarning() << "Invalid APM data format - expected array (tuple struct), got type:" << (int)obj.type;
        }

        qDebug() << "Loaded" << result.size() << "APM data points from" << filePath;

        // Debug: Zeige erste und letzte Datenpunkte
        if (result.size() > 0) {
            QVariantMap first = result.first().toMap();
            QVariantMap last = result.last().toMap();
            qDebug() << "First point: timestamp=" << first["timestamp"] << "ms, apm=" << first["apm"];
            qDebug() << "Last point: timestamp=" << last["timestamp"] << "ms, apm=" << last["apm"];
        }

    } catch (const std::exception& e) {
        qWarning() << "Error loading APM data:" << e.what();
    }

    return result;
}

int ApmLoader::calculateAverageApm(const QVariantList& apmData)
{
    if (apmData.isEmpty()) {
        return 0;
    }

    qint64 totalApm = 0;
    for (const QVariant& var : apmData) {
        QVariantMap map = var.toMap();
        totalApm += map["apm"].toInt();
    }

    return static_cast<int>(totalApm / apmData.size());
}
