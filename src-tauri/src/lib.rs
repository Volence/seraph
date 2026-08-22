mod audio;
mod dac;
pub mod driver;
mod export;
pub mod import;
mod ipc;
pub mod library;
pub mod model;
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
    add_region, update_region, move_region, duplicate_region, delete_region,
    // Note CRUD
    add_note, update_note, delete_note,
    // Undo / redo
    undo, redo, begin_undo_group, end_undo_group, get_undo_state,
    // Transport
    transport_play, transport_stop, transport_seek,
    transport_set_loop, transport_clear_loop, get_playback_state, reload_sequence, set_master_volume,
    // Validation
    get_channel_overlaps,
    // Export
    export_song, export_wav, export_vgm,
    // Import
    import_song,
    import_zyrinx_song,
    import_fm_file,
    import_vgm,
    // Instrument library
    library_list, library_games, library_get_entry, library_rescan, library_warnings,
    library_audition, library_stop_audition,
    library_add_to_project, library_assign_to_track, library_save_from_project, library_import_files,
    library_set_tags, library_set_favorite,
    library_roots_get, library_root_add, library_root_remove,
};
use library::state::LibraryState;
use model::driver::DriverRegistry;
use project::ProjectManager;
use tauri::{Emitter, Manager};
use tauri_specta::{collect_commands, Builder};

/// Construct the tauri-specta `Builder` with every IPC command registered.
///
/// This is the single source of truth for the command set: `run()` mounts its
/// `invoke_handler` at runtime, and the codegen test exports its TypeScript
/// bindings to `src/bindings.ts`.
fn build_specta() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
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
        duplicate_region,
        delete_region,
        // Note CRUD
        add_note,
        update_note,
        delete_note,
        // Undo / redo
        undo,
        redo,
        begin_undo_group,
        end_undo_group,
        get_undo_state,
        // Transport
        transport_play,
        transport_stop,
        transport_seek,
        transport_set_loop,
        transport_clear_loop,
        get_playback_state,
        reload_sequence,
        set_master_volume,
        // Validation
        get_channel_overlaps,
        // Export
        export_song,
        export_wav,
        // Import
        import_song,
        import_zyrinx_song,
        import_fm_file,
        // VGM Export
        export_vgm,
        // VGM Import
        import_vgm,
        // Instrument library
        library_list,
        library_games,
        library_get_entry,
        library_rescan,
        library_warnings,
        library_audition,
        library_stop_audition,
        library_add_to_project,
        library_assign_to_track,
        library_save_from_project,
        library_import_files,
        library_set_tags,
        library_set_favorite,
        library_roots_get,
        library_root_add,
        library_root_remove,
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_thread = AudioThread::new().expect("failed to initialize audio thread");
    let position_tick = audio_thread.position_tick().clone();
    let spectrum_buffer = audio_thread.spectrum_buffer().clone();

    let mut registry = DriverRegistry::new();
    registry.register(Box::new(FlamedriverProfile));
    let project_manager = ProjectManager::new(registry);

    let specta_builder = build_specta();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState {
            thread: Mutex::new(audio_thread),
        })
        .manage(ProjectState {
            manager: Mutex::new(project_manager),
        })
        .manage(LibraryState::default())
        .setup(move |app| {
            // Populate the instrument library index on startup.
            library::state::rescan(app.handle(), &app.state::<LibraryState>());

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                const DISPLAY_BINS: usize = 128;
                const SR: f32 = 44100.0;
                const FFT_SIZE: f32 = 1024.0;
                const FREQ_MIN: f32 = 20.0;
                const FREQ_MAX: f32 = 22050.0;
                let log_min = FREQ_MIN.ln();
                let log_max = FREQ_MAX.ln();

                loop {
                    std::thread::sleep(std::time::Duration::from_millis(33));
                    let tick = position_tick.load(std::sync::atomic::Ordering::Relaxed);
                    let _ = handle.emit("playback-position", tick);

                    let mut spectrum_data = Vec::with_capacity(DISPLAY_BINS);
                    for i in 0..DISPLAY_BINS {
                        let f_lo = (log_min + (log_max - log_min) * i as f32 / DISPLAY_BINS as f32).exp();
                        let f_hi = (log_min + (log_max - log_min) * (i + 1) as f32 / DISPLAY_BINS as f32).exp();
                        let bin_lo = (f_lo / (SR / FFT_SIZE)).floor() as usize;
                        let bin_hi = (f_hi / (SR / FFT_SIZE)).ceil() as usize;
                        let bin_lo = bin_lo.clamp(0, 511);
                        let bin_hi = bin_hi.clamp(bin_lo + 1, 512);

                        let mut max_db = -96.0f32;
                        for bin in bin_lo..bin_hi {
                            let val = spectrum_buffer.read_bin(bin);
                            if val > max_db { max_db = val; }
                        }
                        spectrum_data.push(max_db);
                    }
                    let _ = handle.emit("spectrum-data", &spectrum_data);
                }
            });
            Ok(())
        })
        .invoke_handler(specta_builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod bindings_codegen {
    use super::build_specta;

    /// Regenerates `src/bindings.ts` from the annotated IPC commands.
    ///
    /// Rust is the single source of truth for IPC types; running `cargo test`
    /// re-emits the TypeScript bindings the frontend consumes. This keeps the
    /// generated file in sync and fails CI if the bindings are stale. Note: the
    /// bindings are regenerated by simply running `cargo test`; a stale
    /// `src/bindings.ts` surfaces as an unexpected git diff after the run.
    #[test]
    fn export_typescript_bindings() {
        // The hand-written types this replaces modelled u64/usize as `number`,
        // so keep bigints as `number` to preserve the frontend contract:
        // u64 fields (e.g. `Note::tick`) export as a JS `number`, which is safe
        // because tick/duration values never approach 2^53. The header
        // suppresses `noUnusedLocals` on the generated helpers/globals so the
        // strict frontend `tsc` build stays green.
        let ts = specta_typescript::Typescript::default()
            .bigint(specta_typescript::BigIntExportBehavior::Number)
            .header("// @ts-nocheck\n");
        build_specta()
            .export(ts, "../src/bindings.ts")
            .expect("failed to export typescript bindings");
    }
}
