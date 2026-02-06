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

    SettingsManager *settingsSingleton = new SettingsManager();

    QQmlApplicationEngine engine;

    qmlRegisterSingletonType<SettingsManager>(
        "App", 1, 0, "Settings",
        [settingsSingleton](QQmlEngine *, QJSEngine *) -> QObject* {
            return settingsSingleton;
        }
        );

    ClipModel clipModel;

    QString videoPath = settingsSingleton->value(SettingsManager::Key_VideoPath).toString();
    if (!videoPath.isEmpty()) {
        clipModel.loadFromPath(videoPath);
    }

    engine.addImageProvider("thumbnails", new ThumbnailProvider(&clipModel));
    engine.rootContext()->setContextProperty("clipModel", &clipModel);

    engine.loadFromModule("Kaptik", "Main");

    if (engine.rootObjects().isEmpty())
        return -1;

    settingsSingleton->connectToRecorder();

    return app.exec();
}
