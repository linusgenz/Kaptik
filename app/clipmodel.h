#ifndef CLIPMODEL_H
#define CLIPMODEL_H

#include <QAbstractListModel>
#include <QDir>
#include <QFileInfoList>
#include <QImage>

struct Clip {
    QString name;
    QString path;
    QString duration;
    qint64  durationMs;
    QString date;
    QImage thumbnail;
    QString dataFilePath;
};

class ClipModel : public QAbstractListModel {
    Q_OBJECT

public:
    enum ClipRoles {
        NameRole = Qt::UserRole + 1,
        PathRole,
        DateRole,
        DurationRole,
        DurationMsRole,
        ThumbnailRole,
        DataFilePathRole,
    };
    explicit ClipModel(QObject* parent = nullptr);

    Q_PROPERTY(int count READ rowCount NOTIFY countChanged)

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    QHash<int, QByteArray> roleNames() const override;

    QImage thumbnailAt(int index) const;
    Q_INVOKABLE int getDurationMs(int index) const;

    Q_INVOKABLE void loadFromPath(const QString &dirPath);

signals:
    void countChanged();

private:
    QList<Clip> m_clips;
};

#endif // CLIPMODEL_H
