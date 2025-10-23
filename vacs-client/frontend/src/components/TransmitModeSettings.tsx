import Select from "./ui/Select.tsx";
import {useEffect, useState} from "preact/hooks";
import {
    withLabels,
    isTransmitMode,
    TransmitConfig,
    TransmitConfigWithLabels
} from "../types/transmit.ts";
import {invokeSafe, invokeStrict} from "../error.ts";
import KeyCapture from "./ui/KeyCapture.tsx";

function TransmitModeSettings() {
    const [transmitConfig, setTransmitConfig] = useState<TransmitConfigWithLabels | undefined>(undefined);

    const handleOnCapture = async (code: string) => {
        let newConfig: TransmitConfig;
        if (transmitConfig === undefined || transmitConfig.mode === "VoiceActivation") {
            return;
        } else if (transmitConfig.mode === "PushToTalk") {
            newConfig = {...transmitConfig, pushToTalk: code};
        } else if (transmitConfig.mode === "PushToMute") {
            newConfig = {...transmitConfig, pushToMute: code};
        } else {
            // TODO
            newConfig = {...transmitConfig, pushToMute: code};
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withLabels(newConfig));
        } catch {
        }
    };

    const handleOnModeChange = async (value: string) => {
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

    const handleOnRemoveClick = async () => {
        if (transmitConfig === undefined) return;

        let newConfig: TransmitConfig;
        if (transmitConfig.mode === "PushToTalk") {
            newConfig = {...transmitConfig, pushToTalk: null};
        } else if (transmitConfig.mode === "PushToMute") {
            newConfig = {...transmitConfig, pushToMute: null};
        } else {
            // TODO: RadioIntegration?
            newConfig = {...transmitConfig, pushToMute: null};
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withLabels(newConfig));
        } catch {
        }
    };

    useEffect(() => {
        const fetchConfig = async () => {
            const config = await invokeSafe<TransmitConfig>("keybinds_get_transmit_config");
            if (config === undefined) return;

            setTransmitConfig(await withLabels(config));
        };
        void fetchConfig();
    }, []);

    return (
        <div className="py-0.5 flex flex-col gap-2">
            {transmitConfig !== undefined ? (
                <>
                    <div className="grow flex flex-col gap-0.5">
                        <p className="text-center font-semibold pt-1 uppercase border-t-2 border-zinc-200">Transmit Mode</p>
                        <div className="w-full grow px-3 flex flex-row gap-3 items-center justify-center">
                            <Select
                                className="w-min h-full !mb-0"
                                name="keybind-mode"
                                options={[
                                    {value: "VoiceActivation", text: "Voice activation"},
                                    {value: "PushToTalk", text: "Push-to-talk"},
                                    {value: "PushToMute", text: "Push-to-mute"},
                                    {value: "RadioIntegration", text: "Radio Integration"}
                                ]}
                                selected={transmitConfig.mode}
                                onChange={handleOnModeChange}
                            />
                            <KeyCapture
                                label={transmitConfig.mode === "PushToTalk" ? transmitConfig.pushToTalkLabel : transmitConfig.pushToMuteLabel}
                                onCapture={handleOnCapture} onRemove={handleOnRemoveClick}
                                disabled={transmitConfig.mode === "VoiceActivation"}/>
                        </div>
                    </div>
                    <div className="grow flex flex-col gap-0.5">
                        <div className="w-full flex flex-row gap-2 items-center justify-center border-t-2 border-zinc-200">
                            <p className="font-semibold pt-1 uppercase">Radio Integration</p>
                            <span className="text-sm w-4 h-4 text-center bg-gray-400 text-gray-300 rounded-full">?</span>
                        </div>
                        <div className="w-full grow px-3 flex flex-row gap-3 items-center justify-center">
                            <Select
                                className="w-min h-full !mb-0"
                                name="external-keybind-mode"
                                options={[
                                    {value: "afv", text: "Audio-for-Vatsim"},
                                    {value: "trackaudio", text: "TrackAudio"},
                                ]}
                                selected={"afv"}
                                onChange={handleOnModeChange}
                                disabled={transmitConfig.mode !== "RadioIntegration"}
                            />
                            <KeyCapture
                                label={transmitConfig.mode === "PushToTalk" ? transmitConfig.pushToTalkLabel : transmitConfig.pushToMuteLabel}
                                onCapture={handleOnCapture} onRemove={handleOnRemoveClick}
                                disabled={transmitConfig.mode !== "RadioIntegration"}/>
                        </div>
                    </div>
                </>
            ) : <p className="w-full text-center">Loading...</p>}
        </div>
    );
}

export default TransmitModeSettings;