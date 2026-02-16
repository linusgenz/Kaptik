#include "clipmodel.h"
#include <QDateTime>
#include "win/videometadata.h"

ClipModel::ClipModel(QObject* parent)
    : QAbstractListModel(parent)
{
}

int ClipModel::rowCount(const QModelIndex &parent) const {
    if (parent.isValid())
        return 0;
    return m_clips.size();
}

QVariant ClipModel::data(const QModelIndex &index, int role) const {
    if (!index.isValid() || index.row() >= m_clips.size())
        return {};

    const auto &clip = m_clips[index.row()];
    switch (role) {
    case NameRole: return clip.name;
    case PathRole: return clip.path;
    case DateRole: return clip.date;
    case DurationRole: return clip.duration;
    case DurationMsRole: return clip.durationMs;
    case ThumbnailRole:
        return QString("image://thumbnails/%1").arg(index.row());
    case DataFilePathRole: return clip.dataFilePath;
    default: return {};
    }
}

QHash<int, QByteArray> ClipModel::roleNames() const {
    return {
        {NameRole, "name"},
        {PathRole, "path"},
        {DateRole, "date"},
        {DurationRole, "duration"},
        {DurationMsRole, "durationMs"},
        {ThumbnailRole, "thumbnail"},
        {DataFilePathRole, "dataFilePath"},
    };
}

QImage ClipModel::thumbnailAt(int index) const {
    if (index < 0 || index >= m_clips.size())
        return QImage();
    return m_clips[index].thumbnail;
}

Q_INVOKABLE int ClipModel::getDurationMs(int index) const {
    if (index < 0 || index >= m_clips.size())
        return 0;
    return m_clips[index].durationMs;
}


void ClipModel::loadFromPath(const QString &dirPath) {
    beginResetModel();
    m_clips.clear();

    QDir dir(dirPath);
    if (!dir.exists()) {
        endResetModel();
        emit countChanged();
        return;
    }

    QStringList filters = {"*.mp4", "*.mkv", "*.avi"};
    QFileInfoList files = dir.entryInfoList(filters, QDir::Files, QDir::Time);

    for (const auto &fileInfo : files) {
        Clip clip;
        clip.name = fileInfo.baseName();
        clip.path = fileInfo.absoluteFilePath();
        clip.date = fileInfo.lastModified().toString("dd.MM.yyyy hh:mm");
        quint64 durationMs = 0;
        QString durationStr;
        getWindowsVideoDuration(clip.path, durationStr, durationMs);
        clip.duration = durationStr;
        clip.durationMs = durationMs;
        clip.thumbnail = getWindowsThumbnail(clip.path, 320);

        QString recordingId = getRecordingIdFromVideo(clip.path);

        if (!recordingId.isEmpty()) {
            QDir dir(QDir::homePath() + "/AppData/Local/Kaptik/recordings");

            QString filePath = dir.filePath(recordingId + ".msgpack");

            if (QFileInfo::exists(filePath)) {
                clip.dataFilePath = filePath;
                qDebug() << clip.dataFilePath;
            }
        }

        m_clips.append(clip);
    }

    endResetModel();
    emit countChanged();
}
