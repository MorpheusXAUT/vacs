import {DirectAccessKey} from "../../types/profile.ts";
import Button from "./Button.tsx";
import {clsx} from "clsx";
import {useStationsStore} from "../../stores/stations-store.ts";
import {useErrorOverlayStore} from "../../stores/error-overlay-store.ts";
import {startCall, useCallStore} from "../../stores/call-store.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeSafe, invokeStrict} from "../../error.ts";

type DirectAccessStationKeyProps = {
    data: DirectAccessKey;
    className?: string;
};

function DirectAccessStationKey({
    data: {stationId, label},
    className,
}: DirectAccessStationKeyProps) {
    const blink = useCallStore(state => state.blink);
    const stations = useStationsStore(state => state.stations);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);
    const openErrorOverlay = useErrorOverlayStore(state => state.open);
    const {endCall, dismissRejectedCall, dismissErrorCall} = useCallStore(state => state.actions);

    const hasStationId = stationId !== undefined;
    const station = hasStationId && stations.get(stationId);
    const online = station !== undefined;
    const own = station !== undefined && station;

    const incomingCall = incomingCalls.find(
        call => hasStationId && call.source.stationId === stationId,
    );
    const isCalling = incomingCall !== undefined;
    const beingCalled =
        hasStationId &&
        callDisplay?.type === "outgoing" &&
        callDisplay.call.target.station === stationId;
    const involved =
        callDisplay !== undefined &&
        (callDisplay.call.source.stationId === stationId ||
            callDisplay.call.target.station === stationId);
    const inCall = hasStationId && !own && involved && callDisplay.type === "accepted";
    const isRejected = hasStationId && !own && involved && callDisplay?.type === "rejected";
    const isError = hasStationId && !own && involved && callDisplay?.type === "error";

    const handleClick = useAsyncDebounce(async () => {
        if (own) {
            openErrorOverlay("Call error", "You cannot call yourself", false, 5000);
            return;
        }

        if (isCalling) {
            // Can't accept someone's call if something is in your call display
            if (callDisplay !== undefined) return;

            await invokeSafe("signaling_accept_call", {callId: incomingCall.callId});
        } else if (beingCalled || inCall) {
            try {
                await invokeStrict("signaling_end_call", {callId: callDisplay.call.callId});
                endCall();
            } catch {}
        } else if (isRejected) {
            dismissRejectedCall();
        } else if (isError) {
            dismissErrorCall();
        } else if (callDisplay === undefined) {
            await startCall({station: stationId});
        }
    });

    return (
        <Button
            color={
                inCall
                    ? "green"
                    : (isCalling || isRejected) && blink
                      ? "green"
                      : isError && blink
                        ? "red"
                        : "gray"
            }
            highlight={beingCalled || isRejected ? "green" : undefined}
            disabled={stationId === undefined || !online}
            className={clsx(
                className,
                "w-25 h-full rounded p-1.5",
                (own || !online) && "text-gray-500",
            )}
            onClick={handleClick}
        >
            {label.map((s, index) => (
                <p key={index}>{s}</p>
            ))}
        </Button>
    );
}

export default DirectAccessStationKey;
