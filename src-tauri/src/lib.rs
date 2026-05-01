mod audio;
mod dac;
mod driver;
mod ipc;
mod model;
mod project;
mod sn76489;
mod ym2612;

use std::sync::Mutex;

use audio::AudioThread;
use driver::FlamedriverProfile;
use ipc::{
    AudioState, ProjectState,
    close_project, create_project, get_driver_info, get_project_info,
    list_drivers, open_project, play_fm_test_tone, play_psg_test_tone,
    save_project, stop_all_sound,
};
use model::driver::DriverRegistry;
use project::ProjectManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_thread = AudioThread::new().expect("failed to initialize audio thread");

    let mut registry = DriverRegistry::new();
    registry.register(Box::new(FlamedriverProfile));
    let project_manager = ProjectManager::new(registry);

    tauri::Builder::default()
        .manage(AudioState {
            thread: Mutex::new(audio_thread),
        })
        .manage(ProjectState {
            manager: Mutex::new(project_manager),
        })
        .invoke_handler(tauri::generate_handler![
            play_fm_test_tone,
            play_psg_test_tone,
            stop_all_sound,
            create_project,
            open_project,
            save_project,
            close_project,
            get_project_info,
            list_drivers,
            get_driver_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
