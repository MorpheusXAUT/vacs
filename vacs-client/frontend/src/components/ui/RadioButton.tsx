import Button from "./Button.tsx";
import {useEffect, useState} from "preact/hooks";
import {clsx} from "clsx";
import {listen} from "@tauri-apps/api/event";
import {invokeStrict} from "../../error.ts";
import type {RadioState} from "../../types/radio.ts";

function RadioButton() {
    const [state, setState] = useState<RadioState>("NotConfigured");
    const disabled = state === "NotConfigured";

    useEffect(() => {
        const fetchState = async () => {
            try {
                const hasRadio = await invokeStrict<boolean>("keybinds_has_radio");
                setState(hasRadio ? "Disconnected" : "NotConfigured");
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
                return "emerald";
            case "RxIdle":
                return "emerald";
            case "RxActive":
                return "green";
            case "TxActive":
                return "cornflower";
            case "Error":
                return "red";
            default:
                return "gray";
        }
    };

    const handleButtonClick = async () => {
        if (state === "Disconnected" || state === "Error") {
            await invokeStrict("keybinds_reconnect_radio");
        }
    };

    return (
        <Button color={buttonColor()}
                disabled={disabled}
                onClick={handleButtonClick}
                className={clsx("text-xl w-46", disabled && "text-gray-500")}>
            Radio
        </Button>
    );
}

export default RadioButton;