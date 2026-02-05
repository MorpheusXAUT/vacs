# Client Configuration

This reference explains how to control various aspects of the client's behavior, such as window settings, release channels, and ignore lists using the `ClientConfig` settings, read from the (optional) `client.toml` config file.

For general information on the configuration file format, file locations, and recommended editors, please refer to the [main configuration reference](README.md).

## Overview

The `client` configuration allows you to control the client's behavior and local logic. It consists of the following sections:

- **[Ignore list](#ignore-list)** - Manage ignored users
- **[Extra stations config](#extra-stations-config)** - Load an additional stations config file
- **[Selected stations profile](#selected-stations-profile)** - Currently active stations profile
- **[Transmit configuration](#transmit-configuration)** - Configure transmission mode and PTT keys
- **[Keybinds](#keybinds)** - Configure general keybinds
- **[Call](#call)** - Configure call behavior

## Configuration structure

```toml
[client]
ignored = []
selected_stations_profile = "Default"
# extra_stations_config = "/path/to/extra_stations.toml"

[client.transmit_config]
mode = "VoiceActivation" # or "PushToTalk", "PushToMute", "RadioIntegration"
# push_to_talk = "ShiftRight"
# push_to_mute = "AltRight"
# radio_push_to_talk = "ControlRight"

[client.keybinds]
# accept_call = "ControlRight"
# end_call = "ControlRight"

[client.call]
# highlight_incoming_call_target = true

```

---

## Ignore list

The `ignored` list allows you to completely ignore specific users client-side (identified by their VATSIM CID).

**Type:** Array of strings (CIDs)  
**Default:** `[]` (empty list)  
**Optional:** Yes

Any incoming calls initiated by a CID in this list will be silently ignored by the client. Their call attempts will also not show up in your call history, however to an ignored user, it will still look like you are online and simply not answering their calls.

**This is not a block feature:** You can still initiate calls to users in your ignore list. The setting only suppresses _incoming_ interactions.

> [!NOTE]  
> This is a global setting and independent from your currently selected [stations profile](stations.md#profiles).

You can change this list manually in the configuration file before startup, or by going to the `Telephone` page in the client and modifying the list of ignored users in the `Ign.` tab. Alternatively, you can select a call from the `Call List` and ignore the caller using the `Ignore CID` button.

Note that all changes made to the config file only apply after `vacs` has been restarted and might be overwritten if you change any other settings via the UI.

**Example:**

```toml
[client]
# Ignore calls from these CIDs
ignored = ["10000003", "1234567"]
```

---

## Extra stations config

The `extra_stations_config` setting allows you to load an additional stations configuration file. This is useful for including the stations profiles provided by your NAV team in your FIR's sector file.

**Type:** String (Path)  
**Default:** None  
**Optional:** Yes

The path can be absolute or relative to the configuration directory. You can pick an extra stations config file to apply via the `Mission` page in the client.

> [!WARNING]  
> Under Windows, all backslashes in the path must be escaped with another backslash (e.g., `C:\\Users\\user\\Documents\\EuroScope\\SectorFile\\stations.toml`).

**Example:**

```toml
[client]
extra_stations_config = "/home/user/Documents/EuroScope/SectorFile/stations.toml"
```

> [!NOTE]  
> The extra stations config is merged with the main stations config. If there are any conflicts, the extra stations config takes precedence.

---

## Selected stations profile

The `selected_stations_profile` setting determines which [stations profile](stations.md#profiles) is currently active.

**Type:** String  
**Default:** `"Default"`  
**Optional:** Yes

This value is updated automatically when you switch profiles in the client UI.

---

## Transmit configuration

The `transmit_config` section controls how your voice is transmitted during calls.

#### `mode`: transmission mode

**Type:** String (Enum)  
**Default:** `"VoiceActivation"`  
**Possible values:**

- `"VoiceActivation"`: Microphone is open when you speak.
- `"PushToTalk"`: Microphone is open only when the PTT key is held.
- `"PushToMute"`: Microphone is open unless the PTM key is held.
- `"RadioIntegration"`: Microphone is managed by the radio integration (requires `radio_push_to_talk` key).

---

#### `push_to_talk`: Push-to-talk key

**Type:** String (Key Code)  
**Optional:** Yes (Required if `mode` is `"PushToTalk"`)

Key code for Push-to-Talk mode.

---

#### `push_to_mute`: Push-to-mute key

**Type:** String (Key Code)  
**Optional:** Yes (Required if `mode` is `"PushToMute"`)

Key code for Push-to-Mute mode.

---

#### `radio_push_to_talk`: Radio Integration Push-to-talk key

**Type:** String (Key Code)  
**Optional:** Yes (Required if `mode` is `"RadioIntegration"`)

Key code for Radio Integration Push-to-talk.

---

## Keybinds

The `keybinds` section allows you to configure global keybinds. These keybinds work independently of the application focus.

#### `accept_call`: accept call key

**Type:** String (Key Code)  
**Optional:** Yes

Key code to accept an incoming call.

---

#### `end_call`: end call key

**Type:** String (Key Code)  
**Optional:** Yes

Key code to end an active call.

---

### Tips

> [!NOTE]  
> Even if you use `VoiceActivation` transmit mode (which normally requires no keybinds), you can still define `accept_call` and `end_call` keybinds to control calls via keyboard.

> [!TIP]
> **Contextual Call Control**
>
> By assigning the same key to both `accept_call` and `end_call`, you enable contextual behavior based on the current call state:
>
> - **Incoming call:** Accepts the call.
> - **Active call:** Ends the current call.
> - **Active call + new incoming call:** First press ends the current call, second press accepts the incoming one.

### Complete Examples

#### Example 1: Standard Push-to-Talk

Simple setup with separate keys for PTT and call control.

```toml
[client.transmit_config]
mode = "PushToTalk"
push_to_talk = "ShiftRight"

[client.keybinds]
accept_call = "ControlRight"
end_call = "AltRight"
```

#### Example 2: Voice Activation with Call Control

Use voice activation for transmission but keep call controls on keys.

```toml
[client.transmit_config]
mode = "VoiceActivation"

[client.keybinds]
accept_call = "ControlRight"
end_call = "AltRight"
```

#### Example 3: Contextual Call Control (Single Key)

Combine accept and end call into a single context-aware key (`ControlRight`).

```toml
[client.transmit_config]
mode = "PushToTalk"
push_to_talk = "ShiftRight"

[client.keybinds]
# Use ControlRight for both actions
accept_call = "ControlRight"
end_call = "ControlRight"
```

---

## Call

The `call` section allows you to configure call behavior.

#### `highlight_incoming_call_target`: toggle highlighting for incoming call targets

**Type:** Boolean  
**Optional:** Yes

If enabled, the DA key of the incoming call target will be highlighted in a sage green color.
