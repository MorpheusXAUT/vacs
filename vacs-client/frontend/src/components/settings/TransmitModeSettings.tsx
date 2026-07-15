import {clsx} from "clsx";
import {TargetedEvent} from "preact";
import {useEffect, useState} from "preact/hooks";
import {invokeSafe, invokeStrict} from "../../error.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {useCapabilitiesStore} from "../../stores/capabilities-store.ts";
import {setPage} from "../../stores/navigation-store.ts";
import {useRadioStore} from "../../stores/radio-store.ts";
import {useSettingsStore} from "../../stores/settings-store.ts";
import {callMicModeToKeybind} from "../../types/keybinds.ts";
import {RadioState} from "../../types/radio.ts";
import {
    CallMicMode,
    RadioConfig,
    RadioConfigWithLabels,
    TransmitConfig,
    TransmitConfigWithLabels,
    isCallMicMode,
    isRadioIntegration,
    withRadioLabels,
    withTransmitLabels,
} from "../../types/transmit.ts";
import {openUrl} from "../../utils/tauri.ts";
import Select from "../ui/Select.tsx";
import StatusIndicator, {Status} from "../ui/StatusIndicator.tsx";
import KeyCapture from "./KeyCapture.tsx";

function TransmitModeSettings() {
    const capKeybindListener = useCapabilitiesStore(state => state.keybindListener);
    const capPlatform = useCapabilitiesStore(state => state.platform);

    const transmitConfig = useSettingsStore(state => state.transmitConfig);
    const setTransmitConfig = useSettingsStore(state => state.setTransmitConfig);
    const radioConfig = useSettingsStore(state => state.radioConfig);
    const setRadioConfig = useSettingsStore(state => state.setRadioConfig);

    return (
        <div className="h-full flex flex-col">
            <div className="flex flex-col gap-0.5">
                <div className="w-full mb-1 flex flex-row gap-2 items-center justify-center border-b-2 border-zinc-200">
                    <p className="font-semibold uppercase">Call Mode</p>
                    <HelpIcon url="https://docs.vacs.network/settings/transmit" /> {/* TODO */}
                </div>
                {!capKeybindListener ? (
                    <div className="w-full px-3 flex flex-row gap-3 items-center justify-center">
                        <p
                            className="text-sm text-gray-700 py-1.5 cursor-help"
                            title={`Unfortunately, keybinds are not yet supported on ${capPlatform}`}
                        >
                            Not available.
                        </p>
                    </div>
                ) : (
                    <>
                        <div className="w-full px-3 flex flex-row gap-3 items-center justify-center">
                            {transmitConfig !== undefined ? (
                                <TransmitConfigSettings
                                    transmitConfig={transmitConfig}
                                    setTransmitConfig={setTransmitConfig}
                                />
                            ) : (
                                <p className="w-full text-center">Loading...</p>
                            )}
                        </div>
                        <p className="py-2 px-3 text-sm text-gray-800">
                            <b>Voice activation:</b> Mic unmuted, toggle{" "}
                            <span className="bg-[#92e1fe] border-2 border-t-cyan-100 border-l-cyan-100 border-r-cyan-950 border-b-cyan-950 rounded px-1 text-xs text-black font-semibold">
                                RADIO PRIO
                            </span>{" "}
                            to mute.
                            <br />
                            <b>Push-to-talk:</b> Mic muted, press and hold key to talk in a call.
                            <br />
                            <b>Push-to-mute:</b> Mic unmuted, press and hold key to mute in a call.
                        </p>
                    </>
                )}
            </div>
            <div className="grow flex flex-col gap-0.5">
                <div className="w-full pt-1 mb-1 flex flex-row gap-2 items-center justify-center border-t-2 border-zinc-200">
                    <p className="font-semibold uppercase">Radio Integration</p>
                    <HelpIcon url="https://docs.vacs.network/settings/transmit" /> {/* TODO */}
                </div>
                {!capKeybindListener ? (
                    <div className="w-full px-3 flex flex-row gap-3 items-center justify-center">
                        <p
                            className="text-sm text-gray-700 py-1.5 cursor-help"
                            title={`Unfortunately, keybind emitters are not yet supported on ${capPlatform}`}
                        >
                            Not available.
                        </p>
                    </div>
                ) : (
                    <>
                        {transmitConfig !== undefined && radioConfig !== undefined ? (
                            <RadioIntegrationSettings
                                transmitConfig={transmitConfig}
                                radioConfig={radioConfig}
                                setTransmitConfig={setTransmitConfig}
                                setRadioConfig={setRadioConfig}
                            />
                        ) : (
                            <div className="w-full px-3 flex items-center justify-center">
                                <p className="w-full text-center">Loading...</p>
                            </div>
                        )}
                    </>
                )}
            </div>
        </div>
    );
}

