import Select from "./ui/Select.tsx";
import {clsx} from "clsx";
import {useCallback, useEffect, useRef, useState} from "preact/hooks";
import {codeToLabel, isTransmitMode, TransmitConfig} from "../types/transmit.ts";
import {invokeSafe, invokeStrict} from "../error.ts";

function TransmitModeSettings() {
    const [transmitConfig, setTransmitConfig] = useState<TransmitConfig | undefined>(undefined);
    const pushToTalkLabel = useRef<string | undefined>(undefined);
    const pushToMuteLabel = useRef<string | undefined>(undefined);
    const [capturing, setCapturing] = useState<boolean>(false);
    const transmitConfigRef = useRef<TransmitConfig>();

    const handleKeyDownEvent = useCallback(async (event: KeyboardEvent) => {
        event.preventDefault();

        console.log("loo");

        let newConfig: TransmitConfig;
        const transmitConfig = transmitConfigRef.current;
        if (transmitConfig === undefined || transmitConfig.mode === "VoiceActivation") {
            return;
        } else if (transmitConfig.mode === "PushToTalk") {
            newConfig = {...transmitConfig, pushToTalk: event.code};
        } else {
            newConfig = {...transmitConfig, pushToMute: event.code};
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            pushToTalkLabel.current = newConfig.pushToTalk && await codeToLabel(newConfig.pushToTalk);
            pushToMuteLabel.current = newConfig.pushToMute && await codeToLabel(newConfig.pushToMute);
            setTransmitConfig(newConfig);
        } finally {
            setCapturing(false);
            document.removeEventListener("keydown", handleKeyDownEvent);
            document.removeEventListener("keyup", preventKeyUpEvent);
        }
    }, []);

    const handleClickEvent = useCallback(() => {
        setCapturing(false);
        document.removeEventListener("keydown", handleKeyDownEvent);
        document.removeEventListener("keyup", preventKeyUpEvent);
        document.removeEventListener("click", handleClickEvent);
    }, []);

    const handleKeySelectOnClick = async () => {
        if (transmitConfig === undefined || transmitConfig.mode === "VoiceActivation") return;

        if (capturing) {
            setCapturing(false);
            document.removeEventListener("keydown", handleKeyDownEvent);
            document.removeEventListener("keyup", preventKeyUpEvent);
            document.addEventListener("click", handleClickEvent);
        } else {
            setCapturing(true);
            document.addEventListener("keydown", handleKeyDownEvent);
            document.addEventListener("keyup", preventKeyUpEvent);
            document.removeEventListener("click", handleClickEvent);
        }
    };

    const handleOnModeChange = async (value: string) => {
        if (isTransmitMode(value)) {
            const previousTransmitConfig = transmitConfig;

            setTransmitConfig(config => {
                if (config === undefined) return;
                return {...config, mode: value};
            });

            try {
                await invokeStrict("keybinds_set_transmit_config", {transmitConfig: transmitConfig});
            } catch {
                setTransmitConfig(previousTransmitConfig);
            }
        }
    };

    const isRemoveDisabled = transmitConfig === undefined || transmitConfig.mode === "VoiceActivation" || (transmitConfig.mode === "PushToTalk" && transmitConfig.pushToTalk === undefined) || (transmitConfig.mode === "PushToMute" && transmitConfig.pushToMute === undefined);

    const handleOnRemoveClick = async () => {
        if (isRemoveDisabled) return;

        let newConfig: TransmitConfig;
        if (transmitConfig.mode === "PushToTalk") {
            newConfig = {...transmitConfig, pushToTalk: undefined};
        } else {
            newConfig = {...transmitConfig, pushToMute: undefined};
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            pushToTalkLabel.current = newConfig.pushToTalk && await codeToLabel(newConfig.pushToTalk);
            pushToMuteLabel.current = newConfig.pushToMute && await codeToLabel(newConfig.pushToMute);
            setTransmitConfig(newConfig);
        } catch {
        }
    };

    useEffect(() => {
        const fetchConfig = async () => {
            const config = await invokeSafe<TransmitConfig>("keybinds_get_transmit_config");
            if (config === undefined) return;

            pushToTalkLabel.current = config.pushToTalk && await codeToLabel(config.pushToTalk);
            pushToMuteLabel.current = config.pushToMute && await codeToLabel(config.pushToMute);

            setTransmitConfig(config);
        };
        void fetchConfig();

        return () => {
            document.removeEventListener("keydown", handleKeyDownEvent);
            document.removeEventListener("keyup", preventKeyUpEvent);
            document.removeEventListener("click", handleClickEvent);
        };
    }, []);

    useEffect(() => {
        transmitConfigRef.current = transmitConfig;
    }, [transmitConfig]);

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
                            onKeyDown={(e) => capturing && handleKeyDownEvent(e)}
                            onKeyUp={(e) => capturing && preventKeyUpEvent(e)}
                            onClick={handleKeySelectOnClick}
                            className={clsx("relative w-full h-full truncate text-sm py-1 px-2 rounded text-center flex items-center justify-center",
                                "bg-gray-300 border-2 ",
                                capturing ?
                                    "border-r-gray-100 border-b-gray-100 border-t-gray-700 border-l-gray-700 [&>*]:translate-y-[1px] [&>*]:translate-x-[1px]"
                                    : "border-t-gray-100 border-l-gray-100 border-r-gray-700 border-b-gray-700",
                                transmitConfig.mode === "VoiceActivation" && "brightness-90 cursor-not-allowed")}>
                            <p>{capturing ? "Press your key" : transmitConfig.mode !== "VoiceActivation" ? transmitConfig.mode === "PushToTalk" ? pushToTalkLabel.current : pushToMuteLabel.current : ""}</p>
                        </div>
                        <svg onClick={handleOnRemoveClick}
                             xmlns="http://www.w3.org/2000/svg" width="27" height="27"
                             viewBox="0 0 24 24" fill="none" strokeWidth="2" strokeLinecap="round"
                             strokeLinejoin="round"
                             className={clsx("p-1 !pr-0", isRemoveDisabled ? "stroke-gray-500 cursor-not-allowed" : "stroke-gray-700 hover:stroke-red-500 transition-colors")}>
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