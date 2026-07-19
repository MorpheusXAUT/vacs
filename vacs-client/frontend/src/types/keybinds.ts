import {CallMicMode, InputBinding} from "./transmit.ts";

export type KeybindType =
    "PushToTalk" | "PushToMute" | "RadioPushToTalk" | "AcceptCall" | "EndCall" | "ToggleRadioPrio";

export type KeybindsConfig = {
    acceptCall: InputBinding | null;
    endCall: InputBinding | null;
    toggleRadioPrio: InputBinding | null;
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