type TransmitConfigSettingsProps = {
    transmitConfig: TransmitConfigWithLabels;
    setTransmitConfig: (config: TransmitConfigWithLabels) => void;
};

function TransmitConfigSettings({transmitConfig, setTransmitConfig}: TransmitConfigSettingsProps) {
    const capPlatform = useCapabilitiesStore(state => state.platform);
    const [waylandBinding, setWaylandBinding] = useState<string | undefined>(undefined);

    const handleOnTransmitCapture = async (code: string) => {
        if (transmitConfig === undefined || transmitConfig.callMicMode === "VoiceActivation")
            return;

        let newConfig: TransmitConfig;
        switch (transmitConfig.callMicMode) {
            case "PushToTalk":
                newConfig = {...transmitConfig, pushToTalk: code};
                break;
            case "PushToMute":
                newConfig = {...transmitConfig, pushToMute: code};
                break;
        }

        if (code === transmitConfig.radioPushToTalk) {
            newConfig.radioPushToTalk = null;
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withTransmitLabels(newConfig));
        } catch {}
    };

    const handleOnTransmitModeChange = async (value: string) => {
        if (!isCallMicMode(value) || transmitConfig === undefined) return;

        const previousTransmitConfig = transmitConfig;
        const newTransmitConfig: TransmitConfigWithLabels = {...transmitConfig, callMicMode: value};

        setTransmitConfig(newTransmitConfig);

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newTransmitConfig});
        } catch {
            setTransmitConfig(previousTransmitConfig);
        }
    };

    const handleOnTransmitRemoveClick = async () => {
        if (transmitConfig === undefined || transmitConfig.callMicMode === "VoiceActivation") {
            return;
        }

        let newConfig: TransmitConfig;
        switch (transmitConfig.callMicMode) {
            case "PushToTalk":
                newConfig = {...transmitConfig, pushToTalk: null};
                break;
            case "PushToMute":
                newConfig = {...transmitConfig, pushToMute: null};
                break;
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withTransmitLabels(newConfig));
        } catch {}
    };

    const handleOpenSystemShortcutsOnClick = useAsyncDebounce(async () => {
        await invokeSafe("keybinds_open_system_shortcuts_settings");
    });

    useEffect(() => {
        const fetchExternalBinding = async () => {
            const keybind = callMicModeToKeybind(transmitConfig.callMicMode);
            if (keybind === null) {
                setWaylandBinding(undefined);
                return;
            }

            const binding = await invokeSafe<string | null>("keybinds_get_external_binding", {
                keybind,
            });
            setWaylandBinding(binding ?? undefined);
        };

        if (capPlatform === "LinuxWayland" && transmitConfig !== undefined) {
            if (transmitConfig.callMicMode === "VoiceActivation") {
                setWaylandBinding(undefined);
            } else {
                void fetchExternalBinding();
            }
        }
    }, [capPlatform, transmitConfig]);

    return (
        <>
            <Select
                className="w-[21ch]! h-full"
                name="keybind-mode"
                options={[
                    {value: "VoiceActivation", text: "Voice activation"},
                    {value: "PushToTalk", text: "Push-to-talk"},
                    {value: "PushToMute", text: "Push-to-mute"},
                ]}
                selected={transmitConfig.callMicMode}
                onChange={handleOnTransmitModeChange}
            />
            {capPlatform === "LinuxWayland" ? (
                <div
                    onClick={handleOpenSystemShortcutsOnClick}
                    title={
                        transmitConfig.callMicMode !== "VoiceActivation"
                            ? "On Wayland, shortcuts are managed by the system. Please configure the shortcut in your desktop environment settings. Click this field to try opening the appropriate system settings."
                            : ""
                    }
                    className={clsx(
                        "w-full h-full min-w-0 min-h-8 grow text-sm py-1 px-2 rounded text-center flex items-center justify-center",
                        "bg-gray-300 border-2 border-t-gray-100 border-l-gray-100 border-r-gray-700 border-b-gray-700",
                        "brightness-90 cursor-help",
                        transmitConfig.callMicMode === "VoiceActivation" &&
                            "brightness-90 cursor-not-allowed",
                    )}
                >
                    <p className="truncate max-w-full">
                        {transmitConfig.callMicMode !== "VoiceActivation"
                            ? waylandBinding || "Not bound"
                            : ""}
                    </p>
                </div>
            ) : (
                <KeyCapture
                    label={
                        transmitConfig.callMicMode === "PushToTalk"
                            ? transmitConfig.pushToTalkLabel
                            : transmitConfig.callMicMode === "PushToMute"
                              ? transmitConfig.pushToMuteLabel
                              : ""
                    }
                    onCapture={handleOnTransmitCapture}
                    onRemove={handleOnTransmitRemoveClick}
                    disabled={transmitConfig.callMicMode === "VoiceActivation"}
                />
            )}
        </>
    );
}

