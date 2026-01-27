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

// GUID für BHID_PropertyStore
// {0384E1A4-1523-439C-A4C8-AB911052F586}
static const GUID BHID_PropertyStore =
    { 0x0384e1a4, 0x1523, 0x439c, { 0xa4, 0xc8, 0xab, 0x91, 0x10, 0x52, 0xf5, 0x86 } };

QString getWindowsVideoDuration(const QString &filePath)
{
    QString durationStr = "00:00";

    if (!QFile::exists(filePath)) {
        return durationStr;
    }

    HRESULT hr = CoInitialize(nullptr);
    if (FAILED(hr)) {
        return durationStr;
    }

    IShellItem* shellItem = nullptr;
    QString fixedPath = QDir::toNativeSeparators(filePath);;

    std::wstring wPath = fixedPath.toStdWString();
    hr = SHCreateItemFromParsingName(wPath.c_str(), nullptr, IID_PPV_ARGS(&shellItem));
    if (FAILED(hr)) {
        CoUninitialize();
        return durationStr;
    }

    IPropertyStore* propStore = nullptr;
    hr = shellItem->BindToHandler(nullptr, BHID_PropertyStore, IID_PPV_ARGS(&propStore));
    if (FAILED(hr)) {
        shellItem->Release();
        CoUninitialize();
        return durationStr;
    }

    PROPVARIANT var;
    PropVariantInit(&var);

    hr = propStore->GetValue(PKEY_Media_Duration, &var);
    if (SUCCEEDED(hr) && var.vt == VT_UI8) {
        // var.uhVal.QuadPart = Dauer in 100-Nanosekunden
        quint64 duration100ns = var.uhVal.QuadPart;
        quint64 totalSeconds = duration100ns / 10000000; // 10^7
        quint64 minutes = totalSeconds / 60;
        quint64 seconds = totalSeconds % 60;
        durationStr = QString("%1:%2")
                          .arg(minutes, 2, 10, QChar('0'))
                          .arg(seconds, 2, 10, QChar('0'));
    }

    PropVariantClear(&var);
    propStore->Release();
    shellItem->Release();
    CoUninitialize();

    return durationStr;
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

#endif
