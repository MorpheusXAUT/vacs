import {create} from "zustand/react";
import {invokeStrict} from "../error.ts";
import {CallConfig} from "../types/settings.ts";

type SettingsState = {
    callConfig: CallConfig;
    setCallConfig: (config: CallConfig) => void;
};

export const useSettingsStore = create<SettingsState>()(set => ({
    callConfig: {
        highlightIncomingCallTarget: true,
    },
    setCallConfig: config => set({callConfig: config}),
}));

export async function fetchCallConfig() {
    try {
        const callConfig = await invokeStrict<CallConfig>("app_get_call_config");

        useSettingsStore.getState().setCallConfig(callConfig);
    } catch {}
}