type RadioIntegrationSettingsProps = {
    transmitConfig: TransmitConfigWithLabels;
    radioConfig: RadioConfigWithLabels;
    setTransmitConfig: (config: TransmitConfigWithLabels) => void;
    setRadioConfig: (config: RadioConfigWithLabels) => void;
};

function RadioIntegrationSettings({
    transmitConfig,
    radioConfig,
    setTransmitConfig,
    setRadioConfig,
}: RadioIntegrationSettingsProps) {
    const capKeybindEmitter = useCapabilitiesStore(state => state.keybindEmitter);
    const [trackAudioEndpoint, setTrackAudioEndpoint] = useState<string>(
        radioConfig.trackAudio?.endpoint ?? "",
    );

    const handleOnRadioIntegrationChange = async (value: string) => {
        if (radioConfig === undefined) return;

        const previousRadioConfig = radioConfig;

        let newRadioConfig;
        if (isRadioIntegration(value)) {
            newRadioConfig = {...radioConfig, integration: value};
        } else if (value === "None") {
            newRadioConfig = {...radioConfig, integration: null};
        } else {
            return;
        }

        setRadioConfig(newRadioConfig);

        try {
            await invokeStrict("radio_set_config", {radioConfig: newRadioConfig});
            if (value === "AudioForVatsim") {
                setPage("phone");
            }
        } catch {
            setRadioConfig(previousRadioConfig);
        }
    };

    const handleOnRadioPushToTalkCapture = async (code: string) => {
        if (transmitConfig === undefined || code === transmitConfig.radioPushToTalk) {
            return;
        }

        let newConfig: TransmitConfig = {...transmitConfig, radioPushToTalk: code};
        if (transmitConfig.callMicMode !== "VoiceActivation") {
            const callKey =
                transmitConfig.callMicMode === "PushToTalk"
                    ? transmitConfig.pushToTalk
                    : transmitConfig.pushToMute;

            if (callKey === code) {
                newConfig.radioPushToTalk = null;
            }
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withTransmitLabels(newConfig));
        } catch {}
    };

    const handleOnRadioPushToTalkRemoveClick = async () => {
        if (transmitConfig === undefined) {
            return;
        }

        let newConfig: TransmitConfig = {...transmitConfig, radioPushToTalk: null};

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withTransmitLabels(newConfig));
        } catch {}
    };

    const handleOnAfvEmitCapture = async (code: string) => {
        if (transmitConfig === undefined || radioConfig === undefined) {
            return;
        }

        let newConfig: RadioConfig;
        switch (radioConfig.integration) {
            case "AudioForVatsim":
                newConfig = {
                    ...radioConfig,
                    audioForVatsim: {
                        emit: code,
                    },
                };
                break;
            default:
                return;
        }

        try {
            await invokeStrict("radio_set_config", {radioConfig: newConfig});
            setRadioConfig(await withRadioLabels(newConfig));
        } catch {}
    };

    const handleOnAfvEmitRemoveClick = async () => {
        if (radioConfig === undefined) return;

        let newConfig: RadioConfig;
        switch (radioConfig.integration) {
            case "AudioForVatsim":
                newConfig = {
                    ...radioConfig,
                    audioForVatsim: {
                        emit: null,
                    },
                };
                break;
            default:
                return;
        }

        try {
            await invokeStrict("radio_set_config", {radioConfig: newConfig});
            setRadioConfig(await withRadioLabels(newConfig));
        } catch {}
    };

    const handleOnTrackAudioEndpointChange = (e: TargetedEvent<HTMLInputElement>) => {
        if (!(e.target instanceof HTMLInputElement)) return;
        setTrackAudioEndpoint(e.target.value);
    };

    const handleOnTrackAudioEndpointCommit = async () => {
        if (transmitConfig === undefined || radioConfig === undefined) {
            return;
        }

        const endpoint = trackAudioEndpoint.trim() === "" ? null : trackAudioEndpoint.trim();
        if (endpoint === radioConfig.trackAudio?.endpoint) return;

        let newConfig: RadioConfig;
        if (radioConfig.integration === "TrackAudio") {
            newConfig = {
                ...radioConfig,
                trackAudio: {
                    endpoint: endpoint,
                },
            };
            try {
                await invokeStrict("radio_set_config", {radioConfig: newConfig});
                setRadioConfig(await withRadioLabels(newConfig));
            } catch {
                setTrackAudioEndpoint(radioConfig.trackAudio?.endpoint ?? "");
            }
        }
    };

    return (
        <div className="w-full px-3 flex flex-col gap-2 items-center justify-center">
            <div className="w-full flex flex-row gap-3 items-center justify-center">
                <Select
                    className="shrink-0 w-[21ch]! h-full"
                    name="radio-integration"
                    options={[
                        {value: "None", text: "None"},
                        {value: "TrackAudio", text: "TrackAudio"},
                        ...(capKeybindEmitter
                            ? [{value: "AudioForVatsim", text: "Audio for Vatsim"}]
                            : []),
                    ]}
                    selected={radioConfig.integration ?? "None"}
                    onChange={handleOnRadioIntegrationChange}
                />
                <KeyCapture
                    label={
                        radioConfig.integration === null
                            ? ""
                            : transmitConfig.callMicMode === "VoiceActivation"
                              ? transmitConfig.radioPushToTalkLabel
                              : transmitConfig.callMicMode === "PushToTalk"
                                ? (transmitConfig.radioPushToTalkLabel ??
                                  transmitConfig.pushToTalkLabel)
                                : transmitConfig.pushToMuteLabel
                    }
                    className={clsx(
                        transmitConfig.radioPushToTalkLabel === null && "text-gray-500",
                    )}
                    disabled={
                        radioConfig.integration === null ||
                        transmitConfig.callMicMode === "PushToMute"
                    }
                    onCapture={handleOnRadioPushToTalkCapture}
                    onRemove={handleOnRadioPushToTalkRemoveClick}
                />
            </div>
            {radioConfig.integration === "TrackAudio" ? (
                <>
                    <RadioPttDescription callMicMode={transmitConfig.callMicMode} />
                    <p className="text-sm text-gray-800 leading-4.5">
                        Connection status is indicated by the button color:{" "}
                        <span className="bg-[#05cf9c] border-2 border-t-green-200 border-l-green-200 border-r-green-950 border-b-green-950 rounded px-1 text-xs text-black font-semibold">
                            Radio
                        </span>{" "}
                        (idle and ready to receive),{" "}
                        <span className="bg-[#5B95F9] border-2 border-t-blue-300 border-l-blue-300 border-r-blue-900 border-b-blue-900 rounded px-1 text-xs text-black font-semibold">
                            Radio
                        </span>{" "}
                        (receiving or transmitting), or{" "}
                        <span className="bg-red-500 border-2 border-t-red-200 border-l-red-200 border-r-red-900 border-b-red-900 rounded px-1 text-xs text-black font-semibold">
                            Radio
                        </span>{" "}
                        (error). A gray button indicates the radio is not ready.
                    </p>
                    <div className="w-full flex flex-row items-center">
                        <div className="w-full flex flex-row gap-3 items-center justify-center">
                            <p className="text-sm w-[21ch]! shrink-0 text-right">Endpoint:</p>
                            <input
                                type="text"
                                className={clsx(
                                    "w-full h-full px-3 py-1.5 border border-gray-700 bg-gray-300 rounded text-sm text-center focus:border-blue-500 focus:outline-none placeholder:text-gray-500",
                                    "disabled:brightness-90 disabled:cursor-not-allowed",
                                )}
                                placeholder="localhost:49080"
                                title="The address where TrackAudio is running. Accepts a hostname or IP address, with an optional port (e.g., '192.168.1.69' or '192.168.1.69:49080'). If you're running TrackAudio on the same machine as vacs, you can leave this value empty as it will automatically attempt to connect to TrackAudio on its default listener at 'localhost:49080'."
                                value={trackAudioEndpoint}
                                onInput={handleOnTrackAudioEndpointChange}
                                onBlur={handleOnTrackAudioEndpointCommit}
                                onKeyDown={e => {
                                    if (e.key === "Enter") {
                                        e.currentTarget.blur();
                                    }
                                }}
                            />
                        </div>
                        <div className="w-7 flex justify-center items-center p-1 pr-0!">
                            <TrackAudioStatusIndicator />
                        </div>
                    </div>
                </>
            ) : radioConfig.integration === "AudioForVatsim" ? (
                <>
                    <RadioPttDescription callMicMode={transmitConfig.callMicMode} />
                    <p className="text-sm text-gray-800 leading-4.5">
                        Set the emit key as your PTT key in AFV. You do not press it yourself, vacs
                        will do so automatically for you. Choosing a rarely used key such as
                        ScrollLock helps avoid accidental triggers.
                    </p>
                    <div className="w-full flex flex-row gap-3 items-center justify-center">
                        <p className="text-sm w-[21ch]! shrink-0 text-right">Emit key:</p>
                        <KeyCapture
                            label={radioConfig.audioForVatsim?.emitLabel ?? null}
                            onCapture={handleOnAfvEmitCapture}
                            onRemove={handleOnAfvEmitRemoveClick}
                        />
                    </div>
                </>
            ) : (
                <p className="w-full py-1 text-sm text-gray-800 leading-4.5">
                    <b>None: </b> No radio integration is configured. You can use vacs completely on
                    its own.
                    <br />
                    <b>TrackAudio: </b> vacs can connect to your TrackAudio client to trigger
                    transmissions, manage radio & frequency state and play back radio transmissions.
                    <br />
                    <b>Audio for Vatsim: </b> vacs simulates a key press for you to trigger a radio
                    transmission in AFV. The radio page and playback recording are not available.
                </p>
            )}
        </div>
    );
}

