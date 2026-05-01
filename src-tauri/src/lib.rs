mod audio;
mod model;
mod ym2612;
mod sn76489;
mod ipc;

use std::sync::Mutex;
use ipc::{AudioState, play_fm_test_tone, play_psg_test_tone, stop_all_sound};
use audio::AudioThread;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_thread = AudioThread::new()
        .expect("failed to initialize audio thread");

    tauri::Builder::default()
        .manage(AudioState {
            thread: Mutex::new(audio_thread),
        })
        .invoke_handler(tauri::generate_handler![
            play_fm_test_tone,
            play_psg_test_tone,
            stop_all_sound,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
