use crate::settings::Settings;
use windows::Win32::Media::Audio::eConsole;
use windows::{
    Win32::Media::Audio::MMDeviceEnumerator,
    Win32::Media::Audio::{IMMDevice, IMMDeviceEnumerator, eCapture, eRender},
    Win32::System::Com::{
        CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    },
    core::Result,
};

fn get_default_device_id(role_render: bool) -> Result<String> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

        let data_flow = if role_render { eRender } else { eCapture };

        let device: IMMDevice = enumerator.GetDefaultAudioEndpoint(data_flow, eConsole)?;

        let id = device.GetId()?;

        CoUninitialize();
        Ok(id.to_string()?)
    }
}

pub fn get_game_audio_device(settings: &Settings) -> Result<String> {
    if settings.selected_game_audio_device.is_empty() {
        get_default_device_id(true).expect("Error while trying to get default output device");
    }

    Ok(settings.selected_game_audio_device.clone())
}

pub fn get_microphone_device(settings: &Settings) -> Result<String> {
    if settings.selected_microphone_device.is_empty() {
        get_default_device_id(true).expect("Error while trying to get default mic device");
    }

    Ok(settings.selected_microphone_device.clone())
}
