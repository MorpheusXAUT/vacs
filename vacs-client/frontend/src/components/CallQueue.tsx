import Button from "./ui/Button.tsx";
import {useCallStore} from "../stores/call-store.ts";
import {invokeStrict} from "../error.ts";
import unplug from "../assets/unplug.svg";
import {ClientInfo} from "../types/client-info.ts";

function CallQueue() {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);
    const {
        acceptCall,
        endCall,
        dismissRejectedPeer,
        dismissErrorPeer,
        removePeer
    } = useCallStore(state => state.actions);

    const handleCallDisplayClick = async (peerId: string) => {
        if (callDisplay?.type === "accepted" || callDisplay?.type === "outgoing") {
            try {
                await invokeStrict("signaling_end_call", {peerId: peerId});
                endCall();
            } catch {
            }
        } else if (callDisplay?.type === "rejected") {
            dismissRejectedPeer();
        } else if (callDisplay?.type === "error") {
            dismissErrorPeer();
        }
    };

    const handleAnswerKeyClick = async (peer: ClientInfo) => {
        // Can't accept someone's call if something is in your call display
        if (callDisplay !== undefined) return;

        try {
            acceptCall(peer);
            await invokeStrict("signaling_accept_call", {peerId: peer.id});
        } catch {
            removePeer(peer.id);
        }
    }

    const cdColor = callDisplay?.type === "accepted" ? "green" : callDisplay?.type === "rejected" && blink ? "green" : callDisplay?.type === "error" && blink ? "red" : "gray";

    return (
        <div className="flex flex-col-reverse gap-2.5 pt-3 pr-[1px] overflow-y-auto" style={{scrollbarWidth: "none"}}>
            {/*Call Display*/}
            {callDisplay !== undefined ? (
                <div className="relative">
                    {callDisplay.connectionState === "disconnected" &&
                        <img className="absolute top-1 left-1 h-5 w-5" src={unplug} alt="Disconnected"/>}
                    <Button color={cdColor}
                            highlight={callDisplay.type === "outgoing" || callDisplay.type === "rejected" ? "green" : undefined}
                            softDisabled={true}
                            onClick={() => handleCallDisplayClick(callDisplay.peer.id)}
                            className="h-16 text-sm p-1.5"><p className="wrap-break-word max-w-full leading-3.5">{callDisplay.peer.displayName}</p></Button>
                </div>
            ) : (
                <div className="w-full border rounded-md min-h-16"></div>
            )}

            {/*Answer Keys*/}
            {incomingCalls.map((peer, idx) => (
                <Button key={idx} color={blink ? "green" : "gray"} className={"min-h-16 text-sm"}
                        onClick={() => handleAnswerKeyClick(peer)}><p>{peer.displayName}<br/>{peer.frequency}</p></Button>
            ))}
            {Array.from(Array(Math.max(5 - incomingCalls.length, 0)).keys()).map((idx) =>
                <div key={idx} className="w-full border rounded-md min-h-16"></div>
            )}
        </div>
    );
}

export default CallQueue;