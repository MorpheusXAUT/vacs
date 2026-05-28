use crate::app::state::AppState;
use crate::audio::PlaybackDeviceType;
use crate::audio::manager::AudioManagerHandle;
use crate::config::{CLIENT_SETTINGS_FILE_NAME, Persistable, PersistedClientConfig};
use crate::error::Error;
use crate::playback::recorder::{CLIP_PROGRESS_EVENT, PlaybackRecorderHandle};
use crate::playback::{ClipMeta, PlaybackError};
use crate::radio::track_audio::TrackAudioRadioHandle;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use vacs_audio::sources::wav::WavSource;

// TODO: Do we need some sort of a status?

#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_get_enabled(app_state: State<'_, AppState>) -> Result<bool, Error> {
    Ok(app_state.lock().await.config.client.playback.enabled)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_set_enabled(
    app: AppHandle,
    app_state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), Error> {
    let (persisted_client_config, playback_config) = {
        let mut state = app_state.lock().await;

        if state.config.client.playback.enabled == enabled {
            return Ok(());
        }

        state.config.client.playback.enabled = enabled;
        let playback_config = state.config.client.playback.clone();
        (
            PersistedClientConfig::from(state.config.client.clone()),
            playback_config,
        )
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_client_config.persist(&config_dir, CLIENT_SETTINGS_FILE_NAME)?;

    if enabled {
        // Start the recorder live if a TrackAudioRadio is currently active. If not, the
        // recorder will be started the next time the radio integration comes up.
        let radio = app.state::<TrackAudioRadioHandle>().read().clone();
        if let Some(radio) = radio {
            playback_config.start(&app, radio).await;
        } else {
            log::info!("playback enabled in config but no TrackAudio radio is active");
        }
    } else {
        // Stop any currently running recorder. The slot stays in place; future
        // PlaybackConfig::start calls will be no-ops while playback is disabled.
        let handle = app.state::<PlaybackRecorderHandle>();
        let existing = handle.write().take();
        if let Some(recorder) = existing {
            recorder.shutdown();
            log::info!("playback disabled; stopped active recorder");
        }
    }

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_list(
    recorder: State<'_, PlaybackRecorderHandle>,
) -> Result<Vec<ClipMeta>, Error> {
    Ok(recorder
        .read()
        .as_ref()
        .map(|r| r.list())
        .unwrap_or_default())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_delete(
    recorder: State<'_, PlaybackRecorderHandle>,
    id: u64,
) -> Result<bool, Error> {
    let Some(deleted) = recorder.read().as_ref().map(|r| r.delete(id)).transpose()? else {
        return Ok(false);
    };
    Ok(deleted)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_clear(recorder: State<'_, PlaybackRecorderHandle>) -> Result<(), Error> {
    if let Some(r) = recorder.read().as_ref() {
        r.clear()?;
    }
    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_play(
    app: AppHandle,
    recorder: State<'_, PlaybackRecorderHandle>,
    audio_manager: State<'_, AudioManagerHandle>,
    id: u64,
    device_type: PlaybackDeviceType,
) -> Result<(), Error> {
    stop_playing_source(&recorder, &audio_manager);

    let path: Option<PathBuf> = recorder
        .read()
        .as_ref()
        .and_then(|r| r.get(id).map(|m| m.path));
    let Some(path) = path else {
        return Err(PlaybackError::Other(Box::new(anyhow::anyhow!("clip {id} not found"))).into());
    };

    let audio_manager = audio_manager.read();

    let source_id = audio_manager.add_audio_source(
        move |sample_rate, channels| {
            Box::new(
                WavSource::from_file(
                    path,
                    sample_rate,
                    channels as usize,
                    1.0,
                    None,
                    Some(Box::new(move |progress| {
                        app.emit(CLIP_PROGRESS_EVENT, progress).ok();
                        if progress == 1.0 {
                            let recorder = app.state::<PlaybackRecorderHandle>();
                            if let Some(r) = recorder.write().as_mut() {
                                r.set_playing_source_id(None)
                            }
                        }
                    })),
                )
                .unwrap(),
            )
        },
        device_type,
    );
    audio_manager.start_audio_source(source_id, device_type);

    if let Some(r) = recorder.write().as_mut() {
        r.set_playing_source_id(Some((source_id, device_type)))
    }

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_stop(
    app: AppHandle,
    recorder: State<'_, PlaybackRecorderHandle>,
    audio_manager: State<'_, AudioManagerHandle>,
) -> Result<(), Error> {
    if !stop_playing_source(&recorder, &audio_manager) {
        log::warn!("playback stop called but no clip is playing");
    }

    app.emit(CLIP_PROGRESS_EVENT, 0.0).ok();

    Ok(())
}

fn stop_playing_source(
    recorder: &State<PlaybackRecorderHandle>,
    audio_manager: &State<AudioManagerHandle>,
) -> bool {
    if let Some((source_id, is_speaker)) = recorder
        .write()
        .as_mut()
        .and_then(|r| r.take_playing_source_id())
    {
        audio_manager
            .read()
            .remove_audio_source(source_id, is_speaker);
        return true;
    }
    false
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_seek(
    recorder: State<'_, PlaybackRecorderHandle>,
    audio_manager: State<'_, AudioManagerHandle>,
    millis: i64,
) -> Result<(), Error> {
    if millis == 0 {
        return Ok(());
    }

    if let Some((source_id, is_speaker)) = recorder
        .write()
        .as_mut()
        .and_then(|r| r.get_playing_source_id())
    {
        if millis < 0 {
            audio_manager.read().rewind_in_audio_source(
                source_id,
                Duration::from_millis(millis.unsigned_abs()),
                is_speaker,
            );
        } else {
            audio_manager.read().skip_in_audio_source(
                source_id,
                Duration::from_millis(millis.unsigned_abs()),
                is_speaker,
            );
        }
    } else {
        log::warn!("playback skip called but no clip is playing");
    }

    Ok(())
}

/// Copy a clip to the saved directory within the app data dir. Saved clips are exempt
/// from rolling-deque eviction. Returns the destination path.
#[tauri::command]
#[vacs_macros::log_err]
pub async fn playback_export(
    app: AppHandle,
    recorder: State<'_, PlaybackRecorderHandle>,
    id: u64,
) -> Result<PathBuf, Error> {
    let Some(path) = recorder
        .read()
        .as_ref()
        .map(|r| r.export(id, None))
        .transpose()?
    else {
        return Err(Error::Other(Box::new(anyhow::anyhow!(
            "recorder not running"
        ))));
    };

    if let Err(err) = app.opener().open_path(path.to_string_lossy(), None::<&str>) {
        return Err(PlaybackError::Other(Box::new(anyhow::anyhow!(
            "Cannot open export directory: {err}"
        )))
        .into());
    }

    Ok(path)
}
