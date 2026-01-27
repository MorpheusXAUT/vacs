import Button from "./ui/Button.tsx";
import {useCallStore} from "../stores/call-store.ts";
import {invokeStrict} from "../error.ts";
import unplug from "../assets/unplug.svg";
import {Call} from "../types/call.ts";
import {useProfileStationKeys} from "../stores/profile-store.ts";
import {DirectAccessKey} from "../types/profile.ts";
import {ComponentChild} from "preact";
import {ClientId, PositionId, StationId} from "../types/generic.ts";

function CallQueue() {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);
    const {endCall, dismissRejectedCall, dismissErrorCall, removeCall} = useCallStore(
        state => state.actions,
    );
    const stationKeys = useProfileStationKeys();

    const handleCallDisplayClick = async (call: Call) => {
        if (callDisplay?.type === "accepted" || callDisplay?.type === "outgoing") {
            try {
                await invokeStrict("signaling_end_call", {callId: call.callId});
                endCall();
            } catch {}
        } else if (callDisplay?.type === "rejected") {
            dismissRejectedCall();
        } else if (callDisplay?.type === "error") {
            dismissErrorCall();
        }
    };

    const handleAnswerKeyClick = async (call: Call) => {
        // Can't accept someone's call if something is in your call display
        if (callDisplay !== undefined) return;

        try {
            await invokeStrict("signaling_accept_call", {callId: call.callId});
        } catch {
            removeCall(call.callId);
        }
    };

    const cdColor =
        callDisplay?.type === "accepted"
            ? "green"
            : callDisplay?.type === "rejected" && blink
              ? "green"
              : callDisplay?.type === "error" && blink
                ? "red"
                : "gray";

    return (
        <div
            className="flex flex-col-reverse gap-2.5 pt-3 pr-px overflow-y-auto"
            style={{scrollbarWidth: "none"}}
        >
            {/*Call Display*/}
            {callDisplay !== undefined ? (
                <div className="relative">
                    {callDisplay.connectionState === "disconnected" && (
                        <img
                            className="absolute top-1 left-1 h-5 w-5"
                            src={unplug}
                            alt="Disconnected"
                        />
                    )}
                    <Button
                        color={cdColor}
                        highlight={
                            callDisplay.type === "outgoing" || callDisplay.type === "rejected"
                                ? "green"
                                : undefined
                        }
                        softDisabled={true}
                        onClick={() => handleCallDisplayClick(callDisplay.call)}
                        className="h-16 text-sm p-1.5 [&_p]:leading-3.5"
                    >
                        {callDisplay.type === "outgoing" // TODO: Fix jumping text when blinking
                            ? callLabel(
                                  callDisplay.call.target.station,
                                  callDisplay.call.target.position,
                                  callDisplay.call.target.client,
                                  stationKeys,
                              )
                            : callLabel(
                                  callDisplay.call.source.stationId,
                                  callDisplay.call.source.positionId,
                                  callDisplay.call.source.clientId,
                                  stationKeys,
                              )}
                    </Button>
                </div>
            ) : (
                <div className="w-full h-16 border rounded-md"></div>
            )}

            {/*Answer Keys*/}
            {incomingCalls.map((call, idx) => (
                <Button
                    key={idx}
                    color={blink ? "green" : "gray"}
                    className="h-16 text-sm p-1.5 [&_p]:leading-3.5"
                    onClick={() => handleAnswerKeyClick(call)}
                >
                    {callLabel(
                        call.source.stationId,
                        call.source.positionId,
                        call.source.clientId,
                        stationKeys,
                    )}
                </Button>
            ))}
            {Array.from(Array(Math.max(5 - incomingCalls.length, 0)).keys()).map(idx => (
                <div key={idx} className="w-full h-16 border rounded-md"></div>
            ))}
        </div>
    );
}

const callLabel = (
    stationId: StationId | undefined,
    positionId: PositionId | undefined,
    clientId: ClientId | undefined,
    stationKeys: DirectAccessKey[],
): ComponentChild => {
    if (stationId !== undefined) {
        // TODO: Display who is being called (call.target.station)
        const station = stationKeys.find(key => key.stationId === stationId);
        if (station !== undefined) {
            return (
                <>
                    {station.label.map((s, index) => (
                        <p key={index} className="max-w-full whitespace-nowrap" title={s}>
                            {s}
                        </p>
                    ))}
                </>
            );
        }
        return (
            <p className="max-w-full whitespace-nowrap text-center w-full" title={stationId}>
                {stationId}
            </p>
        );
    } else if (positionId !== undefined) {
        // TODO: Display who is being called (call.target.station)
        return (
            <p className="max-w-full whitespace-nowrap text-center w-full" title={positionId}>
                {positionId}
            </p>
        );
    }
    return (
        <p className="max-w-full whitespace-nowrap text-center w-full" title={clientId}>
            {clientId}
        </p>
    );
};

export default CallQueue;
