import {TransmitMode} from "./transmit";

export type KeybindType =
    | "PushToTalk"
    | "PushToMute"
    | "RadioIntegration"
    | "AcceptCall"
    | "EndCall"
    | "ToggleRadioPrio";

export type KeybindsConfig = {
    acceptCall: string | null;
    endCall: string | null;
    toggleRadioPrio: string | null;
};

export function transmitModeToKeybind(mode: TransmitMode): KeybindType | null {
    switch (mode) {
        case "PushToTalk":
            return "PushToTalk";
        case "PushToMute":
            return "PushToMute";
        case "RadioIntegration":
            return "RadioIntegration";
        case "VoiceActivation":
            return null;
    }
}
