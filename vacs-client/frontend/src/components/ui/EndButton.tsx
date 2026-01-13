import Button from "./Button.tsx";
import {navigate} from "wouter/use-browser-location";
import {invokeStrict} from "../../error.ts";
import {useCallStore} from "../../stores/call-store.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {useFilterStore} from "../../stores/filter-store.ts";
import {useProfileStore, useProfileType} from "../../stores/profile-store.ts";
import {clsx} from "clsx";

function EndButton() {
    const callDisplay = useCallStore(state => state.callDisplay);
    const {endCall, dismissRejectedPeer, dismissErrorPeer} = useCallStore(state => state.actions);
    const setFilter = useFilterStore(state => state.setFilter);
    const setSelectedPage = useProfileStore(state => state.setPage);

    const collapsed = useProfileType() === "tabbed";

    const endAnyCall = useAsyncDebounce(async () => {
        if (callDisplay?.type === "accepted" || callDisplay?.type === "outgoing") {
            try {
                await invokeStrict("signaling_end_call", {peerId: callDisplay.peer.id});
                endCall();
            } catch {}
        } else if (callDisplay?.type === "rejected") {
            dismissRejectedPeer();
        } else if (callDisplay?.type === "error") {
            dismissErrorPeer();
        }
    });

    const handleOnClick = async () => {
        setFilter("");
        setSelectedPage(undefined);
        navigate("/");

        void endAnyCall();
    };

    return (
        <Button
            color="cyan"
            className={clsx("text-xl transition-[width]", collapsed ? "w-20" : "w-44 px-10")}
            onClick={handleOnClick}
        >
            END
        </Button>
    );
}

export default EndButton;
