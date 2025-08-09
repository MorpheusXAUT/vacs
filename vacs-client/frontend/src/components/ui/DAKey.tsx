import {ClientInfo} from "../../types/client-info.ts";
import Button from "./Button.tsx";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeSafe} from "../../error.ts";
import {useCallStore} from "../../stores/call-store.ts";

type DAKeyProps = {
    client: ClientInfo
}

function DAKey({client}: DAKeyProps) {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);
    const setOutgoingCall = useCallStore(state => state.setOutgoingCall);

    const isCalling = incomingCalls.some(call => call.peerId === client.id);
    const beingCalled = callDisplay?.type === "outgoing" && callDisplay.peerId === client.id;
    const inCall = callDisplay?.type === "accepted" && callDisplay.peerId === client.id;

    const handleClick = useAsyncDebounce(async () => {
        if (isCalling) {
            // TODO: accept call
            console.log("accepting call from " + client.id);
            return;
        } else if (beingCalled || inCall) {
            // TODO: end call
            console.log("ending call with " + client.id);
            return;
        } if (callDisplay === undefined) {
            await invokeSafe("signaling_da_key_click", {clientId: client.id});
            setOutgoingCall(client.id);
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