import Select from "./ui/Select.tsx";
import {clsx} from "clsx";
import {useCallback, useEffect, useRef, useState} from "preact/hooks";
import {
    withLabels,
    isTransmitMode,
    TransmitConfig,
    TransmitConfigWithLabels
} from "../types/transmit.ts";
import {invokeSafe, invokeStrict} from "../error.ts";

function TransmitModeSettings() {
    const [transmitConfig, setTransmitConfig] = useState<TransmitConfigWithLabels | undefined>(undefined);
    const [capturing, setCapturing] = useState<boolean>(false);
    const keySelectRef = useRef<HTMLDivElement | null>(null);

    const isRemoveDisabled = transmitConfig === undefined || transmitConfig.mode === "VoiceActivation" || (transmitConfig.mode === "PushToTalk" && transmitConfig.pushToTalk === null) || (transmitConfig.mode === "PushToMute" && transmitConfig.pushToMute === null);

    const handleKeyDownEvent = useCallback(async (event: KeyboardEvent) => {
        event.preventDefault();

        // For some keys (e.g., the MediaPlayPause one), the code returned is empty and the event only contains a key.
        // Since we want to remain layout independent, we prefer to use the code value, but fall back to the key if required.
        let code = event.code || event.key;

        // Additionally, we need to check if the NumLock key is active, since the code returned by the event will always be the numpad digit,
        // however, in case it's deactivated, we want to bind the key instead (e.g., ArrowLeft instead of Numpad4).
        // The DOM_KEY_LOCATION defines the location of the key on the keyboard, where DOM_KEY_LOCATION_NUMPAD (value 3) corresponds to the numpad.
        if (event.location === KeyboardEvent.DOM_KEY_LOCATION_NUMPAD && !event.getModifierState("NumLock")) {
            code = event.key;
        }

        let newConfig: TransmitConfig;
        if (transmitConfig === undefined || transmitConfig.mode === "VoiceActivation") {
            return;
        } else if (transmitConfig.mode === "PushToTalk") {
            newConfig = {...transmitConfig, pushToTalk: code};
        } else {
            newConfig = {...transmitConfig, pushToMute: code};
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withLabels(newConfig));
        } finally {
            setCapturing(false);
        }
    }, [transmitConfig]);

    const handleClickOutside = useCallback((event: MouseEvent) => {
        if (keySelectRef.current === null || keySelectRef.current.contains(event.target as Node)) return;
        setCapturing(false);
    }, []);

    const handleKeySelectOnClick = async () => {
        if (transmitConfig === undefined || transmitConfig.mode === "VoiceActivation") return;

        void invokeSafe("audio_play_ui_click");

        setCapturing(!capturing);
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
        if (isRemoveDisabled) return;

        void invokeSafe("audio_play_ui_click");

        if (capturing) {
            setCapturing(false);
            return;
        }

        let newConfig: TransmitConfig;
        if (transmitConfig.mode === "PushToTalk") {
            newConfig = {...transmitConfig, pushToTalk: null};
        } else {
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

    useEffect(() => {
        if (!capturing) return;

        document.addEventListener("keydown", handleKeyDownEvent);
        document.addEventListener("keyup", preventKeyUpEvent);
        document.addEventListener("click", handleClickOutside);

        return () => {
            if (capturing) {
                document.removeEventListener("keydown", handleKeyDownEvent);
                document.removeEventListener("keyup", preventKeyUpEvent);
                document.removeEventListener("click", handleClickOutside);
            }
        };
    }, [capturing, handleKeyDownEvent, handleClickOutside]);

    return (
        <div className="w-full px-3 py-1.5 flex flex-row gap-3 items-center justify-center">
            {transmitConfig !== undefined ? (
                <>
                    <Select
                        className="w-min h-full !mb-0"
                        name="keybind-mode"
                        options={[
                            {value: "VoiceActivation", text: "Voice activation"},
                            {value: "PushToTalk", text: "Push-to-talk"},
                            {value: "PushToMute", text: "Push-to-mute"}
                        ]}
                        selected={transmitConfig.mode}
                        onChange={handleOnModeChange}
                    />
                    <div className="grow h-full flex flex-row items-center justify-center">
                        <div
                            ref={keySelectRef}
                            onClick={handleKeySelectOnClick}
                            className={clsx("w-full h-full truncate text-sm py-1 px-2 rounded text-center flex items-center justify-center",
                                "bg-gray-300 border-2",
                                capturing ?
                                    "border-r-gray-100 border-b-gray-100 border-t-gray-700 border-l-gray-700 [&>*]:translate-y-[1px] [&>*]:translate-x-[1px]"
                                    : "border-t-gray-100 border-l-gray-100 border-r-gray-700 border-b-gray-700",
                                transmitConfig.mode === "VoiceActivation" ? "brightness-90 cursor-not-allowed" : "cursor-pointer")}>
                            <p>{capturing ? "Press your key" : transmitConfig.mode !== "VoiceActivation" ? transmitConfig.mode === "PushToTalk" ? transmitConfig.pushToTalkLabel : transmitConfig.pushToMuteLabel : ""}</p>
                        </div>
                        <svg onClick={handleOnRemoveClick}
                             xmlns="http://www.w3.org/2000/svg" width="27" height="27"
                             viewBox="0 0 24 24" fill="none" strokeWidth="2" strokeLinecap="round"
                             strokeLinejoin="round"
                             className={clsx("p-1 !pr-0",
                                 isRemoveDisabled ?
                                     "stroke-gray-500 cursor-not-allowed"
                                     : "stroke-gray-700 hover:stroke-red-500 transition-colors cursor-pointer"
                             )}>
                            <path d="M18 6 6 18"/>
                            <path d="m6 6 12 12"/>
                        </svg>
                    </div>
                </>
            ) : <p className="w-full text-center">Loading...</p>}
        </div>
    );
}

const preventKeyUpEvent = (event: KeyboardEvent) => {
    event.preventDefault();
}

export default TransmitModeSettings;