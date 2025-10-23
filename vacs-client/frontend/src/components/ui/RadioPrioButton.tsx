import Button from "./Button.tsx";
import {useCallStore} from "../../stores/call-store.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeSafe} from "../../error.ts";
import {useEffect, useState} from "preact/hooks";
import {listen} from "@tauri-apps/api/event";

function RadioPrioButton() {
    const [prio, setPrio] = useState<boolean>(false);
    const [implicitRadioPrio, setImplicitRadioPrio] = useState<boolean>(false);
    const callDisplayType = useCallStore(state => state.callDisplay?.type);

    const handleOnClick = useAsyncDebounce(async () => {
        if (implicitRadioPrio) return;
        void invokeSafe("audio_set_radio_prio", {prio: !prio});
        setPrio(prio => !prio);
    });

    useEffect(() => {
        if (callDisplayType !== "accepted") {
            setPrio(false);
        }

        const unlisten = listen<boolean>("audio:implicit-radio-prio", (event) => {
            setImplicitRadioPrio(event.payload);
        });

        return () => unlisten.then(fn => fn());
    }, [callDisplayType]);

    return (
        <Button
            color={implicitRadioPrio || prio ? "blue" : "cyan"} className="text-lg w-46" disabled={callDisplayType !== "accepted"}
            onClick={handleOnClick}
        >
            <p>RADIO<br/>PRIO</p>
        </Button>
    );
}

export default RadioPrioButton;