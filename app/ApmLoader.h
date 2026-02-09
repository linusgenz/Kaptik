// ApmLoader.h
#ifndef APMLOADER_H
#define APMLOADER_H

#include <QObject>
#include <QVariantList>
#include <QString>
#include <QFile>
#include <QDebug>

// Einfacher Loader für APM-Daten aus MessagePack
class ApmLoader : public QObject
{
    Q_OBJECT

public:
    explicit ApmLoader(QObject *parent = nullptr);

    Q_INVOKABLE QVariantList loadApmData(const QString& filePath);

    Q_INVOKABLE int calculateAverageApm(const QVariantList& apmData);

private:
    QVariantList parseMsgPack(const QByteArray& data);
};

#endif // APMLOADER_H
