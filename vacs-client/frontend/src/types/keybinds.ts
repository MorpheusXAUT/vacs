import {TransmitMode} from "./transmit";

export type Keybind = "PushToTalk" | "PushToMute" | "RadioIntegration" | "AcceptCall" | "EndCall";

export function transmitModeToKeybind(mode: TransmitMode): Keybind | null {
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
