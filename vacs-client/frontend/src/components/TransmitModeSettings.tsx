import Select from "./ui/Select.tsx";
import {Dispatch, StateUpdater, useEffect, useState} from "preact/hooks";
import {
    withLabels,
    isTransmitMode,
    TransmitConfig,
    TransmitConfigWithLabels, RadioConfig, withRadioLabels, RadioConfigWithLabels, isRadioIntegration,
} from "../types/transmit.ts";
import {invokeSafe, invokeStrict} from "../error.ts";
import KeyCapture from "./ui/KeyCapture.tsx";
import {useCapabilitiesStore} from "../stores/capabilities-store.ts";

function TransmitModeSettings() {
    const capKeybinds = useCapabilitiesStore(state => state.keybinds);
    const capPlatform = useCapabilitiesStore(state => state.platform);
    const [transmitConfig, setTransmitConfig] = useState<TransmitConfigWithLabels | undefined>(undefined);
    const [radioConfig, setRadioConfig] = useState<RadioConfigWithLabels | undefined>(undefined);

    useEffect(() => {
        const fetchConfig = async () => {
            const transmitConfig = await invokeSafe<TransmitConfig>("keybinds_get_transmit_config");
            if (transmitConfig === undefined) return;
            const radioConfig = await invokeSafe<RadioConfig>("keybinds_get_radio_config");
            if (radioConfig === undefined) return;

            setTransmitConfig(await withLabels(transmitConfig));
            setRadioConfig(await withRadioLabels(radioConfig));
        };

        if (capKeybinds) {
            void fetchConfig();
        }
    }, [capKeybinds]);

    return (
        <div className="py-0.5 flex flex-col gap-2">
            <div className="grow pt-1 flex flex-col gap-0.5">
                <p className="text-center font-semibold uppercase border-t-2 border-zinc-200">
                    Transmit Mode
                </p>
                <div className="w-full grow px-3 flex flex-row gap-3 items-center justify-center">
                    {!capKeybinds ? (
                        <p className="text-sm text-gray-700 py-1.5 cursor-help"
                           title={`Unfortunately, keybinds are not yet supported on ${capPlatform}`}
                        >Not available.</p>
                    ) : transmitConfig !== undefined ? (
                        <TransmitConfigSettings transmitConfig={transmitConfig} setTransmitConfig={setTransmitConfig}/>
                    ) : <p className="w-full text-center">Loading...</p>}
                </div>
            </div>
            <div className="grow flex flex-col gap-0.5">
                <div
                    className="w-full pt-1 flex flex-row gap-2 items-center justify-center border-t-2 border-zinc-200">
                    <p className="font-semibold uppercase">Radio Integration</p>
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24"
                         fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"
                         strokeLinejoin="round"
                         className="stroke-gray-600 cursor-help"
                    >
                        <title>Use one key for all transmissions. Configure the radio integration’s PTT key and let vacs
                            press it automatically: when not in a call, pressing the vacs PTT keys your radio; in a
                            call, it transmits on vacs; with RADIO PRIO enabled, you can (temporarily) key the radio
                            without interrupting your coordination.</title>
                        <circle cx="12" cy="12" r="10"/>
                        <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
                        <path d="M12 17h.01"/>
                    </svg>
                </div>
                <div className="w-full grow px-3 flex flex-row gap-3 items-center justify-center">
                    {!capKeybinds ? (
                        <p className="text-sm text-gray-700 py-1.5 cursor-help"
                           title={`Unfortunately, keybinds are not yet supported on ${capPlatform}`}
                        >Not available.</p>
                    ) : transmitConfig !== undefined && radioConfig !== undefined ? (
                        <RadioConfigSettings transmitConfig={transmitConfig} radioConfig={radioConfig}
                                             setRadioConfig={setRadioConfig}/>
                    ) : <p className="w-full text-center">Loading...</p>}
                </div>
            </div>
        </div>
    );
}

type TransmitConfigSettingsProps = {
    transmitConfig: TransmitConfigWithLabels;
    setTransmitConfig: Dispatch<StateUpdater<TransmitConfigWithLabels | undefined>>;
};

