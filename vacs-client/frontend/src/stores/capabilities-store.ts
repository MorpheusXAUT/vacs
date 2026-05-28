import {create} from "zustand/react";
import {Capabilities} from "../types/capabilities.ts";
import {invokeStrict} from "../error.ts";

type CapabilitiesState = Capabilities & {
    setCapabilities: (capabilities: Capabilities) => void;
};

export const useCapabilitiesStore = create<CapabilitiesState>()(set => ({
    alwaysOnTop: false,
    keybindListener: false,
    keybindEmitter: false,
    playback: false,
    platform: "Unknown",
    setCapabilities: capabilities => {
        set({...capabilities});
    },
}));

export const fetchCapabilities = async () => {
    try {
        const capabilities = await invokeStrict<Capabilities>("app_platform_capabilities");

        useCapabilitiesStore.getState().setCapabilities(capabilities);
    } catch {}
};
