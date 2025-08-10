import {ClientInfo} from "../../types/client-info.ts";
import Button from "./Button.tsx";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeStrict} from "../../error.ts";
import {useCallStore} from "../../stores/call-store.ts";

type DAKeyProps = {
    client: ClientInfo
}

function DAKey({client}: DAKeyProps) {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);
    const setOutgoingCall = useCallStore(state => state.setOutgoingCall);
    const getSdpFromIncomingCall = useCallStore(state => state.getSdpFromIncomingCall);
    const acceptCall = useCallStore(state => state.acceptCall);
    const endCall = useCallStore(state => state.endCall);

    const isCalling = incomingCalls.some(call => call.peerId === client.id);
    const beingCalled = callDisplay?.type === "outgoing" && callDisplay.peerId === client.id;
    const inCall = callDisplay?.type === "accepted" && callDisplay.peerId === client.id;

    const handleClick = useAsyncDebounce(async () => {
        if (isCalling) {
            if (callDisplay !== undefined) return;

            const sdp = getSdpFromIncomingCall(client.id);

            if (sdp === undefined) {
                console.error("Tried to accept call with no SDP present in incoming list");
                return;
            }

            try {
                await invokeStrict("signaling_accept_call", {peerId: client.id, sdp: sdp});
                acceptCall(client.id);
            } catch {}
        } else if (beingCalled || inCall) {
            try {
                await invokeStrict("signaling_end_call", {peerId: client.id});
                endCall();
            } catch {}
        } else if (callDisplay === undefined) {
            try {
                await invokeStrict("signaling_start_call", {peerId: client.id});
                setOutgoingCall(client.id);
            } catch {}
        }
    });

    return (
        <Button color={inCall ? "green" : isCalling && blink ? "green" : "gray"}
                className="w-25 h-[calc((100%-3.75rem)/6)] rounded !leading-4.5 text-lg"
                highlight={beingCalled ? "green" : undefined}
                onClick={handleClick}
        >
            {client.id}
        </Button>
    );
    // 320-340<br/>E2<br/>EC
}

export default DAKey;