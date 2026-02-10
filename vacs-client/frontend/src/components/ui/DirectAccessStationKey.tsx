import {DirectAccessKey} from "../../types/profile.ts";
import Button, {ButtonColor, ButtonHighlightColor} from "./Button.tsx";
import {clsx} from "clsx";
import {useStationsStore} from "../../stores/stations-store.ts";
import {startCall, useCallStore} from "../../stores/call-store.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeSafe, invokeStrict} from "../../error.ts";
import ButtonLabel from "./ButtonLabel.tsx";
import {useSettingsStore} from "../../stores/settings-store.ts";

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
    const {endCall, dismissRejectedCall, dismissErrorCall} = useCallStore(state => state.actions);

    const defaultStationSource = useStationsStore(state => state.defaultSource);
    const temporaryStationSource = useStationsStore(state => state.temporarySource);
    const setDefaultStationSource = useStationsStore(state => state.setDefaultSource);
    const setTemporaryStationSource = useStationsStore(state => state.setTemporarySource);

    const highlightTarget = useSettingsStore(state => state.callConfig.highlightIncomingCallTarget);
    const enablePrio = useSettingsStore(state => state.callConfig.enablePriorityCalls);

    const hasStationId = stationId !== undefined;
    const station = hasStationId && stations.get(stationId);
    const online = station !== undefined;
    const own = station !== undefined && station;

    const incomingCall = incomingCalls.find(
        call => hasStationId && call.source.stationId === stationId,
    );
    const isCalling = incomingCall !== undefined && !own;
    const beingCalled =
        hasStationId &&
        !own &&
        callDisplay?.type === "outgoing" &&
        callDisplay.call.target.station === stationId;
    const involved =
        !own &&
        callDisplay !== undefined &&
        (callDisplay.call.source.stationId === stationId ||
            callDisplay.call.target.station === stationId);
    const inCall = hasStationId && involved && callDisplay.type === "accepted";
    const isRejected = hasStationId && involved && callDisplay?.type === "rejected";
    const isError = hasStationId && involved && callDisplay?.type === "error";

    const isTarget =
        highlightTarget &&
        hasStationId &&
        (incomingCalls.some(call => call.target.station === stationId) ||
            (own &&
                callDisplay?.type === "accepted" &&
                callDisplay.call.target.station === stationId));

    const handleClick = useAsyncDebounce(async () => {
        if (own) {
            if (defaultStationSource !== stationId && temporaryStationSource !== stationId) {
                setTemporaryStationSource(stationId);
            } else if (
                temporaryStationSource === stationId &&
                defaultStationSource !== stationId &&
                defaultStationSource === undefined
            ) {
                setDefaultStationSource(stationId);
                setTemporaryStationSource(undefined);
            } else if (defaultStationSource === stationId) {
                setDefaultStationSource(undefined);
            } else {
                setTemporaryStationSource(undefined);
            }
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

    const outgoingPrio = callDisplay?.call.prio === true && enablePrio;
    const incomingPrio = incomingCall?.prio === true && enablePrio;

    const color: ButtonColor = inCall
        ? outgoingPrio
            ? "yellow"
            : "green"
        : isCalling && blink
          ? incomingPrio
              ? "yellow"
              : "green"
          : isCalling && !blink
            ? "gray"
            : beingCalled && outgoingPrio && blink
              ? "yellow"
              : beingCalled && outgoingPrio && !blink
                ? "gray"
                : isRejected && blink
                  ? "green"
                  : isError && blink
                    ? "red"
                    : isTarget
                      ? "sage"
                      : temporaryStationSource === stationId && temporaryStationSource !== undefined
                        ? "peach"
                        : defaultStationSource === stationId && defaultStationSource !== undefined
                          ? "honey"
                          : "gray";

    const highlight: ButtonHighlightColor | undefined =
        isCalling && incomingPrio
            ? blink
                ? "green"
                : "gray"
            : beingCalled || isRejected || (inCall && outgoingPrio)
              ? "green"
              : undefined;

    return (
        <Button
            color={color}
            highlight={highlight}
            disabled={stationId === undefined || !online}
            className={clsx(
                className,
                "w-25 h-full rounded",
                (own || !hasStationId) && "text-gray-500",
                color === "gray" ? "p-1.5" : "p-[calc(0.375rem+1px)]",
            )}
            onClick={handleClick}
        >
            <ButtonLabel label={label} />
        </Button>
    );
}

export default DirectAccessStationKey;
