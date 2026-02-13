#ifndef VIDEOMETADATA_H
#define VIDEOMETADATA_H

#include <QImage>

#ifdef Q_OS_WIN

QImage getWindowsThumbnail(const QString &filePath, int size = 256);
void getWindowsVideoDuration(const QString &filePath, QString &duration, quint64 &durationMs);
QString getRecordingIdFromVideo(const QString& videoPath);
QString getApmPathForRecording(const QString& recordingId);
QString getEventsPathForRecording(const QString& recordingId);
#endif
#endif
