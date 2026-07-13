import {CallMicMode} from "./transmit.ts";

export type KeybindType =
    "PushToTalk" | "PushToMute" | "RadioIntegration" | "AcceptCall" | "EndCall" | "ToggleRadioPrio";

export type KeybindsConfig = {
    acceptCall: string | null;
    endCall: string | null;
    toggleRadioPrio: string | null;
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
