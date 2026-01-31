//
// Created by Linus Genz on 25.01.2026.
// Copyright (c) 2026 Linus Genz. All rights reserved.
//

#ifndef KAPTIK_THUMBNAILPROVIDER_H
#define KAPTIK_THUMBNAILPROVIDER_H
#include <QQuickImageProvider>
#include "clipmodel.h"

class ThumbnailProvider : public QQuickImageProvider {
public:
    ThumbnailProvider(ClipModel* model)
        : QQuickImageProvider(QQuickImageProvider::Image),
        m_model(model) {}

    QImage requestImage(const QString &id, QSize *size, const QSize &requestedSize) override {
        bool ok = false;
        int index = id.toInt(&ok);

        if (!ok || index < 0 || index >= m_model->rowCount()) {
            return QImage();
        }

        const QImage img = m_model->thumbnailAt(index);

        if (size)
            *size = img.size();

        if (requestedSize.isValid())
            return img.scaled(requestedSize, Qt::KeepAspectRatio, Qt::SmoothTransformation);

        return img;
    }

private:
    ClipModel* m_model;
};

#endif