function RadioPrioBadge() {
    return (
        <span className="bg-[#92e1fe] border-2 border-t-cyan-100 border-l-cyan-100 border-r-cyan-950 border-b-cyan-950 rounded px-1 text-xs text-black font-semibold">
            RADIO PRIO
        </span>
    );
}

function RadioPttDescription({callMicMode}: {callMicMode: CallMicMode}) {
    return (
        <p className="py-1 text-sm text-gray-800 leading-4.5 w-full min-h-14">
            The key configured above is your radio push to talk. You must not bind it in your radio
            client.{" "}
            {callMicMode === "VoiceActivation" && (
                <>
                    Pressing it will transmit your voice on frequency.{" "}
                    <span className="text-red-600 font-semibold">IMPORTANT:</span> Unless you
                    manually toggle <RadioPrioBadge />, your radio transmission will be heard during
                    a call.
                    <br />
                    <br />
                </>
            )}
            {callMicMode === "PushToTalk" && (
                <>
                    When using the same key as your call PTT, toggling <RadioPrioBadge /> allows you
                    to transmit on frequency during a call. Binding a different key lets you operate
                    the radio independently.
                </>
            )}
            {callMicMode === "PushToMute" && (
                <>
                    Pressing this key during a call will transmit on frequency without being audible
                    in the call. A different radio key is not supported in push-to-mute mode, hence
                    the key capture above is disabled.
                </>
            )}
        </p>
    );
}

