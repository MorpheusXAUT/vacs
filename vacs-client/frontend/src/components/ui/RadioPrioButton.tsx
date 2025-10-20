import Button from "./Button.tsx";
import {useCallStore} from "../../stores/call-store.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeSafe} from "../../error.ts";
import {useEffect, useState} from "preact/hooks";

function RadioPrioButton() {
    const [prio, setPrio] = useState<boolean>(false);
    const callDisplayType = useCallStore(state => state.callDisplay?.type);

    const handleOnClick = useAsyncDebounce(async () => {
        void invokeSafe("audio_set_radio_prio", {prio: !prio});
        setPrio(prio => !prio);
    });

    useEffect(() => {
        if (callDisplayType !== "accepted") {
            setPrio(false);
        }
    }, [callDisplayType]);

    return (
        <Button
            color={prio ? "blue" : "cyan"} className="text-lg w-46" disabled={callDisplayType !== "accepted"}
            onClick={handleOnClick}
        >
            <p>RADIO<br/>PRIO</p>
        </Button>
    );
}

export default RadioPrioButton;