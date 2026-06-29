# Keybinds & Radio Feature Analysis

This document captures the current architecture of the keybind and radio systems in
`vacs-client`, their interconnections, and notes on what makes them hard to separate.
It is the precursor to a refactoring design.

---

## 1. The Keybind System

### 1.1 Core Types

**`keybinds.rs`** defines:

- `KeyEvent { code: Code, label: String, state: KeyState }` — a raw key event from the OS listener.
- `Keybind` enum — the six logical actions that can be bound to a physical key:
  - `PushToTalk`, `PushToMute`, `RadioIntegration` — transmit-mode markers
  - `AcceptCall`, `EndCall`, `ToggleRadioPrio` — call-control actions
- `KeybindsError` — errors from the listener/emitter layer.

**`keybinds/runtime.rs`** defines:
- `KeybindListener` trait — captures global OS keyboard events asynchronously.
  Returns an `UnboundedReceiver<KeyEvent>` at startup.
- `KeybindEmitter` trait — injects (synthesizes) key presses into other applications.
  Synchronous `emit(code, state)`.
- `DynKeybindListener = Arc<dyn KeybindListener>`
- `DynKeybindEmitter = Arc<dyn KeybindEmitter>`
- Platform selection via `cfg_select!`:
  - Windows → `WindowsKeybindListener` + `WindowsKeybindEmitter`
  - macOS → `MacOsKeybindListener` + `MacOsKeybindEmitter`
  - Linux → `LinuxKeybindListener` + `LinuxKeybindEmitter` (Wayland via XDG portal;
    emitter is a no-op stub on Wayland)
  - Other → `NoopKeybindListener` + `NoopKeybindEmitter`

### 1.2 Config (`config.rs`)

Three separate config structs control keybind behavior:

```
TransmitConfig {
    mode: TransmitMode,          // VoiceActivation | PushToTalk | PushToMute | RadioIntegration
    push_to_talk: Option<Code>,
    push_to_mute: Option<Code>,
    radio_push_to_talk: Option<Code>,
}

KeybindsConfig {
    accept_call: Option<Code>,
    end_call: Option<Code>,
    toggle_radio_prio: Option<Code>,
}

RadioConfig {
    integration: RadioIntegration,          // AudioForVatsim | TrackAudio
    audio_for_vatsim: Option<AudioForVatsimRadioConfig { emit: Option<Code> }>,
    track_audio: Option<TrackAudioRadioConfig { endpoint: Option<String> }>,
}
```

`RadioConfig::radio()` is an async factory method that constructs the concrete radio
implementation (`PushToTalkRadio` or `TrackAudioRadio`) based on the configured integration.
This factory lives on `RadioConfig`, inside `config.rs`, making the config module directly
aware of both radio backends.

### 1.3 The Keybind Engine (`keybinds/engine.rs`)

`KeybindEngine` is the central orchestrator. It owns and manages everything related to
key-driven behavior, including the radio.

**Fields:**
```rust
KeybindEngine {
    mode: TransmitMode,
    transmit_code: Option<Code>,
    accept_call_code: Option<Code>,
    end_call_code: Option<Code>,
    toggle_radio_prio_code: Option<Code>,
    radio_config: RadioConfig,        // radio config stored on the engine
    app: AppHandle,
    listener: RwLock<Option<DynKeybindListener>>,
    radio: RadioHandle,               // radio instance owned by the engine
    rx_task: Option<JoinHandle<()>>,
    shutdown_token: CancellationToken,
    stop_token: Option<CancellationToken>,
    pressed: Arc<AtomicBool>,
    call_active: Arc<AtomicBool>,
    radio_prio: Arc<AtomicBool>,
    implicit_radio_prio: Arc<AtomicBool>,
}
```

**Lifecycle:**

- `start()` — starts the OS listener; if mode is `RadioIntegration`, also constructs the
  radio from config and stores it in `self.radio` AND in the global Tauri state
  (`app.state::<RadioHandle>()`). Then spawns the rx loop.
- `stop()` — drops the listener; clears `self.radio` AND `app.state::<RadioHandle>()`;
  emits `radio:integration-available = false`.
- `set_config()` — calls `stop()`, updates transmit/keybinds codes, calls `start()`.
- `set_radio_config()` — calls `stop()`, updates `self.radio_config`, calls `start()`.
- `reconnect_radio()` — delegates to the radio's `reconnect()` method.

**The Rx Loop (inside `spawn_rx_loop`):**

On every key event from the OS listener:

1. If `KeyState::Down`: run `handle_call_control_event()` — checks accept/end-call
   and toggle-radio-prio codes, then dispatches into `AppState` (signaling/webrtc).
2. If code matches the transmit key, update `pressed` state and dispatch based on
   mode × call_active × radio_prio:

   | mode             | call_active | radio_prio | action on key down/up                  |
   |------------------|-------------|------------|----------------------------------------|
   | RadioIntegration | false       | -          | `radio.transmit(Active/Inactive)`      |
   | RadioIntegration | true        | false      | `set_input_muted(muted/unmuted)`       |
   | RadioIntegration | true        | true       | both: `set_input_muted(true)` + transmit |
   | PushToTalk       | true        | false      | `set_input_muted(muted/unmuted)`       |
   | PushToTalk       | true        | true       | `set_input_muted(true)` (blocked)      |
   | PushToMute       | true        | false      | `set_input_muted(muted/unmuted)`       |

3. On `KeyState::Up`, if `implicit_radio_prio` is set, clean it up — either clear prio
   or force `radio.transmit(Inactive)`.

**Implicit Radio Prio:**

When a call becomes active while the RadioIntegration PTT key is already held, the engine
automatically sets `radio_prio = true` and `implicit_radio_prio = true`. On key release,
this is cleaned up. This logic lives entirely inside the engine and `set_call_active()`.

**Platform-specific Wayland Code Mapping:**

On Linux/Wayland, the transmit codes and call-control codes are replaced with virtual
function keys (F31–F35) because Wayland shortcuts are configured at the OS level via the
XDG portal. The portal triggers these virtual keys, and the engine processes them normally.
`select_active_transmit_code()`, `select_accept_call_code()`, etc. perform this mapping.

**Key methods exposed by the engine (used by state/commands):**

- `set_call_active(bool)` — called when a call starts/ends
- `set_radio_prio(bool)` — called by keybind or audio command
- `radio_prio() -> bool`
- `radio_state() -> RadioState`
- `radio() -> Option<DynRadio>`
- `get_external_binding(Keybind) -> Option<String>` (Wayland only)
- `should_attach_input_muted() -> bool` — used when attaching a new WebRTC call

### 1.4 Tauri Commands (`keybinds/commands.rs`)

All front-end interactions with the keybind system go through these Tauri commands:

- `keybinds_get_transmit_config` / `keybinds_set_transmit_config`
- `keybinds_get_keybinds_config` / `keybinds_set_binding`
- `keybinds_get_radio_config` / `keybinds_set_radio_config` ← radio config via keybind command
- `keybinds_get_radio_state` ← radio state via keybind command
- `keybinds_get_external_binding`
- `keybinds_open_system_shortcuts_settings`
- `keybinds_reconnect_radio` ← radio reconnect via keybind command

Note that radio management commands are named and namespaced under `keybinds_`, not `radio_`.

All commands that change config check `capabilities.keybind_listener` and return
`CapabilityNotAvailable` if unavailable. Even radio config changes fail if the listener
is not available.

---

## 2. The Radio System

### 2.1 Core Types (`radio.rs`)

```rust
trait Radio: Send + Sync + Debug + Any {
    async fn transmit(&self, state: TransmissionState) -> Result<(), RadioError>;
    async fn reconnect(&self) -> Result<(), RadioError>;
    fn state(&self) -> RadioState;
    async fn add_station(&self, callsign: &str) -> Result<RadioStation, RadioError>;
    async fn set_station_state(&self, freq, update) -> Result<RadioStation, RadioError>;
    async fn get_stations(&self) -> Result<Vec<RadioStation>, RadioError>;
    async fn fast_couple(&self) -> Result<(), RadioError>;
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}

type DynRadio = Arc<dyn Radio>;
type RadioHandle = Arc<RwLock<Option<DynRadio>>>;
```

`TransmissionState` converts to/from `KeyState` — the radio system is already aware of
keyboard types at the type level.

`RadioIntegration::default()` checks `Capabilities::default().keybind_emitter` to choose
between AudioForVatsim and TrackAudio — the radio default depends on a keybind capability.

### 2.2 `RadioState` Enum

```
NotConfigured | Disconnected | Connected | VoiceConnected | RxIdle | RxActive(HashSet<Frequency>) | TxActive | Error
```

`RadioState` has an `emit()` helper that fires `radio:state` over Tauri events.

### 2.3 `RadioStation` and `StationStateUpdate`

