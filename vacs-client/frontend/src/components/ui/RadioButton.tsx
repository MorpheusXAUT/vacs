import Button from "./Button.tsx";
import {useEffect, useState} from "preact/hooks";
import {clsx} from "clsx";
import {listen} from "@tauri-apps/api/event";
import {invokeStrict} from "../../error.ts";

type RadioButtonState = "disabled" | "enabled" | "pressed";

function RadioButton() {
    const [state, setState] = useState<RadioButtonState>("disabled");

    useEffect(() => {
        const fetchState = async () => {
            try {
                const hasRadio = await invokeStrict<boolean>("keybinds_has_radio");
                setState(hasRadio ? "enabled" : "disabled");
            } catch { }
        };

        void fetchState();

        const unlisten1 = listen<boolean>("radio:integration-available", (event) => {
            setState(state => event.payload ? (state !== "pressed" ? "enabled" : "pressed") : "disabled");
        });

        const unlisten2 = listen<"Active" | "Inactive">("radio:transmission-state", (event) => {
            setState(event.payload === "Active" ? "pressed" : "enabled");
        });

        return () => {
            unlisten1.then(fn => fn());
            unlisten2.then(fn => fn());
        };
    }, []);

    return (
        <Button color={state === "disabled" ? "gray" : state === "enabled" ? "emerald" : "cornflower"}
                disabled={state === "disabled"}
                className={clsx("text-xl w-46", state === "disabled" && "text-gray-500")}>Radio</Button>
    );
}

export default RadioButton;