mod audio;
mod dac;
mod driver;
mod export;
mod ipc;
mod model;
mod project;
mod sequencer;
mod sn76489;
mod ym2612;

use std::sync::Mutex;

use audio::AudioThread;
use driver::FlamedriverProfile;
use ipc::{
    AudioState, ProjectState,
    // Phase 1
    play_fm_test_tone, play_psg_test_tone, stop_all_sound,
    // Project management
    create_project, open_project, save_project, close_project, get_project_info,
    // Driver info
    list_drivers, get_driver_info,
    // FM instruments
    add_fm_instrument, update_fm_instrument, delete_fm_instrument,
    list_fm_instruments, preview_fm_instrument, stop_fm_preview,
    // PSG instruments
    add_psg_instrument, update_psg_instrument, delete_psg_instrument,
    list_psg_instruments, preview_psg_instrument,
    // DAC instruments
    import_dac_wav, import_dac_raw, update_dac_instrument, reconvert_dac,
    delete_dac_instrument, list_dac_instruments, preview_dac,
    get_dac_pcm_data,
    // Track CRUD
    add_track, update_track, delete_track, list_tracks,
    // Region CRUD
    add_region, update_region, move_region, delete_region,
    // Note CRUD
    add_note, update_note, delete_note,
    // Transport
    transport_play, transport_stop, transport_seek,
    transport_set_loop, transport_clear_loop, get_playback_state,
    // Validation
    get_channel_overlaps,
    // Export
    export_song,
};
use model::driver::DriverRegistry;
use project::ProjectManager;
use tauri::Emitter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_thread = AudioThread::new().expect("failed to initialize audio thread");
    let position_tick = audio_thread.position_tick().clone();

    let mut registry = DriverRegistry::new();
    registry.register(Box::new(FlamedriverProfile));
    let project_manager = ProjectManager::new(registry);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState {
            thread: Mutex::new(audio_thread),
        })
        .manage(ProjectState {
            manager: Mutex::new(project_manager),
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(33));
                    let tick = position_tick.load(std::sync::atomic::Ordering::Relaxed);
                    let _ = handle.emit("playback-position", tick);
                }
            });
            Ok(())
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
            add_fm_instrument,
            update_fm_instrument,
            delete_fm_instrument,
            list_fm_instruments,
            preview_fm_instrument,
            stop_fm_preview,
            add_psg_instrument,
            update_psg_instrument,
            delete_psg_instrument,
            list_psg_instruments,
            preview_psg_instrument,
            import_dac_wav,
            import_dac_raw,
            update_dac_instrument,
            reconvert_dac,
            delete_dac_instrument,
            list_dac_instruments,
            preview_dac,
            get_dac_pcm_data,
            // Track CRUD
            add_track,
            update_track,
            delete_track,
            list_tracks,
            // Region CRUD
            add_region,
            update_region,
            move_region,
            delete_region,
            // Note CRUD
            add_note,
            update_note,
            delete_note,
            // Transport
            transport_play,
            transport_stop,
            transport_seek,
            transport_set_loop,
            transport_clear_loop,
            get_playback_state,
            // Validation
            get_channel_overlaps,
            // Export
            export_song,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