`RadioStation` is the vacs-canonical station representation. All backends convert their
types into this. `StationStateUpdate` is a partial update struct for user-controllable
fields (rx, tx, xca, headset, output_muted). The `xc` field is read-only (backend-computed).

### 2.4 `PushToTalkRadio` (`radio/push_to_talk.rs`)

Implements `Radio` by wrapping a `DynKeybindEmitter`. When `transmit(Active)` is called, it
calls `emitter.emit(code, KeyState::Down)`. On `transmit(Inactive)`, it emits `KeyState::Up`.
Emits `radio:state` events directly.

**Key coupling point**: `PushToTalkRadio` directly imports and uses `DynKeybindEmitter` and
`PlatformEmitter` from `keybinds/runtime.rs`. It is a radio backend that IS a keybind
emitter — these are architecturally the same thing here. The radio module directly imports
the keybinds runtime module.

The `Drop` impl releases the PTT key if it was held, preventing stuck keys.

### 2.5 `TrackAudioRadio` (`radio/track_audio.rs`)

A full WebSocket-based radio backend. Connects to a TrackAudio instance, subscribes to its
event stream, and maps TrackAudio events to `RadioState` + Tauri events.

Notable: In its `ConnectionState::Connected` handler, it reads `app.state::<RadioHandle>()`
to access the current radio instance and start the playback recorder:

```rust
let radio = app.state::<RadioHandle>().read().clone();
if let Some(radio) = radio {
    state.config.client.playback.start(app, radio).await;
}
```

This creates a dependency on the global `RadioHandle` Tauri state being set — which is only
done by the `KeybindEngine`. If the radio were constructed outside the engine, this would
still work only if `RadioHandle` in Tauri state is kept in sync.

### 2.6 Tauri Commands (`radio/commands.rs`)

```rust
radio_add_station(keybind_engine, callsign)
radio_set_station_state(keybind_engine, frequency, update)
radio_get_stations(keybind_engine)
radio_fast_couple(keybind_engine)
```

Every radio command takes `keybind_engine: State<'_, KeybindEngineHandle>` as its first
state argument and gets the radio via `engine.read().await.radio()`. The radio itself
is inaccessible directly from Tauri state without going through the engine.

---

## 3. Interconnections — The Coupling Map

### 3.1 Lifecycle Coupling (Tight)

The radio's lifetime is entirely controlled by the `KeybindEngine`:

```
KeybindEngine::start()
  └─ if TransmitMode::RadioIntegration:
       └─ RadioConfig::radio(app).await  →  construct DynRadio
            └─ store in self.radio
            └─ store in app.state::<RadioHandle>()

KeybindEngine::stop()
  └─ self.radio.write().take()           →  drop DynRadio
  └─ app.state::<RadioHandle>().write().take()
  └─ app.emit("radio:integration-available", false)
```

The radio only exists when:
1. The engine is running, AND
2. `TransmitMode` is `RadioIntegration`.

If the user switches to `PushToTalk`, the radio is destroyed.
If the engine is reconfigured, the radio is destroyed and recreated.

### 3.2 State Coupling (Redundant Dual Storage)

The active radio instance is stored in two places simultaneously:
- `KeybindEngine.radio: RadioHandle` — the engine's local reference
- `app.state::<RadioHandle>()` — the global Tauri managed state

Both are updated atomically in `start()` and `stop()`. This dual storage exists because:
- The engine's rx loop clones `self.radio.read()` at spawn time for use during the loop.
- The global state is needed by `TrackAudioRadio`'s event handler (to access the
  radio for playback) and potentially by other future consumers.

### 3.3 Config Coupling (RadioConfig Lives on the Engine)

`RadioConfig` is a field of `KeybindEngine`. Changing the radio config requires calling
`engine.set_radio_config()`, which internally stops and restarts the engine.

The command `keybinds_set_radio_config` is the only way to change radio config from the
frontend. It is gated by `capabilities.keybind_listener`, meaning you cannot change radio
configuration on platforms without a keybind listener even though `TrackAudio` integration
does not require a keybind listener at all.

### 3.4 Command Routing Coupling

Radio station management commands (`radio_add_station`, etc.) route through the engine:

```
radio_add_station
  → keybind_engine.read().await.radio()
      → self.radio.read().clone()
          → radio.add_station(callsign).await
```

