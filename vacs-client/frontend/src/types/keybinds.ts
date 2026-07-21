import {CallMicMode, InputBinding} from "./transmit.ts";

export type KeybindType =
    "PushToTalk" | "PushToMute" | "RadioPushToTalk" | "AcceptCall" | "EndCall" | "ToggleRadioPrio";

/// A joystick device identified by its stable SDL GUID. `name` is the
/// last-seen device name, kept for display while the device is unplugged.
export type JoystickDevice = {
    device: string;
    name?: string | null;
};

export type KeybindsConfig = {
    acceptCall: InputBinding | null;
    endCall: InputBinding | null;
    toggleRadioPrio: InputBinding | null;
    /// Devices excluded from binding capture, persisted across unplugs.
    ignoredJoysticks: JoystickDevice[];
};

export function callMicModeToKeybind(mode: CallMicMode): KeybindType | null {
    switch (mode) {
        case "PushToTalk":
            return "PushToTalk";
        case "PushToMute":
            return "PushToMute";
        case "VoiceActivation":
            return null;
    }
}
