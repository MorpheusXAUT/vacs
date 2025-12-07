import {ClientInfoWithAlias, splitDisplayName} from "../../types/client-info.ts";
import Button from "./Button.tsx";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeStrict} from "../../error.ts";
import {startCall, useCallStore} from "../../stores/call-store.ts";
import {useSignalingStore} from "../../stores/signaling-store.ts";

type DAKeyProps = {
    client: ClientInfoWithAlias
}

function DAKey({client}: DAKeyProps) {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);
    const {
        acceptCall,
        endCall,
        dismissRejectedPeer,
        dismissErrorPeer
    } = useCallStore(state => state.actions);
    const selectedProfile = useSignalingStore(state => state.getActiveStationsProfileConfig());

    const isCalling = incomingCalls.some(peer => peer.id === client.id);
    const beingCalled = callDisplay?.type === "outgoing" && callDisplay.peer.id === client.id;
    const inCall = callDisplay?.type === "accepted" && callDisplay.peer.id === client.id;
    const isRejected = callDisplay?.type === "rejected" && callDisplay.peer.id === client.id;
    const isError = callDisplay?.type === "error" && callDisplay.peer.id === client.id;

    const handleClick = useAsyncDebounce(async () => {
        if (isCalling) {
            if (callDisplay !== undefined) return;

            try {
                acceptCall(client);
                await invokeStrict("signaling_accept_call", {peerId: client.id});
            } catch {}
        } else if (beingCalled || inCall) {
            try {
                await invokeStrict("signaling_end_call", {peerId: client.id});
                endCall();
            } catch {}
        } else if (isRejected) {
            dismissRejectedPeer();
        } else if (isError) {
            dismissErrorPeer();
        } else if (callDisplay === undefined) {
            await startCall(client);
        }
    });

    const [stationName, stationType] = splitDisplayName(client);
    const showFrequency = client.frequency !== "" && (
        selectedProfile?.frequencies === "ShowAll" ||
        (selectedProfile?.frequencies === "HideAliased" && client.alias === undefined));

    return (
        <Button
            color={inCall ? "green" : (isCalling || isRejected) && blink ? "green" : isError && blink ? "red" : "gray"}
            className="w-25 h-full rounded !leading-4.5 p-1.5"
            highlight={beingCalled || isRejected ? "green" : undefined}
            onClick={handleClick}
        >
            <p className="w-full truncate" title={client.displayName}>{stationName}</p>
            {stationType !== "" && <p>{stationType}</p>}
            {showFrequency && <p title={client.frequency}>{client.frequency}</p>}
        </Button>
    );
    // 320-340<br/>E2<br/>EC
}

export default DAKey;