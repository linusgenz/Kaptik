#include "videometadata.h"

#ifdef Q_OS_WIN
#include <windows.h>
#include <shobjidl.h>
#include <QPixmap>
#include <QDir>
#include <propkey.h>
#include <propsys.h>
#include <initguid.h>
#include <QFile>
#include <QStandardPaths>

// GUID für BHID_PropertyStore
// {0384E1A4-1523-439C-A4C8-AB911052F586}
static const GUID BHID_PropertyStore =
    { 0x0384e1a4, 0x1523, 0x439c, { 0xa4, 0xc8, 0xab, 0x91, 0x10, 0x52, 0xf5, 0x86 } };

void getWindowsVideoDuration(const QString &filePath, QString &duration, quint64 &durationMs)
{
    duration = "00:00";
    durationMs = 0;

    if (!QFile::exists(filePath)) {
        return;
    }

    HRESULT hr = CoInitialize(nullptr);
    if (FAILED(hr)) {
        return;
    }

    IShellItem* shellItem = nullptr;
    QString fixedPath = QDir::toNativeSeparators(filePath);
    std::wstring wPath = fixedPath.toStdWString();

    hr = SHCreateItemFromParsingName(wPath.c_str(), nullptr, IID_PPV_ARGS(&shellItem));
    if (FAILED(hr)) {
        CoUninitialize();
        return;
    }

    IPropertyStore* propStore = nullptr;
    hr = shellItem->BindToHandler(nullptr, BHID_PropertyStore, IID_PPV_ARGS(&propStore));
    if (FAILED(hr)) {
        shellItem->Release();
        CoUninitialize();
        return;
    }

    PROPVARIANT var;
    PropVariantInit(&var);

    hr = propStore->GetValue(PKEY_Media_Duration, &var);
    if (SUCCEEDED(hr) && var.vt == VT_UI8) {
        // Dauer in 100-Nanosekunden
        quint64 duration100ns = var.uhVal.QuadPart;
        durationMs = duration100ns / 10000; // 10^4 -> Millisekunden
        quint64 totalSeconds = durationMs / 1000;
        quint64 minutes = totalSeconds / 60;
        quint64 seconds = totalSeconds % 60;
        duration = QString("%1:%2")
                       .arg(minutes, 2, 10, QChar('0'))
                       .arg(seconds, 2, 10, QChar('0'));
    }

    PropVariantClear(&var);
    propStore->Release();
    shellItem->Release();
    CoUninitialize();
}

QImage getWindowsThumbnail(const QString &filePath, int size)
{
    QImage image;

    if (!QFile::exists(filePath))
        return QImage();

    HRESULT hrInit = CoInitialize(nullptr);

    IShellItemImageFactory* factory = nullptr;
    QString fixedPath = QDir::toNativeSeparators(filePath);

    std::wstring wPath = fixedPath.toStdWString();

    HRESULT hr = SHCreateItemFromParsingName(
        wPath.c_str(),
        nullptr,
        IID_PPV_ARGS(&factory)
        );

    if (SUCCEEDED(hr)) {
        SIZE sz = { size, size };
        HBITMAP hBitmap = nullptr;

        hr = factory->GetImage(sz, SIIGBF_BIGGERSIZEOK, &hBitmap);

        if (SUCCEEDED(hr) && hBitmap) {
            image = QImage::fromHBITMAP(hBitmap);
            DeleteObject(hBitmap);
        }

        factory->Release();
    }

    CoUninitialize();

    return image;
}

QString getRecordingIdFromVideo(const QString& videoPath)
{
    if (!QFile::exists(videoPath))
        return QString();

    if (FAILED(CoInitialize(nullptr)))
        return QString();

    IShellItem* shellItem = nullptr;
    std::wstring wPath = QDir::toNativeSeparators(videoPath).toStdWString();

    if (FAILED(SHCreateItemFromParsingName(wPath.c_str(), nullptr, IID_PPV_ARGS(&shellItem)))) {
        CoUninitialize();
        return QString();
    }

    IPropertyStore* propStore = nullptr;
    if (FAILED(shellItem->BindToHandler(nullptr, BHID_PropertyStore, IID_PPV_ARGS(&propStore)))) {
        shellItem->Release();
        CoUninitialize();
        return QString();
    }

    QString recordingId;
    PROPERTYKEY key;

    if (SUCCEEDED(PSGetPropertyKeyFromName(L"System.Comment", &key))) {
        PROPVARIANT var;
        PropVariantInit(&var);

        if (SUCCEEDED(propStore->GetValue(key, &var)) && var.vt == VT_LPWSTR && var.pwszVal) {
            QString comment = QString::fromWCharArray(var.pwszVal);
            const QString prefix = "recording_id=";
            if (comment.startsWith(prefix))
                recordingId = comment.mid(prefix.length());
        }

        PropVariantClear(&var);
    }

    propStore->Release();
    shellItem->Release();
    CoUninitialize();

    return recordingId;
}

QString getApmPathForRecording(const QString& recordingId)
{
    if (recordingId.isEmpty())
        return QString();

    QDir dir(QDir::homePath() + "/AppData/Local/Kaptik/recordings");

    dir.mkpath("Kaptik/recordings");

    return dir.filePath( recordingId + ".apm" );
}



#endif