There is no way to call radio station management without a reference to the engine.
The `DynRadio` is not directly accessible from Tauri state (the `RadioHandle` in global
state is only set by the engine, and the radio commands don't read from global state).

### 3.5 Transmit Mode Coupling

`TransmitMode::RadioIntegration` is both a transmit mode AND an implicit signal to
activate radio integration. The two concerns are fused:
- "When should audio be muted?" depends on whether a call is active and whether radio prio is set.
- "Should a radio be started?" depends on the transmit mode.

Switching transmit mode has a side effect of creating or destroying the radio.

### 3.6 `PushToTalkRadio` ↔ Keybind Emitter Coupling

`PushToTalkRadio` imports `DynKeybindEmitter` and `PlatformEmitter` from
`keybinds/runtime.rs`. It is architecturally a radio that IS a keybind emitter. This is
the deepest coupling — the radio module directly depends on the keybind runtime module
at the type level.

This coupling exists because "AFV radio integration" works by simulating key presses into
Audio For VATSIM. The design choice to model this as a `Radio` backend rather than a
keybind passthrough means the radio module must depend on the keybind emitter.

### 3.7 Radio Default Depends on Keybind Capability

```rust
impl Default for RadioIntegration {
    fn default() -> Self {
        if Capabilities::default().keybind_emitter {
            RadioIntegration::AudioForVatsim
        } else {
            RadioIntegration::TrackAudio
        }
    }
}
```

The default radio integration is determined by whether the keybind emitter works on the
current platform. This is a logical dependency but signals that the two systems are
conceptually entangled.

### 3.8 `radio:integration-available` Event

The engine emits `radio:integration-available = false` when stopping (i.e., when switching
away from `RadioIntegration` mode or shutting down). This event only makes sense in the
context of the current coupling — a standalone radio system would manage its own
availability.

### 3.9 Call Control Logic Interleaved with Radio Logic

The engine's rx loop handles both call-control keybinds (accept/end call, toggle prio)
and radio transmission in the same event loop. The `call_active` and `radio_prio` atomic
booleans, together with `implicit_radio_prio`, implement interaction logic between calls
and radio transmission that crosses both domains.

---

## 4. Use Cases and Data Flows

### 4.1 User Presses PTT (RadioIntegration mode, no active call)

```
OS key event → LinuxWayland XDG portal / Windows hook / macOS accessibility
  → PlatformListener::rx channel → KeybindEngine::rx loop
      → code matches transmit_code (F35/RadioIntegration key)
      → mode == RadioIntegration, call_active == false
      → radio.transmit(Active) → TrackAudioRadio::transmit(Active)
          → TrackAudio WebSocket API: transmit(true)
      OR
      → radio.transmit(Active) → PushToTalkRadio::transmit(Active)
          → emitter.emit(afv_key, KeyState::Down)   ← keybind emitter
          → app.emit("radio:state", TxActive)
```

### 4.2 User Presses PTT (RadioIntegration mode, active call)

```
OS key event → KeybindEngine::rx loop
  → mode == RadioIntegration, call_active == true, radio_prio == false
  → AudioManager.set_input_muted(false)   ← talk into the call
  (radio is NOT transmitted)

  OR if radio_prio == true:
  → AudioManager.set_input_muted(true)   ← mute the call mic
  → radio.transmit(Active)               ← AND transmit on radio
```

### 4.3 Call Becomes Active While PTT is Held (Implicit Radio Prio)

```
AppState::set_call_active(true) → keybind_engine.set_call_active(true)
  → pressed == true, mode == RadioIntegration, radio_prio == false
  → radio_prio = true, implicit_radio_prio = true
  → app.emit("audio:implicit-radio-prio", true)

Later, key released:
  → implicit_radio_prio.swap(false) → radio_prio.swap(false) → cleanup
  → app.emit("audio:implicit-radio-prio", false)
```

### 4.4 User Toggles Radio Prio via Keybind

```
OS key event (toggle_radio_prio code, KeyState::Down)
  → handle_call_control_event()
      → call_active == true
      → prio = !engine.radio_prio()
      → engine.set_radio_prio(prio)
          → if prio OFF and key held: implicit_radio_prio = true (cleanup on release)
          → if mode == VoiceActivation or PushToMute not pressed: AudioManager.set_input_muted(prio)
      → app.emit("audio:radio-prio", prio)
```

### 4.5 Frontend Updates Radio Config

```
Frontend → keybinds_set_radio_config(FrontendRadioConfig)
  → validate_afv_radio_integration_config()   ← keybind/radio cross-validation
  → keybind_engine.set_radio_config(new_config)
      → engine.stop()   ← destroys current radio
      → engine.radio_config = new_config
      → engine.start()   ← creates new radio from new config
  → persist ClientConfig
```

### 4.6 Frontend Queries Radio Stations

```
Frontend → radio_get_stations(keybind_engine)
  → keybind_engine.read().radio()   ← goes through engine
      → self.radio.read().clone()
  → radio.get_stations().await
      → TrackAudioState::stations()   ← cached in-memory
```

---

## 5. What Makes Separation Non-Trivial

The following specific issues must be resolved in any separation effort:

### 5.1 Radio Lifecycle is Gated on TransmitMode

Currently, a radio only exists when `TransmitMode::RadioIntegration`. A separated radio
system would need its own lifecycle independent of transmit mode. The engine currently
infers "should a radio exist?" from the transmit mode. A standalone radio manager would
need an explicit enabled/disabled concept separate from the transmit mode.

### 5.2 Dual RadioHandle Storage with Implicit Synchronization

The `RadioHandle` in Tauri global state and the one inside the engine must be kept in sync.
Any separation must decide which is the authoritative source and how to keep the other
in sync (or eliminate one of them).

### 5.3 PushToTalkRadio is a Keybind Emitter

`PushToTalkRadio` imports `PlatformEmitter` from `keybinds/runtime.rs`. It is a radio
that wraps a keybind emitter. There are two options for separation:
- Move `PlatformEmitter` / `KeybindEmitter` to a neutral module that both radio and
  keybinds can import.
- Accept that `PushToTalkRadio` depends on the keybind system and keep it as a bridge.

### 5.4 `RadioIntegration` Default Depends on `keybind_emitter` Capability

This logical coupling can be broken by making the default a simple constant or by having
the radio system expose its own capability check that happens to consult the same platform
detection.

### 5.5 All Radio Station Commands Route Through the Engine

To decouple radio station commands from the engine, a `DynRadio` would need to be
accessible directly from Tauri state or from a dedicated radio manager. The radio commands
would then read from that manager rather than from the engine.

### 5.6 `RadioConfig` Validation Is Mixed with Keybind Validation

`validate_afv_radio_integration_config()` in `keybinds/commands.rs` validates that the
AFV emit key differs from the PTT key. This cross-concern validation makes sense (both
configs must be consistent) but lives in the keybinds command file. After separation, this
validation either moves to a shared location or is enforced at the engine level where both
configs are visible.

### 5.7 `set_call_active()` Touches Both Audio and Radio

The engine's `set_call_active()` method contains logic that involves both the audio mute
state and the radio implicit prio state. This is genuinely cross-concern logic and cannot
be split cleanly without an event/notification mechanism between the two systems.

### 5.8 keybinds_set_radio_config is Gated on keybind_listener Capability

Even `TrackAudio` configuration (which does not require a keybind listener) is blocked on
`capabilities.keybind_listener`. A separated system would gate each feature on its own
capability.

---

## 6. Summary of Coupling Points (Ordered by Severity)

| # | Coupling | Severity | Notes |
|---|----------|----------|-------|
| 1 | Radio lifecycle controlled by `KeybindEngine` | High | Radio created/destroyed on transmit mode change |
| 2 | All radio commands route through engine handle | High | No direct access to `DynRadio` from Tauri state |
| 3 | `PushToTalkRadio` imports `PlatformEmitter` from `keybinds/runtime` | High | Radio module directly depends on keybind runtime |
| 4 | Dual `RadioHandle` storage (engine + global state) | Medium | Implicit sync, redundant |
| 5 | `RadioConfig` stored on engine; changing it restarts engine | Medium | Radio config and engine lifecycle conflated |
| 6 | `keybinds_set_radio_config` gated on `keybind_listener` | Medium | TrackAudio blocked unnecessarily on platforms without listener |
| 7 | `RadioIntegration::default()` depends on `keybind_emitter` | Low | Logical coupling, easy to decouple |
| 8 | Radio config validation lives in keybinds commands | Low | Cross-concern but currently the only correct location |
| 9 | `radio:integration-available` event emitted by engine | Low | Naming/semantics coupling |
| 10 | `TransmissionState` converts to/from `KeyState` | Low | Type-level coupling in `radio.rs` |

---

## 7. Feature Analysis: Radio Integration Without a Shared PTT Key

### 7.1 The Fundamental Conflation

The current `TransmitMode` enum fuses two conceptually independent concerns into a single
setting:

1. **Call mic activation** -- how the microphone is gated for voice calls
   (VAD / press-to-unmute / press-to-mute).
2. **Radio activation** -- whether a radio integration is running and what PTT key controls it.

`RadioIntegration` is the only transmit mode that activates the radio at all. The radio
exists only while that mode is selected, and its PTT key is the same physical key that
doubles as the call mic PTT when a call is active (unless radio prio overrides it).

This means every mode other than `RadioIntegration` -- including `VoiceActivation` -- is
permanently and silently radio-off. There is no path to have a live radio connection while
using voice-activated calling.

### 7.2 What the Current Rx Loop Does with a Single Key

The engine rx loop dispatches on (mode, call_active, radio_prio):

`
RadioIntegration, call_active=false:
  key down/up -> radio.transmit(Active/Inactive)
  (call mic irrelevant -- not attached)

RadioIntegration, call_active=true, radio_prio=false:
  key down -> set_input_muted(false)   [call mic = speak]
  key up   -> set_input_muted(true)    [call mic = muted]
  (radio does NOT transmit)

RadioIntegration, call_active=true, radio_prio=true:
  key down -> set_input_muted(true) + radio.transmit(Active)
  key up   -> set_input_muted(true) + radio.transmit(Inactive)
  (both channels; call mic stays muted throughout)

PushToTalk/PushToMute, call_active=true:
  key controls call mic only; radio never touched
`

The key observation: when a call is active in RadioIntegration mode **without radio
prio**, the key becomes a pure call mic PTT and radio transmission is completely
suppressed. Only radio prio restores radio transmission -- at the cost of silencing
the call mic for the entire duration the key is held.

### 7.3 The set_radio_prio Side Effect in VA Mode

There is a quiet interaction in the existing code directly relevant to the new feature:

`
ust
// In set_radio_prio():
match (&self.mode, self.pressed.load(Ordering::Relaxed)) {
    (TransmitMode::VoiceActivation, _) | (TransmitMode::PushToMute, false) => {
        self.app.state::<AudioManagerHandle>().read().set_input_muted(prio);
    }
    _ => {}
}
`

In VoiceActivation mode, calling set_radio_prio(true) **immediately mutes the call mic**.
Calling set_radio_prio(false) unmutes it. The engine already knows how to gate the call
mic based on radio prio in VA mode -- it just has no source of radio PTT events to drive
those transitions. If a radio PTT key existed and the engine called set_radio_prio(true)
on key down and set_radio_prio(false) on key up, the call mic gating would already work
correctly for the VA + radio prio case.

### 7.4 should_attach_input_muted and Initial Call Mic State

When a call is established (on_peer_connected), the input device is attached with an
initial mute state determined by the engine:

`
ust
pub fn should_attach_input_muted(&self) -> bool {
    match (&self.mode, self.pressed.load(Ordering::Relaxed)) {
        (TransmitMode::PushToTalk, false) => true,         // PTT not held -> attach muted
        (TransmitMode::PushToMute, true) => true,          // PTM held -> attach muted
        (TransmitMode::RadioIntegration, false) => true,   // radio PTT not held -> attach muted
        (TransmitMode::RadioIntegration, true) => self.radio_prio.load(Ordering::Relaxed),
        _ => false,                                         // VA -> attach unmuted
    }
}
`

In VoiceActivation mode this always returns false -- the call mic always attaches unmuted,
and VAD inside the audio pipeline decides whether to transmit audio.

However, if a radio PTT key is independently held when the call starts (and radio prio is
active), the call mic should attach muted. This edge case is currently handled by
implicit_radio_prio in RadioIntegration mode only. The new model requires the same logic
to fire for any call mic mode when the radio PTT key is independently held.

### 7.5 Implicit Radio Prio -- What It Does and Why It Matters

implicit_radio_prio is set when set_call_active(true) fires while the PTT key is already
held in RadioIntegration mode:

`
ust
if active {
    if matches!(self.mode, TransmitMode::RadioIntegration)
        && self.pressed.load(Ordering::Relaxed)
        && !self.radio_prio.load(Ordering::Relaxed)
    {
        self.radio_prio.store(true, Ordering::Relaxed);
        self.implicit_radio_prio.store(true, Ordering::Relaxed);
        self.app.emit("audio:implicit-radio-prio", true).ok();
    }
}
`

Without this: a call arriving while the user holds the radio PTT key would attach the
call mic in whatever state should_attach_input_muted() computes, but the radio was
already transmitting. The user would inadvertently talk into both channels.

implicit_radio_prio overrides should_attach_input_muted() to return true at that moment,
so the call mic attaches muted. On key release, both implicit_radio_prio and radio_prio
are cleared, restoring normal behavior.

In the new model, this guard must fire for **all call mic modes** when the radio PTT
key (now independent from the call PTT key) is held when a call starts.

On key release after implicit prio, two sub-cases apply:
- If radio_prio was separately set (user toggled it on): clear implicit flag, emit
  audio:implicit-radio-prio = false, leave radio_prio in place.
- If radio_prio was only the implicit one: clear both, call radio.transmit(Inactive),
  emit audio:implicit-radio-prio = false.

---

### 7.6 The Full Configuration Matrix

The configuration space has three orthogonal axes:

- **Call mic mode**: VoiceActivation (VA), PushToTalk (PTT), PushToMute (PTM)
- **Radio PTT**: None (no radio PTT key), Same (same physical key as call PTT/PTM),
  Different (separate physical key). For VA there is no call PTT key; the column reads
  None or Yes.
- **Radio backend**: None, TrackAudio, AudioForVatsim (AFV)

Column meanings:

- **Muted/R-PTT**: Is the call mic muted while the radio PTT key is held?
  `(R-Prio)` = only when explicit radio prio is set; `(impl)` = automatic (implicit prio);
  no qualifier = unconditionally.
  The two base rows where Radio PTT = None (PTT and PTM) carry a conditional value that
  documents the mode's general muting rule — not the current row's behavior (since no radio
  PTT key exists there).
- **Radio Page**: Is the radio station management UI page accessible?
- **RX Rec**: Is received radio audio recorded?
- **TX Rec**: Is transmitted radio audio recorded? In RED rows where Radio = None, "Yes"
  means the recording feature would be armed (PTT key exists) but would capture nothing —
  part of why that combination is forbidden. "Yes (Tx: API)" means TX events are observable
  via TrackAudio's WebSocket even without a vacs PTT key.
- **TX vacs**: Does vacs send the TX command (TrackAudio API call or AFV key emit)?
  "No" = the radio application manages TX independently. "Egal" = irrelevant to this
  combination's validity.
- **Impl Prio**: Does the implicit radio prio mechanism apply?
  "(impl, impl)" = each key has independent implicit prio; no cross-key disambiguation needed.

Status:

- **BLACK** = must be available after the refactor
- **ORANGE** = implement only if it does not require extensive extra effort; otherwise RED
- **RED** = must not be a configurable combination; prevented in UI and validation

---

| Status | Call mic | R-PTT     | Radio      | Muted/R-PTT                    | Radio Page | RX Rec | TX Rec        | TX vacs    | Impl Prio        |
|--------|----------|-----------|------------|--------------------------------|------------|--------|---------------|------------|------------------|
| BLACK  | VA       | None      | None       | No                             | No         | No     | No            | No         | No               |
| BLACK  | PTT      | None      | None       | If R-PTT ≠ PTT → Yes           | No         | No     | No            | No         | No               |
| BLACK  | PTM      | None      | None       | If R-PTT = PTM → Yes           | No         | No     | No            | No         | No               |
| RED    | VA       | None      | TrackAudio | No                             | Yes        | Yes    | Yes           | No         | No               |
| ORANGE | VA       | Yes       | TrackAudio | No                             | Yes        | Yes    | Yes           | Yes        | No               |
| RED    | VA       | None      | AFV        | No                             | No         | Yes    | No            | No         | No               |
| ORANGE | VA       | Yes       | AFV        | No                             | No         | Yes    | Yes           | Yes (Egal) | No               |
| RED    | PTT      | Different | None       | Yes                            | No         | No     | Yes           | No         | Yes (impl, impl) |
| RED    | PTT      | Same      | None       | Yes (R-Prio)                   | No         | No     | Yes           | No         | Yes              |
| BLACK  | PTT      | Different | TrackAudio | Yes (impl)                     | Yes        | Yes    | Yes           | Yes (Egal) | Yes (impl, impl) |
| BLACK  | PTT      | Same      | TrackAudio | Yes (R-Prio)                   | Yes        | Yes    | Yes           | Yes        | Yes              |
| BLACK  | PTT      | Different | AFV        | Yes (impl)                     | No         | Yes    | Yes           | Yes (Egal) | Yes (impl, impl) |
| BLACK  | PTT      | Same      | AFV        | Yes (R-Prio)                   | No         | Yes    | Yes           | Yes        | Yes              |
| RED    | PTM      | Different | None       | No                             | No         | No     | Yes           | No         | No               |
| RED    | PTM      | Same      | None       | Yes                            | No         | No     | Yes           | No         | Yes              |
| RED    | PTM      | Different | TrackAudio | No                             | Yes        | Yes    | Yes           | Egal       |                  |
| BLACK  | PTM      | Same      | TrackAudio | Yes (impl)                     | Yes        | Yes    | Yes           | Yes (Egal) | Yes              |
| RED    | PTM      | Different | AFV        | No                             | No         | Yes    | Yes           | Egal       |                  |
| BLACK  | PTM      | Same      | AFV        | Yes (impl)                     | No         | Yes    | Yes           | Egal       | Yes              |
| RED    | VA       | Yes       | None       | No                             | No         | No     | Yes           | No         |                  |
| RED    | PTT      | None      | TrackAudio | No                             | Yes        | Yes    | Yes (Tx: API) | No         |                  |
| RED    | PTT      | None      | AFV        | No                             | No         | Yes    | No            | No         |                  |
| RED    | PTM      | None      | TrackAudio | If R-PTT = PTM → Yes           | Yes        | Yes    | Yes (Tx: API) | No         |                  |
| RED    | PTM      | None      | AFV        | No                             | No         | Yes    | No            | No         |                  |

---

### 7.6a Design Rules Behind the Matrix

The matrix is not arbitrary. Five rules determine each cell's status.

**Rule 1 — PTT mode muting rule** (from base row PTT | None | None):

> In PTT mode, the call mic is muted while the radio PTT key is held only when the radio
> PTT key is DIFFERENT from the call PTT key.

Consequences:
- PTT + Different → Muted: Yes (impl) → **BLACK**: call mic is silenced during radio TX.
- PTT + Same → Muted: Yes (R-Prio) → **BLACK**: single key, disambiguated by radio prio.

**Rule 2 — PTM mode muting rule** (from base row PTM | None | None):

> In PTM mode, the call mic is muted while the radio PTT key is held only when the radio
> PTT key is THE SAME as the PTM key.

Consequences:
- PTM + Same → Muted: Yes (impl) → **BLACK**: PTM key naturally mutes; radio TX coupled.
- PTM + Different → Muted: No → **RED**: call mic stays open during radio TX. The user
  would transmit on radio while the call mic remains unmuted — an unsafe state. This is
  the primary reason PTM + Different is forbidden regardless of radio backend.

**Rule 3 — Radio PTT key and radio backend must be configured together:**

- Radio backend without a vacs PTT key → **RED**: vacs cannot trigger TX (rows:
  VA | None | TA/AFV, PTT | None | TA/AFV, PTM | None | TA/AFV). TrackAudio can observe
  its own TX events (Tx: API) but vacs has no control — unsupported split model.
- Radio PTT key without a radio backend → **RED**: the key arms TX recording and the
  implicit prio mechanism but can never transmit. Confusing and non-functional (rows:
  PTT | Diff/Same | None, PTM | Diff/Same | None, VA | Yes | None).

**Rule 4 — VA + radio is ORANGE, not BLACK:**

VA + radio (VA | Yes | TA/AFV) is technically valid: radio and call mic are fully
independent axes. However it is an unusual ATC workflow — the controller is always-on by
VAD and uses a separate key only for radio. Implement if the refactor makes it cheap;
otherwise treat as RED.

**Rule 5 — Implicit prio does not apply in VA mode:**

VA always attaches unmuted. Even when the radio PTT key is held at call start, the call
mic attaches unmuted (VA baseline). The user controls call-mic silencing via the explicit
radio_prio toggle.

---

### 7.7 Detailed Behavior Analysis for Each Valid Scenario

#### Scenario VA-0: VoiceActivation, no radio

Standard VA mode. The call mic follows VAD; no radio, no PTT key. No changes required
from the current implementation.

---

#### Scenario VA-R: VoiceActivation + Radio PTT + radio backend (ORANGE)

This is the primary new feature: radio and call are fully independent axes.

**No call active:**
- Radio PTT down -> radio.transmit(Active).
- Radio PTT up -> radio.transmit(Inactive).

**Call active, radio_prio = false:**
- Call mic attaches unmuted; VAD controls transmission.
- Radio PTT down -> radio.transmit(Active). Call mic is unaffected -- VAD continues.
- Radio PTT up -> radio.transmit(Inactive). Call mic is unaffected.
- Both channels can be simultaneously open. This is intentional.

**Call active, radio_prio = true:**
- set_radio_prio(true) was called -> set_input_muted(true) via the existing VA path in
  set_radio_prio() (see section 7.3). Call mic is forced muted.
- Radio PTT down -> radio.transmit(Active). Call mic remains muted.
- Radio PTT up -> radio.transmit(Inactive). Call mic remains muted (prio is still on).
- Prio toggle off -> set_input_muted(false). VAD resumes.

**Call starts while radio PTT is held:**
- Implicit prio does NOT fire for VA-R. The call attaches unmuted (VA baseline). The radio
  was already transmitting; the user can toggle radio_prio explicitly if they want to mute
  the call mic during the ongoing radio TX.

**VA + None + radio (no radio PTT key, ORANGE):**
- Radio backend is active but vacs has no PTT event. TX is driven by the radio application
  or TrackAudio's own UI. Vacs provides RX recording and station management.
- Implicit prio does not apply (vacs has no radio PTT event to detect).

---

#### Scenario PTT-0: PushToTalk, no radio

Standard PTT mode. Call PTT key unmutes the call mic while held. No changes required.

---

#### Scenario PTT-Same: PushToTalk + Same key + radio backend

This is the existing RadioIntegration mode, refactored into the new model. The call PTT
key and the radio PTT key share the same physical code.

**No call active:**
- Key down -> radio.transmit(Active).
- Key up -> radio.transmit(Inactive).

**Call active, radio_prio = false:**
- Key down -> set_input_muted(false) [call PTT unmutes].
- Key up -> set_input_muted(true).
- Radio does NOT transmit. Call mic PTT takes precedence.

**Call active, radio_prio = true:**
- Key down -> set_input_muted(true) + radio.transmit(Active).
- Key up -> set_input_muted(true) + radio.transmit(Inactive).
- Call mic stays muted throughout. Radio prio resolves the ambiguity of the shared key.

**Call starts while key is held:**
- Implicit prio fires (radio_prio = true, implicit_radio_prio = true). Call attaches muted.
- On key release: both implicit and explicit radio_prio are cleared; call mic unmutes.

Note: PTT + Same + None (radio PTT key configured but no radio backend) is **RED** and
must not be configurable. The sub-case is therefore not described here.

---

#### Scenario PTT-Diff: PushToTalk + Different keys + radio backend

Two entirely separate keys: call PTT controls call mic; radio PTT controls radio TX.

**No call active:**
- Call PTT: no effect (call mic not attached).
- Radio PTT down -> radio.transmit(Active).
- Radio PTT up -> radio.transmit(Inactive).

**Call active, radio_prio = false:**
- Call PTT down -> set_input_muted(false). Radio unaffected.
- Call PTT up -> set_input_muted(true). Radio unaffected.
- Radio PTT down -> radio.transmit(Active). Call mic is also muted (implicit prio: pressing
  the radio PTT key always triggers muting when a call is active, no explicit toggle needed).
- Radio PTT up -> radio.transmit(Inactive). Call mic restored to reflect call PTT state.

**Call active, radio_prio = true (explicit):**
- Behavior is the same as the implicit case above; explicit prio merely ensures the muting
  persists even if the user had explicitly toggled it.

**Call starts while radio PTT is held:**
- Implicit prio fires: call attaches muted, implicit_radio_prio = true.
- On radio PTT release: clear implicit_radio_prio and radio_prio (if only implicit),
  restore call mic based on call PTT state.

**Call starts while call PTT is held:**
- Standard PTT attach-muted logic (PTT not unmuted yet). This is independent of the radio
  PTT implicit prio and requires no disambiguation.

Because the two implicit behaviors (call-start-while-call-PTT and call-start-while-radio-PTT)
are on separate keys, no cross-key disambiguation is required.

---

#### Scenario PTM-0: PushToMute, no radio

Standard PTM mode. No changes required.

---

#### Scenario PTM-Same: PushToMute + Same key + radio backend

The PTM key and radio PTT key share the same physical code. Pressing the key mutes the call
mic (PTM behavior) and simultaneously triggers radio TX.

**No call active:**
- Key down -> radio.transmit(Active).
- Key up -> radio.transmit(Inactive).

**Call active:**
- Key down -> set_input_muted(true) [PTM mutes] + radio.transmit(Active).
- Key up -> set_input_muted(false) [PTM unmutes] + radio.transmit(Inactive).
- The call mic and radio TX are always coupled on this key. Implicit prio fires when a call
  starts while the key is held.

Note: PTM + Same + None (radio PTT key configured but no radio backend) is **RED** and
must not be configurable.

---

Note: PTM + Different + radio is **RED** (call mic stays open during radio TX). No scenario
description is provided.

---

### 7.7a Behavior Summary Table

For quick reference, the call mic state when radio PTT is held, by scenario:

| Scenario            | Call mic when R-PTT held            | Notes                                      |
|---------------------|-------------------------------------|--------------------------------------------|
| VA-R (prio off)     | Unmuted (VAD)                       | Independent channels; both can be open     |
| VA-R (prio on)      | Muted                               | Explicit prio required; toggle to mute     |
| PTT-Same (prio off) | Unmuted (key acts as call PTT only) | Radio suppressed; prio resolves ambiguity  |
| PTT-Same (prio on)  | Muted                               | Radio TX + call muted                      |
| PTT-Diff            | Muted (implicit, always)            | Separate keys; radio PTT always mutes      |
| PTM-Same            | Muted (PTM behavior, unconditional) | PTM key also TXs radio; muting natural     |
| PTM-Diff            | (RED — not implemented)             | Would leave call open during radio TX      |

---

### 7.8 Radio Prio Semantics in the New Model

In the current model, radio_prio is scoped to RadioIntegration mode. Its meaning is:
"when the shared key is pressed during a call, use it for the radio and mute the call mic,
rather than just unmuting the call mic."

In the new model, radio_prio has scenario-specific semantics because the call mic mode
and the key layout change what the default behavior is:

| Scenario    | radio_prio = off (default)                    | radio_prio = on                              |
|-------------|-----------------------------------------------|----------------------------------------------|
| VA-R        | Call mic open (VAD); radio TX is independent  | Call mic muted when radio PTT held           |
| PTT-Same    | Key acts as call PTT only; radio suppressed   | Key acts as radio PTT; call mic stays muted  |
| PTT-Diff    | Radio PTT always mutes call mic (implicit)    | Explicit flag; same behavior as implicit     |
| PTM-Same    | PTM key mutes call and TXs radio together     | Same; muting is structural (PTM behavior)    |

Key observations:
- For **VA-R**: the channels are independent by default. Explicit radio_prio is needed to
  silence the call mic during radio TX.
- For **PTT-Same**: radio_prio is the disambiguation mechanism for the shared key.
  Without it, the key is used as call PTT only (radio suppressed). This is the existing
  RadioIntegration behavior.
- For **PTT-Diff**: the call mic is always muted when the radio PTT key is pressed during
  a call. No explicit radio_prio toggle is required -- the separate key implies intent.
  This fires via the implicit_radio_prio mechanism each time the radio PTT key is pressed.
- For **PTM-Same**: the PTM key mutes the call mic as its primary function. Radio TX is
  a consequence of pressing the same key. radio_prio as an explicit toggle is structurally
  irrelevant (muting always happens); it may still be tracked for UI state consistency.
- PTM-Diff is RED and not implemented.

The toggle_radio_prio keybind and audio_set_radio_prio command are meaningful for VA-R,
PTT-Same, and PTT-Diff. For PTM-Same they are superfluous but harmless.

---

### 7.9 The Existing RadioIntegration Mode Mapped to the New Model

The current RadioIntegration mode is a specific instance of the new model where the call
PTT key and the radio PTT key share the **same physical code**, and an explicit
disambiguation rule applies:

> If a call is active and the shared key is pressed without radio prio: act as call PTT only, suppress radio.

This rule exists because having radio and call mic both open simultaneously by default
would be confusing. In the new model, when users have two distinct keys, there is no
ambiguity: the radio PTT always controls the radio; the call PTT always controls the call
mic. Both can be active simultaneously.

The RadioIntegration preset can be preserved as a named combination (call mic = PTT,
radio = PTT, same key) with the legacy disambiguation rule applied for that specific case.

---

### 7.10 What Changes Are Required

**Config model:**

TransmitMode::RadioIntegration is removed as a discrete variant. The radio becomes
an orthogonal dimension in the config:

`
Before:
  TransmitMode = { VoiceActivation, PushToTalk, PushToMute, RadioIntegration }

After:
  CallMicMode = { VoiceActivation, PushToTalk, PushToMute }
  radio_push_to_talk: Option<Code>  -- None means no radio
`

TransmitConfig becomes:
`
ust
struct TransmitConfig {
    call_mic_mode: CallMicMode,       // renamed from TransmitMode
    push_to_talk: Option<Code>,       // unchanged
    push_to_mute: Option<Code>,       // unchanged
    radio_push_to_talk: Option<Code>, // already exists; no longer gated on mode
}
`

The radio_push_to_talk field already exists. What changes is that it activates the radio
regardless of call_mic_mode.

**Engine state:**

Two pressed atomics instead of one:
`
ust
call_pressed: Arc<AtomicBool>,   // renamed from pressed; tracks call PTT/PTM key
radio_pressed: Arc<AtomicBool>,  // new; tracks radio PTT key
`

**Radio lifecycle:**

Radio is created when radio_push_to_talk is set AND a valid RadioConfig exists,
regardless of call_mic_mode. It persists across call mic mode changes. This is the
primary structural change enabling the new scenarios.

**Rx loop:**

Handles up to two distinct codes: call_code (optional, for PTT/PTM modes) and
radio_code (optional, for radio PTT). Each fires its own dispatch path independently.

For a radio_code event while call_active:
- PTT-Diff (separate keys):
    key down -> radio.transmit(Active) + set implicit_radio_prio + set_input_muted(true)
    key up   -> radio.transmit(Inactive) + clear implicit_radio_prio + restore PTT state
- VA-R (voice activation):
    key down -> radio.transmit(Active) [call mic unchanged; explicit radio_prio handles it]
    key up   -> radio.transmit(Inactive) [call mic unchanged]
- PTT-Same / PTM-Same: single key code; the same event drives both call mic and radio.
- PTM-Diff: RED, not implemented.

**set_call_active:**

For PTT-Diff only: implicit prio fires when radio_pressed is true at call start.
For VA-R: implicit prio does NOT fire; call attaches unmuted (VA baseline).

**should_attach_input_muted:**

New logic (pseudocode):
```rust
fn should_attach_input_muted(&self) -> bool {
    let call_pressed  = self.call_pressed.load(Relaxed);
    let radio_pressed = self.radio_pressed.load(Relaxed);
    let radio_prio    = self.radio_prio.load(Relaxed);    // includes implicit_radio_prio
    let separate_keys = self.radio_code != self.call_code;

    match self.call_mic_mode {
        VoiceActivation => false,   // VA always attaches unmuted; implicit prio does not fire
        PushToTalk =>
            // muted if: call PTT not held (standard), OR radio PTT active with effective prio
            !call_pressed
            || (radio_pressed && (radio_prio || separate_keys)),
        PushToMute =>
            // PTM-Same: call_pressed = radio_pressed (same key); muted when held
            // PTM-Diff is RED and never reached
            call_pressed,
    }
}
```

Cases verified:
- `PTT-Same, key held, implicit_radio_prio set by set_call_active`:
  !false=false; radio_pressed=true, radio_prio=true, separate=false
  → false || (true && (true || false)) = **muted** ✓
- `PTT-Same, key not held`: !true=true → **muted** ✓ (PTT not held = muted)
- `PTT-Diff, radio held, call not held`: !true=true → **muted** ✓
- `PTT-Diff, both held`: !false=false; radio=true, separate=true → **muted** ✓
- `PTM-Same, key held`: call_pressed=true → **muted** ✓
- `PTM-Same, key not held`: call_pressed=false → **unmuted** ✓
- `VA, any`: **unmuted** ✓

**reset_input_state:**

Call mic reset is based on call_mic_mode only:
- VoiceActivation -> unmuted
- PushToTalk -> muted
- PushToMute -> unmuted

Radio transmission resets to Inactive separately (radio lifecycle concern).

---

### 7.11 What Does NOT Change

- The Radio trait interface and both backend implementations (PushToTalkRadio,
  TrackAudioRadio) are unchanged.
- The radio_prio / toggle_radio_prio keybind and audio_set_radio_prio command retain
  their existing roles and semantics.
- The implicit radio prio concept is preserved; it is extended to cover all call mic modes.
- Platform listeners and emitters are unchanged.
- Wayland virtual key mapping (F31-F35) extends naturally: radio PTT maps to F35 (same
  as today), call PTT to F33/F34, call controls to F31/F32.
- PushToTalkRadio (AFV key emitter) remains Windows/macOS only; that platform limitation
  is independent of the call mic mode change.

---

## 8. Per-Configuration Key Event Reference

This section enumerates exactly what happens in every key event for every valid
configuration. Scenarios:

- **Outside a call** — no WebRTC peer connected.
- **Entering a call** — `on_peer_connected` fires; key(s) already held at that moment.
- **In a call (prio off)** — call mic attached, radio_prio = false.
- **In a call (prio on)** — call mic attached, radio_prio = true (explicit toggle).
- **Leaving a call** — `cleanup_call` fires; key(s) still held at that moment.
- **Radio Prio button** — `audio_set_radio_prio` / `toggle_radio_prio` keybind.

Abbreviations: **MIC** = call microphone mute state (Muted / Unmuted / not attached).
**R-TX** = radio transmission state (Active / Inactive / n/a). ↓ = key down, ↑ = key up.

---

### 8.1 VA | None | None — Voice Activation, no radio

**Keys:** none bound. `call_code = None`, `radio_code = None`. No key events are
processed; `call_pressed` and `radio_pressed` are never set.

#### Outside a call

No keys → nothing. Radio prio button calls `set_input_muted(prio)` via the
`(VoiceActivation, _)` arm in `set_radio_prio`, but MIC is not attached so there is no
audible effect. The `radio_prio` flag is stored.

#### Entering a call

`should_attach_input_muted` returns `false` (VA always unmuted on attach), regardless of
pre-set prio state. If prio was toggled on before the call, the MIC still attaches
**unmuted** — the prio muting is not re-applied at attach time. Implicit prio does not
fire (no radio key).

#### In a call

| State     | MIC            | Notes                              |
|-----------|----------------|------------------------------------|
| prio off  | **Unmuted** (VAD) | VAD controls audio send        |
| prio on   | **Muted**      | Hard-muted; VAD suspended          |

No key events occur (no keys bound). MIC state changes only via the prio button.

#### Leaving a call

MIC detached. `set_call_active(false)` unconditionally clears both `radio_prio` and
`implicit_radio_prio` and emits `audio:implicit-radio-prio, false`. Prio does **not**
persist across calls — the next call always starts with `radio_prio = false`.

#### Radio Prio button

`set_radio_prio` always uses the `(VoiceActivation, _)` arm regardless of `call_pressed`,
calling `set_input_muted(prio)` in all cases.

| Action     | In call: immediate MIC effect   | Outside call: MIC effect |
|------------|---------------------------------|--------------------------|
| Toggle ON  | **Muted immediately**           | None (not attached)      |
| Toggle OFF | **Unmuted immediately** (VAD)   | None (not attached)      |

---

### 8.2 VA | Yes | Radio (ORANGE) — Voice Activation + radio PTT key

**Keys:** R-PTT only. `call_code = None`, `radio_code = Some(r_ptt_code)`. `is_call_key
= false` always; `is_radio_key = true` for R-PTT events; `separate = true` always.
Behavior is identical for TrackAudio and AFV.

#### Outside a call

| Event   | MIC          | R-TX         |
|---------|--------------|--------------|
| R-PTT ↓ | not attached | **Active**   |
| R-PTT ↑ | not attached | **Inactive** |

#### Entering a call

`should_attach_input_muted` returns `false` (VA always unmuted on attach). If prio was
pre-set before the call, the MIC still attaches **unmuted** — prio is not re-applied at
attach time.

| R-PTT held? | MIC attaches | impl_prio fires? | R-TX |
|-------------|--------------|------------------|------|
| No          | **Unmuted**  | No               | Inactive |
| Yes         | **Unmuted**  | No               | Active (was already TX-ing) |

Implicit prio never fires for VA mode: `set_call_active` guards on
`call_mic_mode != VoiceActivation`. The `radio_prio` flag is unchanged by call entry.
Radio TX was already running if R-PTT was held; it continues uninterrupted.

#### In a call — prio off

R-PTT has no MIC effect — the rx loop VA arm always returns `None`. MIC follows VAD
throughout.

| Event   | MIC                         | R-TX         |
|---------|-----------------------------|--------------|
| R-PTT ↓ | **Unmuted** (VAD, unchanged)| **Active**   |
| R-PTT ↑ | **Unmuted** (VAD, unchanged)| **Inactive** |

#### In a call — prio on

`set_radio_prio(true)` mutes MIC immediately via `(VoiceActivation, _)` →
`set_input_muted(true)`. R-PTT still has no MIC effect; MIC stays muted.

| Event   | MIC                       | R-TX         |
|---------|---------------------------|--------------|
| R-PTT ↓ | Muted (unchanged)         | **Active**   |
| R-PTT ↑ | Muted (prio still on)     | **Inactive** |

#### Leaving a call

`set_call_active(false)` unconditionally clears `radio_prio` and `implicit_radio_prio`,
emitting `audio:implicit-radio-prio, false`. Prio does **not** persist across calls.

- **R-PTT not held:** MIC detached.
- **R-PTT held:** MIC detached. Radio TX **continues** (R-PTT still held, no call). R-PTT
  released later → R-TX Inactive.

#### Radio Prio button

`set_radio_prio` uses `(VoiceActivation, _)` → `set_input_muted(prio)` in all cases,
regardless of `call_pressed` or call state.

**Outside a call:** `set_input_muted(prio)` is called but MIC is not attached — no
audible effect. `radio_prio` flag is stored.

**In a call:**

| Action      | Immediate MIC effect            | On R-PTT ↓    | On R-PTT ↑        |
|-------------|---------------------------------|---------------|-------------------|
| Toggle ON   | **Muted immediately**           | R-TX Active   | Muted (prio on)   |
| Toggle OFF  | **Unmuted immediately** (VAD)   | R-TX Active   | Unmuted (VAD)     |

**Toggle OFF with R-PTT held:** `set_radio_prio(false)` also sets `implicit_radio_prio =
true` (because `radio_pressed = true`). MIC unmutes immediately (VA path). On next R-PTT
release: cleanup fires, clears `implicit_radio_prio`, triggers radio TX stop (TODO).

---

### 8.3 PTT | None | None — PushToTalk, no radio

**Keys:** PTT (call push-to-talk). `call_code = Some(ptt)`, `radio_code = None`.
`radio_pressed` is never set; `implicit_radio_prio` never fires. `effective_prio =
radio_prio_loaded` always (no implicit prio to mask).

#### Outside a call

| Event   | MIC          | R-TX |
|---------|--------------|------|
| PTT ↓   | not attached | n/a  |
| PTT ↑   | not attached | n/a  |

Key events have no effect outside a call.

#### Entering a call

`should_attach_input_muted = !call_pressed`. Prio state does not affect the attach value.

- **PTT not held:** MIC **attaches muted**.
- **PTT held:** MIC **attaches unmuted**.

#### In a call — prio off

| Event   | MIC         | R-TX |
|---------|-------------|------|
| PTT ↓   | **Unmuted** | n/a  |
| PTT ↑   | **Muted**   | n/a  |

#### In a call — prio on (mute-lock)

| Event   | MIC                      | R-TX |
|---------|--------------------------|------|
| PTT ↓   | **Muted** (lock active)  | n/a  |
| PTT ↑   | **Muted** (lock active)  | n/a  |

`effective_prio = true` → `(PushToTalk, true, _, true) => Some(true)` for all PTT events.

#### Leaving a call

`set_call_active(false)` unconditionally clears `radio_prio` and `implicit_radio_prio`.
Prio does **not** persist across calls.

- **PTT held:** MIC detached. PTT ↑ later: no effect (no call active).

#### Radio Prio button

`set_radio_prio` matches `_ => {}` for PTT mode — **no immediate MIC effect** in either
direction, regardless of whether the key is held. The `radio_prio` flag is updated;
the new state takes effect on the next PTT key event.

| Action     | Immediate MIC effect | Next PTT ↓   | Next PTT ↑ |
|------------|----------------------|--------------|------------|
| Toggle ON  | None                 | Muted (lock) | Muted      |
| Toggle OFF | None                 | Unmuted      | Muted      |

---

### 8.4 PTT | Different | Radio — PushToTalk + separate radio PTT key

**Keys:** PTT (call) and R-PTT (radio), two distinct physical keys (`radio_code ≠
call_code`). `is_call_key` and `is_radio_key` are mutually exclusive; `separate = true`
for all processed events. Applies to both TrackAudio and AFV.

**Key principle:** the two keys are fully independent. PTT controls the call mic; R-PTT
controls radio TX only. Explicit radio prio acts as a call-PTT mute-lock (identical to
PTT-None prio behavior): while on, PTT does not unmute the call mic. Prio has no effect
on radio TX. Implicit prio does **not** fire for PTT-Diff (`set_call_active` only fires
implicit prio for same-key configs).

#### Outside a call

| Event   | MIC          | R-TX         |
|---------|--------------|--------------|
| PTT ↓   | not attached | n/a          |
| PTT ↑   | not attached | n/a          |
| R-PTT ↓ | not attached | **Active**   |
| R-PTT ↑ | not attached | **Inactive** |

Keys are fully independent outside a call.

#### Entering a call

MIC attaches based solely on PTT state (`should_attach_input_muted = !call_pressed`).
R-PTT state and prio have no influence on the attach muted state. Implicit prio never
fires.

| PTT held? | R-PTT held? | MIC attaches | impl_prio fires? | R-TX |
|-----------|-------------|--------------|------------------|------|
| No        | No          | **Muted**    | No               | Inactive |
| Yes       | No          | **Unmuted**  | No               | Inactive |
| No        | Yes         | **Muted**    | No               | Active (was TX-ing) |
| Yes       | Yes         | **Unmuted**  | No               | Active (was TX-ing) |

#### In a call — prio off

| Event   | MIC           | R-TX         |
|---------|---------------|--------------|
| PTT ↓   | **Unmuted**   | unchanged    |
| PTT ↑   | **Muted**     | unchanged    |
| R-PTT ↓ | unchanged     | **Active**   |
| R-PTT ↑ | unchanged     | **Inactive** |

R-PTT has no MIC effect: the rx loop VA/PTT radio-key events fall through to `_ => None`.

#### In a call — prio on (explicit)

| Event   | MIC                        | R-TX         |
|---------|----------------------------|--------------|
| PTT ↓   | **Muted** (lock active)    | unchanged    |
| PTT ↑   | **Muted** (lock active)    | unchanged    |
| R-PTT ↓ | unchanged                  | **Active**   |
| R-PTT ↑ | unchanged                  | **Inactive** |

Prio is a mute-lock: `effective_prio = true` → `(PushToTalk, true, _, true) => Some(true)`
for PTT events. R-PTT is unaffected.

#### Leaving a call

`set_call_active(false)` clears both `radio_prio` and `implicit_radio_prio`.

- **PTT held:** MIC detached. PTT ↑ later: no effect.
- **R-PTT held:** MIC detached. Radio TX **continues** (R-PTT still held, no call active).
  R-PTT ↑ later → R-TX Inactive.
- **Both held:** as R-PTT case; radio TX continues.

#### Radio Prio button

Identical to PTT-None (§8.3): **no immediate MIC effect** on either toggle direction.
`set_radio_prio` matches `_ => {}` for PTT mode when `call_pressed = true`; if key is not
held, `call_pressed = false` — but PTT is a call key, not a radio key, so this path is
only reached between presses. In practice: toggle ON/OFF changes the `radio_prio` flag;
the new state takes effect on the next PTT key event. R-TX is unaffected by prio.

---

### 8.5 PTT | Same | Radio — PushToTalk + same key for call PTT and radio PTT

**Keys:** one physical key (key_code) that acts as radio PTT outside a call and as call
PTT inside a call, with the role resolved by prio.
Applies to both TrackAudio and AFV.

**Key principle:** outside a call the key always controls radio. Inside a call the prio
flag determines whether the key acts as call PTT (prio off) or radio PTT (prio on).

#### Outside a call

| Event  | MIC          | R-TX         |
|--------|--------------|--------------|
| Key ↓  | not attached | **Active**   |
| Key ↑  | not attached | **Inactive** |

#### Entering a call

| Key held? | prio flag | MIC attaches | impl_prio fires? | R-TX |
|-----------|-----------|--------------|------------------|------|
| No        | off       | **Muted**    | No               | Inactive |
| No        | on        | **Muted**    | No               | Inactive |
| Yes       | off       | **Muted**    | Yes (→ prio on)  | Active (continues) |
| Yes       | on        | **Muted**    | Yes              | Active (continues) |

When key is held at entry: impl_prio fires → radio_prio becomes true temporarily.
`should_attach_input_muted` sees radio_prio = true → attaches muted. Radio TX continues
uninterrupted across the call boundary.

#### In a call — prio off

The key acts as a pure call PTT. Radio is completely suppressed.

| Event  | MIC           | R-TX     | Notes                             |
|--------|---------------|----------|-----------------------------------|
| Key ↓  | **Unmuted**   | Inactive | Call PTT; radio suppressed        |
| Key ↑  | **Muted**     | Inactive |                                   |

#### In a call — prio on

The key acts as radio PTT. Call mic stays muted throughout.

| Event  | MIC           | R-TX         | Notes                              |
|--------|---------------|--------------|------------------------------------|
| Key ↓  | **Muted**     | **Active**   | Radio TX; call mic stays muted     |
| Key ↑  | **Muted**     | **Inactive** | TX stops; call mic stays muted     |

#### Toggling prio while key is at rest (between presses)

| Action     | Immediate MIC effect                         | On next key ↓ |
|------------|----------------------------------------------|----------------|
| Toggle ON  | No immediate change¹                         | Muted + R-TX Active |
| Toggle OFF | No immediate change¹                         | Unmuted (call PTT) |

¹ Toggling while the key is not pressed does not change the current call mic state in PTT
mode. The new prio state takes effect on the next key press.

#### Toggling prio while key is held

`set_radio_prio` matches `_ => {}` for PushToTalk mode — no immediate MIC or radio change.
The role switch takes effect on the next key event:

| Action     | On key ↑                          | On next key ↓              |
|------------|-----------------------------------|----------------------------|
| Toggle ON  | MIC **Muted**, R-TX **Inactive**¹ | MIC Muted, R-TX **Active** |
| Toggle OFF | MIC **Muted**, R-TX **Inactive**² | MIC **Unmuted** (call PTT) |

¹ `effective_prio = true` → PTT-Up arm fires `Some(true)` (muted). Radio TX condition is
true (`radio_prio_loaded = true`) so stop command fires — but radio was not active before
the toggle, so this is a no-op.

² `set_radio_prio(false)` sets `implicit_radio_prio = true` (key held, `radio_pressed =
true`). `effective_prio = false`. PTT-Up arm fires `Some(true)` (muted). Radio TX
condition: `radio_prio_loaded = false` → suppressed. Cleanup on key↑ sends radio TX stop
(TODO path: prio was already false).

#### Leaving a call

`set_call_active(false)` unconditionally clears `radio_prio` and `implicit_radio_prio`,
emitting `audio:implicit-radio-prio, false`. Prio does **not** persist across calls.

- **Key not held:** MIC detached. Both prio flags cleared.
- **Key held (prio off):** MIC detached. Key was acting as call PTT (radio suppressed).
  No radio TX command fires while key remains held (no new key events). Key ↑ later:
  radio TX stop fires (`!call_active → condition true`), but radio was never active — no-op.
- **Key held (prio on):** MIC detached. Radio TX was active. Both prio flags cleared.
  Radio TX **continues** (key still held; no call active → TX condition always true).
  Key ↑ later → R-TX Inactive.

#### Radio Prio button

`set_radio_prio` matches `_ => {}` for PushToTalk — no immediate MIC effect in any case.

| Action     | Outside a call  | In a call (key at rest)          | In a call (key held) — see above  |
|------------|-----------------|----------------------------------|-----------------------------------|
| Toggle ON  | Stores flag     | No immediate change              | No immediate change; key↑ mutes   |
| Toggle OFF | Stores flag     | No immediate change              | No immediate change; key↑ mutes   |

After Toggle ON (key at rest): next key ↓ → R-TX Active + MIC muted.
After Toggle OFF (key at rest): next key ↓ → R-TX Inactive + MIC unmuted (call PTT).

---

### 8.6 PTM | None | None — PushToMute, no radio

**Keys:** PTM (call push-to-mute). No radio code configured, so `radio_pressed` is never
set by the rx loop and `implicit_radio_prio` is never set for this config.

#### Outside a call

| Event   | MIC          | R-TX |
|---------|--------------|------|
| PTM ↓   | not attached | n/a  |
| PTM ↑   | not attached | n/a  |

#### Entering a call

- **PTM not held:** MIC **attaches unmuted** (`call_pressed = false` →
  `should_attach_input_muted` returns false).
- **PTM held:** MIC **attaches muted** (`call_pressed = true` → returns true). On first
  key↑ inside the call: MIC unmutes normally (no prio active).

#### In a call

| Event   | prio off      | prio on           | R-TX |
|---------|---------------|-------------------|------|
| PTM ↓   | **Muted**     | Muted (no change) | n/a  |
| PTM ↑   | **Unmuted**   | Muted (suppressed)| n/a  |

MIC is open by default. With no explicit prio, the key toggles mute freely. Explicit
radio prio acts as a **mute-lock**: the PTM key has no MIC effect — both ↓ and ↑ fall
to `_ => None` in the rx loop (`effective_prio = true` → arm `(PushToMute, _, _, false)`
does not match).

#### Leaving a call

- **PTM held:** MIC detached. PTM ↑ later: no effect.

#### Radio Prio button

**Toggle ON (key not held):** `set_radio_prio(true)` matches `(PushToMute,
call_pressed=false)` → `set_input_muted(true)` immediately. PTM key locked out.

**Toggle ON (key held):** no immediate MIC change (`call_pressed=true` → `_ => {}`).
PTM key locked out from this point: key↑ does not unmute; MIC stays muted after release.

**Toggle OFF (key not held):** `set_radio_prio(false)` matches `(PushToMute, false)` →
`set_input_muted(false)` immediately. PTM coupling resumes.

**Toggle OFF (key held):** no immediate MIC change. `radio_prio` is now false;
`implicit_radio_prio` is NOT set (because `radio_pressed = false` for PTM-None). MIC
**unmutes on key↑** (normal PTM release fires with `radio_prio = false`).

---

### 8.7 PTM | Same | Radio — PushToMute + same key for PTM and radio PTT

**Keys:** one physical key (`radio_code = call_code`). `is_call_key = is_radio_key = true`
for every processed event; `separate = false` always.
Applies to both TrackAudio and AFV.

**Key principle:** pressing the key always activates radio TX, unconditionally. The MIC
effects (mute on ↓, unmute on ↑) are active by default and suppressed only by *explicit*
radio prio. Implicit prio (fired at call entry when key is held) does NOT suppress PTM
unmuting — it is only for radio TX continuity tracking.

The distinction is implemented via `effective_prio = radio_prio && !implicit_radio_prio`.

#### Outside a call

| Event  | MIC          | R-TX         |
|--------|--------------|--------------|
| Key ↓  | not attached | **Active**   |
| Key ↑  | not attached | **Inactive** |

#### Entering a call

| Key held? | MIC attaches | impl_prio fires? | effective_prio | R-TX |
|-----------|--------------|------------------|----------------|------|
| No        | **Unmuted**  | No               | false          | Inactive |
| Yes       | **Muted**    | Yes              | false          | Active (continues) |

When key is held at entry: `call_pressed = true` → `should_attach_input_muted` returns
`true` → MIC attaches muted. `set_call_active` fires impl_prio: `radio_prio = true`,
`implicit_radio_prio = true`. Radio TX continues across the call boundary.

Because `implicit_radio_prio = true`, `effective_prio = false` for all subsequent events.
On first key↑ inside the call: MIC **unmutes** normally (PTM arm fires). The cleanup
path on that release clears both `implicit_radio_prio` and `radio_prio`, emitting
`audio:implicit-radio-prio, false`.

#### In a call — `effective_prio = false` (default)

| Event  | MIC         | R-TX         |
|--------|-------------|--------------|
| Key ↓  | **Muted**   | **Active**   |
| Key ↑  | **Unmuted** | **Inactive** |

PTM muting and radio TX are always coupled. This is the state when no explicit prio is
set, and also immediately after entering a call with the key held (see above).

#### In a call — `effective_prio = true` (explicit prio on)

| Event  | MIC                           | R-TX         |
|--------|-------------------------------|--------------|
| Key ↓  | Muted (already muted)         | **Active**   |
| Key ↑  | **Muted** (unmute suppressed) | **Inactive** |

Radio TX fires unconditionally. PTM unmuting is suppressed: key↑ falls to `_ => None`
in the rx loop.

#### Leaving a call

- **Key not held:** MIC detached. Nothing to track.
- **Key held:** MIC detached. `impl_prio` cleared (if set). Radio TX **continues** (key
  still held, no call active = pure radio PTT role). Key↑ later → R-TX Inactive.

#### Radio Prio button

**Toggle ON (key not held):** `set_radio_prio(true)` matches `(PushToMute,
call_pressed=false)` → `set_input_muted(true)` immediately. `implicit_radio_prio` is
NOT set (condition requires `!prio = false`). `effective_prio = true`. Key↓ → R-TX
Active only (MIC already muted). Key↑ → R-TX Inactive only (unmute suppressed).

**Toggle ON (key held):** `call_pressed = true` → `_ => {}` → no immediate MIC change.
`implicit_radio_prio` is NOT set. `effective_prio = true`. Key↑ → R-TX Inactive, MIC
stays muted (unmute suppressed).

**Toggle OFF (key not held):** `set_radio_prio(false)` matches `(PushToMute, false)` →
`set_input_muted(false)` immediately. `implicit_radio_prio` NOT set (`radio_pressed =
false`). PTM coupling resumes: key↓ = Muted + R-TX Active; key↑ = Unmuted + R-TX
Inactive.

**Toggle OFF (key held):** `call_pressed = true` → no immediate MIC change.
`radio_pressed = true` → `implicit_radio_prio = true`. `effective_prio = false (prio
just set to false)`. Key↑ → MIC **unmutes** (PTM arm fires) + R-TX Inactive. Cleanup
clears `implicit_radio_prio`.
