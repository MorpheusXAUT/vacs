import {create} from "zustand/react";
import {invokeStrict} from "../error.ts";
import {CallConfig} from "../types/settings.ts";
import {ClientPageConfig} from "../types/client.ts";

type SettingsState = {
    callConfig: CallConfig;
    clientPageConfig: ClientPageConfig;
    setCallConfig: (config: CallConfig) => void;
    setClientPageConfig: (config: ClientPageConfig) => void;
};

export const useSettingsStore = create<SettingsState>()(set => ({
    callConfig: {
        highlightIncomingCallTarget: true,
    },
    clientPageConfig: {
        include: [],
        exclude: [],
        priority: ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"],
        frequencies: "ShowAll",
        grouping: "FirAndIcao",
    },
    setCallConfig: config => set({callConfig: config}),
    setClientPageConfig: config => {
        console.log(config);
        set({clientPageConfig: config});
    },
}));

export async function fetchCallConfig() {
    try {
        const callConfig = await invokeStrict<CallConfig>("app_get_call_config");

        useSettingsStore.getState().setCallConfig(callConfig);
    } catch {}
}

export async function fetchClientPageConfig() {
    try {
        const clientPageConfig = await invokeStrict<ClientPageConfig>("app_get_client_page_config");

        useSettingsStore.getState().setClientPageConfig(clientPageConfig);
    } catch {}
}
