//
// Created by Linus Genz on 24.01.2026.
// Copyright (c) 2026 Linus Genz. All rights reserved.
//

#include <QQmlApplicationEngine>
#include <qqmlcontext.h>
#include <QtQuickControls2/QQuickStyle>
#include <QQuickWindow>

#include "SettingsManager.h"
#include "clipmodel.h"
#include "thumbnailprovider.h"

int main(int argc, char *argv[]) {
    qputenv("QSG_RENDER_LOOP", "basic");
    qputenv("QT_MEDIA_BACKEND", "ffmpeg");

    QGuiApplication app(argc, argv);

    QQuickStyle::setStyle("Fusion");

    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);

    QQmlApplicationEngine engine;

    qmlRegisterSingletonType<SettingsManager>("App", 1, 0, "Settings", [](QQmlEngine *, QJSEngine *) -> QObject* {
        return new SettingsManager();
    });

    ClipModel clipModel;
    SettingsManager settings;
    QString videoPath = settings.loadVideoPath();
    if (!videoPath.isEmpty()) {
        clipModel.loadFromPath(videoPath);
    }

    engine.addImageProvider("thumbnails", new ThumbnailProvider(&clipModel));
    engine.rootContext()->setContextProperty("clipModel", &clipModel);

    engine.loadFromModule("Kaptik", "Main");

    if (engine.rootObjects().isEmpty())
        return -1;

    return app.exec();
}
