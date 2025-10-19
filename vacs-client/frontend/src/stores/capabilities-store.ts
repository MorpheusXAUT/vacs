import {create} from "zustand/react";
import {Capabilities, Platform} from "../types/capabilities.ts";

type CapabilitiesState = {
    alwaysOnTop: boolean,
    keybinds: boolean,
    platform: Platform,
    actions: {
        setCapabilities: (capabilities: Capabilities) => void,
    }
}

export const useCapabilitiesStore = create<CapabilitiesState>()((set) => ({
    alwaysOnTop: true,
    keybinds: true,
    platform: "Unknown",
    actions: {
        setCapabilities: (capabilities) => {
            set({...capabilities});
        }
    }
}));