import Button from "./ui/Button.tsx";
import {useCallStore} from "../stores/call-store.ts";

function CallQueue() {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);

    const handleCallDisplayClick = (peerId: string) => {
        // TODO end call
        console.log("ending call with " + peerId);
    };

    const handleAnswerKeyClick = (peerId: string) => {
        // TODO accept call
        console.log("accept call from " + peerId);
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
                        onClick={() => handleAnswerKeyClick(call.peerId)}>{call.peerId}</Button>
            ))}
            {Array.from(Array(Math.max(5 - incomingCalls.length, 0))).map(() => <div
                className="w-full border rounded-md min-h-16"></div>)}
        </div>
    );
}

export default CallQueue;