import Button from "./Button.tsx";
import {useCallStore} from "../../stores/call-store.ts";
import {navigate} from "wouter/use-browser-location";
import {useFilterStore} from "../../stores/filter-store.ts";

function PhoneButton() {
    const blink = useCallStore(state => state.blink);
    const callDisplayType = useCallStore(state => state.callDisplay?.type);
    const setFilter = useFilterStore(state => state.setFilter);

    return (
        <Button
            color={
                callDisplayType === "accepted"
                    ? "green"
                    : callDisplayType === "outgoing"
                      ? "gray"
                      : blink
                        ? callDisplayType === "error"
                            ? "red"
                            : "green"
                        : "gray"
            }
            highlight={
                callDisplayType === "outgoing" || callDisplayType === "rejected"
                    ? "green"
                    : undefined
            }
            className="w-46 min-h-16 text-xl"
            onClick={() => {
                setFilter("");
                navigate("/");
            }}
        >
            Phone
        </Button>
    );
}

export default PhoneButton;
