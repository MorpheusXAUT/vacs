use crate::error::Error;
use crate::radio::{
    DynRadio, Frequency, RadioHandle, RadioState, RadioStation, StationStateUpdate,
};
use tauri::State;

fn radio(radio: &RadioHandle) -> Result<DynRadio, Error> {
    let guard = radio.read();
    Ok(guard
        .as_ref()
        .ok_or_else(|| crate::radio::RadioError::Integration("No radio configured".into()))?
        .clone())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn radio_add_station(
    radio_handle: State<'_, RadioHandle>,
    callsign: String,
) -> Result<RadioStation, Error> {
    Ok(radio(&radio_handle)?.add_station(&callsign).await?)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn radio_set_station_state(
    radio_handle: State<'_, RadioHandle>,
    frequency: Frequency,
    update: StationStateUpdate,
) -> Result<RadioStation, Error> {
    Ok(radio(&radio_handle)?
        .set_station_state(frequency, update)
        .await?)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn radio_get_stations(
    radio_handle: State<'_, RadioHandle>,
) -> Result<Vec<RadioStation>, Error> {
    Ok(radio(&radio_handle)?.get_stations().await?)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn radio_fast_couple(radio_handle: State<'_, RadioHandle>) -> Result<(), Error> {
    Ok(radio(&radio_handle)?.fast_couple().await?)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn radio_reconnect(radio_handle: State<'_, RadioHandle>) -> Result<(), Error> {
    Ok(radio(&radio_handle)?.reconnect().await?)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn radio_get_state(radio_handle: State<'_, RadioHandle>) -> Result<RadioState, Error> {
    Ok(radio_handle
        .read()
        .as_ref()
        .map_or(RadioState::NotConfigured, |radio| radio.state()))
}
