import {create} from "zustand/react";
import {invokeStrict} from "../error.ts";
import {isTauri} from "../transport";
import {ClientPageConfig, ClientPageSettings} from "../types/client.ts";
import {CallConfig, ClockMode} from "../types/settings.ts";
import {
    RadioConfig,
    RadioConfigWithLabels,
    TransmitConfig,
    TransmitConfigWithLabels,
    withRadioLabels,
    withTransmitLabels,
} from "../types/transmit.ts";
import {useStationsStore} from "./stations-store.ts";

type SettingsState = {
    callConfig: CallConfig;
    selectedClientPageConfig: ClientPageConfig & {name: string};
    clientPageConfigs: Record<string, ClientPageConfig>;
    transmitConfig: TransmitConfigWithLabels | undefined;
    radioConfig: RadioConfigWithLabels | undefined;
    clockMode: ClockMode;
    playbackEnabled: boolean;
    setCallConfig: (config: CallConfig) => void;
    setClientPageConfig: (config: ClientPageConfig & {name: string}) => void;
    setClientPageSettings: (settings: ClientPageSettings) => void;
    setTransmitConfig: (config: TransmitConfigWithLabels) => void;
    setRadioConfig: (config: RadioConfigWithLabels) => void;
    setClockMode: (mode: ClockMode) => void;
    setPlaybackEnabled: (enabled: boolean) => void;
};

const emptyClientPageConfig: ClientPageConfig = {
    include: [],
    exclude: [],
    priority: ["*_FMP", "*_CTR", "*_APP", "*_TWR", "*_GND"],
    frequencies: "ShowAll",
    grouping: "FirAndIcao",
};

export const useSettingsStore = create<SettingsState>()(set => ({
    callConfig: {
        highlightIncomingCallTarget: true,
        enablePriorityCalls: true,
        enableCallStartSound: true,
        enableCallEndSound: true,
        useDefaultCallSources: true,
    },
    selectedClientPageConfig: {...emptyClientPageConfig, name: "None"},
    clientPageConfigs: {},
    transmitConfig: undefined,
    radioConfig: undefined,
    clockMode: "Realtime",
    playbackEnabled: false,
    setCallConfig: config => set({callConfig: config}),
    setClientPageConfig: config => set({selectedClientPageConfig: config}),
    setClientPageSettings: ({selected, configs}) => {
        const resolvedConfig = isTauri && selected !== undefined ? configs[selected] : undefined;
        set({
            clientPageConfigs: {None: emptyClientPageConfig, ...configs},
            ...(resolvedConfig !== undefined && selected !== undefined
                ? {selectedClientPageConfig: {...resolvedConfig, name: selected}}
                : {}),
        });
    },
    setTransmitConfig: config => set({transmitConfig: config}),
    setRadioConfig: config => set({radioConfig: config}),
    setClockMode: mode => set({clockMode: mode}),
    setPlaybackEnabled: enabled => set({playbackEnabled: enabled}),
}));

useSettingsStore.subscribe((state, prev) => {
    if (state.callConfig.useDefaultCallSources === prev.callConfig.useDefaultCallSources) return;
    const {stations, positionDefaultSources, setDefaultSource, getPositionDefaultSource} =
        useStationsStore.getState();
    setDefaultSource(getPositionDefaultSource(positionDefaultSources, stations));
});

async function fetchClientPageConfigs() {
    try {
        const settings = await invokeStrict<ClientPageSettings>("app_get_client_page_settings");
        useSettingsStore.getState().setClientPageSettings(settings);
    } catch {}
}

export async function fetchSettings() {
    void fetchClientPageConfigs();

    if (!isTauri) return;

    try {
        const [callConfig, clockMode, transmitConfig, radioConfig, playbackEnabled] =
            await Promise.all([
                invokeStrict<CallConfig>("app_get_call_config"),
                invokeStrict<ClockMode>("app_get_clock_mode"),
                invokeStrict<TransmitConfig>("keybinds_get_transmit_config").then(
                    withTransmitLabels,
                ),
                invokeStrict<RadioConfig>("keybinds_get_radio_config").then(withRadioLabels),
                invokeStrict<boolean>("playback_get_enabled"),
            ]);

        useSettingsStore.setState({
            callConfig,
            clockMode,
            transmitConfig,
            radioConfig,
            playbackEnabled,
        });
    } catch {}
}
