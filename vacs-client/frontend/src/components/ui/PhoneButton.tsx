import Button from "./Button.tsx";
import {useCallStore} from "../../stores/call-store.ts";

function PhoneButton() {
    const blink = useCallStore(state => state.blink);
    const callDisplayType = useCallStore(state => state.callDisplay?.type);

    return (
        <Button color={callDisplayType === "accepted" ? "green" : callDisplayType === "outgoing" ? "gray" : blink ? "green" : "gray"}
                highlight={callDisplayType === "outgoing" ? "green" : undefined}
                className="w-46 min-h-16 text-xl">Phone</Button>
    );
}

export default PhoneButton;