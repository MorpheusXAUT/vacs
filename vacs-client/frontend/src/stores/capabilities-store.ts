import {create} from "zustand/react";
import {Capabilities, Platform} from "../types/capabilities.ts";
import {invokeStrict} from "../error.ts";

type CapabilitiesState = {
    alwaysOnTop: boolean,
    keybindListener: boolean,
    keybindEmitter: boolean,
    platform: Platform,
    setCapabilities: (capabilities: Capabilities) => void,
}

export const useCapabilitiesStore = create<CapabilitiesState>()((set) => ({
    alwaysOnTop: false,
    keybindListener: false,
    keybindEmitter: false,
    platform: "Unknown",
    setCapabilities: (capabilities) => {
        set({...capabilities});
    }
}));

export const fetchCapabilities = async () => {
    try {
        const capabilities = await invokeStrict<Capabilities>("app_platform_capabilities");

        useCapabilitiesStore.getState().setCapabilities(capabilities);
    } catch {
    }
};