function TransmitConfigSettings({transmitConfig, setTransmitConfig}: TransmitConfigSettingsProps) {
    const handleOnTransmitCapture = async (code: string) => {
        if (transmitConfig === undefined || transmitConfig.mode === "VoiceActivation") return;

        let newConfig: TransmitConfig;
        switch (transmitConfig.mode) {
            case "PushToTalk":
                newConfig = {...transmitConfig, pushToTalk: code};
                break;
            case "PushToMute":
                newConfig = {...transmitConfig, pushToMute: code};
                break;
            case "RadioIntegration":
                newConfig = {...transmitConfig, radioPushToTalk: code};
                break;
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withLabels(newConfig));
        } catch {
        }
    };

    const handleOnTransmitModeChange = async (value: string) => {
        if (!isTransmitMode(value) || transmitConfig === undefined) return;

        const previousTransmitConfig = transmitConfig;
        const newTransmitConfig = {...transmitConfig, mode: value};

        setTransmitConfig(newTransmitConfig);

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newTransmitConfig});
        } catch {
            setTransmitConfig(previousTransmitConfig);
        }
    };

    const handleOnTransmitRemoveClick = async () => {
        if (transmitConfig === undefined || transmitConfig.mode === "VoiceActivation") return;

        let newConfig: TransmitConfig;
        switch (transmitConfig.mode) {
            case "PushToTalk":
                newConfig = {...transmitConfig, pushToTalk: null};
                break;
            case "PushToMute":
                newConfig = {...transmitConfig, pushToMute: null};
                break;
            case "RadioIntegration":
                newConfig = {...transmitConfig, radioPushToTalk: null};
                break;
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withLabels(newConfig));
        } catch {
        }
    };

    return (
        <>
            <Select
                className="!w-[21ch] h-full"
                name="keybind-mode"
                options={[
                    {value: "VoiceActivation", text: "Voice activation"},
                    {value: "PushToTalk", text: "Push-to-talk"},
                    {value: "PushToMute", text: "Push-to-mute"},
                    {value: "RadioIntegration", text: "Radio Integration"}
                ]}
                selected={transmitConfig.mode}
                onChange={handleOnTransmitModeChange}
            />
            <KeyCapture
                label={transmitConfig.mode === "PushToTalk" ? transmitConfig.pushToTalkLabel : transmitConfig.mode === "PushToMute" ? transmitConfig.pushToMuteLabel : transmitConfig.radioPushToTalkLabel}
                onCapture={handleOnTransmitCapture} onRemove={handleOnTransmitRemoveClick}
                disabled={transmitConfig.mode === "VoiceActivation"}/>
        </>
    );
}

type RadioConfigSettingsProps = {
    transmitConfig: TransmitConfigWithLabels;
    radioConfig: RadioConfigWithLabels;
    setRadioConfig: Dispatch<StateUpdater<RadioConfigWithLabels | undefined>>;
};

function RadioConfigSettings({transmitConfig, radioConfig, setRadioConfig}: RadioConfigSettingsProps) {
    const handleOnRadioCapture = async (code: string) => {
        if (transmitConfig === undefined || transmitConfig.mode !== "RadioIntegration" || radioConfig === undefined) {
            return;
        }

        let newConfig: RadioConfig;
        switch (radioConfig.integration) {
            case "AudioForVatsim":
                newConfig = {
                    ...radioConfig, audioForVatsim: {
                        emit: code,
                    }
                };
                break;
            case "TrackAudio":
                newConfig = {
                    ...radioConfig, trackAudio: {
                        emit: code,
                    }
                };
                break;
        }

        try {
            await invokeStrict("keybinds_set_radio_config", {radioConfig: newConfig});
            setRadioConfig(await withRadioLabels(newConfig));
        } catch {
        }
    };

    const handleOnRadioIntegrationChange = async (value: string) => {
        if (!isRadioIntegration(value) || radioConfig === undefined) return;

        const previousRadioConfig = radioConfig;
        const newRadioConfig = {...radioConfig, integration: value};

        setRadioConfig(newRadioConfig);

        try {
            await invokeStrict("keybinds_set_radio_config", {radioConfig: newRadioConfig});
        } catch {
            setRadioConfig(previousRadioConfig);
        }
    };

    const handleOnRadioRemoveClick = async () => {
        if (radioConfig === undefined) return;

        let newConfig: RadioConfig;
        switch (radioConfig.integration) {
            case "AudioForVatsim":
                newConfig = {
                    ...radioConfig, audioForVatsim: {
                        emit: null
                    }
                };
                break;
            case "TrackAudio":
                newConfig = {
                    ...radioConfig, trackAudio: {
                        emit: null
                    }
                };
                break;
        }

        try {
            await invokeStrict("keybinds_set_radio_config", {radioConfig: newConfig});
            setRadioConfig(await withRadioLabels(newConfig));
        } catch {
        }
    };

    return (
        <>
            <Select
                className="!w-[21ch] h-full"
                name="radio-integration"
                options={[
                    {value: "AudioForVatsim", text: "Audio for Vatsim"},
                    {value: "TrackAudio", text: "TrackAudio"},
                ]}
                selected={radioConfig.integration}
                onChange={handleOnRadioIntegrationChange}
                disabled={transmitConfig.mode !== "RadioIntegration"}
            />
            <KeyCapture
                label={radioConfig.integration === "AudioForVatsim" ? radioConfig.audioForVatsim && radioConfig.audioForVatsim.emitLabel : radioConfig.trackAudio && radioConfig.trackAudio.emitLabel}
                onCapture={handleOnRadioCapture} onRemove={handleOnRadioRemoveClick}
                disabled={transmitConfig.mode !== "RadioIntegration"}/>
        </>
    );
}

export default TransmitModeSettings;