const RadioStateAsIndicatorState: {[key in RadioState["state"]]: Status} = {
    NotConfigured: "gray",
    Disconnected: "red",
    Error: "red",
    Connected: "green",
    VoiceConnected: "green",
    RxIdle: "green",
    RxActive: "green",
    TxActive: "green",
};

function TrackAudioStatusIndicator() {
    const radioState = useRadioStore(state => state.radioState?.state ?? "NotConfigured");
    const canReconnect =
        radioState !== "NotConfigured" && (radioState === "Disconnected" || radioState === "Error");

    const handleButtonClick = useAsyncDebounce(async () => {
        if (canReconnect) {
            await invokeStrict("radio_reconnect");
        }
    });

    const title = canReconnect
        ? "Reconnect to TrackAudio"
        : radioState !== "NotConfigured"
          ? "Connected to TrackAudio"
          : "Deactivated";

    return (
        <StatusIndicator
            status={RadioStateAsIndicatorState[radioState]}
            className={canReconnect ? "cursor-pointer" : undefined}
            onClick={handleButtonClick}
            title={title}
        />
    );
}

function HelpIcon({url}: {url: string}) {
    return (
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="stroke-gray-600 cursor-pointer"
            onClick={() => openUrl(url)}
        >
            <circle cx="12" cy="12" r="10" />
            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
            <path d="M12 17h.01" />
        </svg>
    );
}

export default TransmitModeSettings;
