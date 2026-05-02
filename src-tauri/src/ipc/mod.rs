pub mod commands;

pub use commands::{
    AudioState, ProjectState,
    // Phase 1
    play_fm_test_tone, play_psg_test_tone, stop_all_sound,
    // Project management
    create_project, open_project, save_project, close_project, get_project_info,
    // Driver info
    list_drivers, get_driver_info,
    // FM instruments
    add_fm_instrument, update_fm_instrument, delete_fm_instrument,
    list_fm_instruments, preview_fm_instrument,
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
    add_region, update_region, delete_region,
    // Note CRUD
    add_note, update_note, delete_note,
    // Transport
    transport_play, transport_stop, transport_seek,
    transport_set_loop, transport_clear_loop, get_playback_state,
    // Validation
    get_channel_overlaps,
};
