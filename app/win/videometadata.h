#ifndef VIDEOMETADATA_H
#define VIDEOMETADATA_H

#include <QImage>

#ifdef Q_OS_WIN

QImage getWindowsThumbnail(const QString &filePath, int size = 256);
QString getWindowsVideoDuration(const QString &filePath);

#endif
#endif
