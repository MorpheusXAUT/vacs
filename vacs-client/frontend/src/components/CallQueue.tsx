import Button from "./ui/Button.tsx";
import {useCallStore} from "../stores/call-store.ts";
import {invokeStrict} from "../error.ts";

function CallQueue() {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);
    const acceptCall = useCallStore(state => state.acceptCall)
    const endCall = useCallStore(state => state.endCall);

    const handleCallDisplayClick = async (peerId: string) => {
        try {
            await invokeStrict("signaling_end_call", {peerId: peerId});
            endCall();
        } catch {}
    };

    const handleAnswerKeyClick = async (peerId: string, sdp: string) => {
        if (callDisplay !== undefined) return;
        try {
            await invokeStrict("signaling_accept_call", {peerId: peerId, sdp: sdp});
            acceptCall(peerId);
        } catch {}
    }

    return (
        <div className="flex flex-col-reverse gap-3 pt-3 pr-[1px] overflow-y-auto" style={{scrollbarWidth: "none"}}>
            {/*Call Display*/}
            {callDisplay !== undefined ? (
                <Button color={callDisplay.type === "accepted" ? "green" : "gray"}
                        highlight={callDisplay.type === "outgoing" ? "green" : undefined}
                        softDisabled={true}
                        onClick={() => handleCallDisplayClick(callDisplay.peerId)}
                        className={"min-h-16 text-sm"}>{callDisplay.peerId}</Button>
            ) : (
                <div className="w-full border rounded-md min-h-16"></div>
            )}

            {/*Answer Keys*/}
            {incomingCalls.map(call => (
                <Button color={blink ? "green" : "gray"} className={"min-h-16 text-sm"}
                        onClick={() => handleAnswerKeyClick(call.peerId, call.sdp)}>{call.peerId}</Button>
            ))}
            {Array.from(Array(Math.max(5 - incomingCalls.length, 0))).map(() => <div
                className="w-full border rounded-md min-h-16"></div>)}
        </div>
    );
}

export default CallQueue;