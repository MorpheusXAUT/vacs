import Button from "./Button.tsx";
import {useEffect, useState} from "preact/hooks";
import {clsx} from "clsx";
import {listen} from "@tauri-apps/api/event";
import {invokeStrict} from "../../error.ts";
import type {RadioState} from "../../types/radio.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";

function RadioButton() {
    const [state, setState] = useState<RadioState>("NotConfigured");
    const disabled = state === "NotConfigured" || state === "Disconnected";
    const textMuted = state === "NotConfigured";

    useEffect(() => {
        const fetchState = async () => {
            try {
                const state = await invokeStrict<RadioState>("keybinds_get_radio_state");
                setState(state);
            } catch { }
        };

        void fetchState();

        const unlisten = listen<RadioState>("radio:state", (event) => {
            setState(event.payload);
        });

        return () => {
            unlisten.then(fn => fn());
        };
    }, []);

    const buttonColor = () => {
        switch (state) {
            case "NotConfigured":
            case "Disconnected":
                return "gray";
            case "Connected":
                return "gray";
            case "RxIdle":
                return "emerald";
            case "RxActive":
                return "cornflower";
            case "TxActive":
                return "cornflower";
            case "Error":
                return "red";
            default:
                return "gray";
        }
    };

    const handleButtonClick = useAsyncDebounce(async () => {
        if (state === "Disconnected" || state === "Error") {
            await invokeStrict("keybinds_reconnect_radio");
        }
    });

    return (
        <Button color={buttonColor()}
                softDisabled={disabled}
                onClick={handleButtonClick}
                className={clsx("text-xl w-46", textMuted && "text-gray-500")}>
            Radio
        </Button>
    );
}

export default RadioButton;