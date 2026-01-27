// import {ClientInfo, splitDisplayName} from "../../types/client-info.ts";
// import Button from "./Button.tsx";
// import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
// import {invokeStrict} from "../../error.ts";
// import {startCall, useCallStore} from "../../stores/call-store.ts";
// import {ClientId} from "../../types/generic.ts";
//
// type DAKeyProps = {
//     client: ClientInfo;
// };
//
// function DirectAccessClientKey({client}: DAKeyProps) {
//     const blink = useCallStore(state => state.blink);
//     const callDisplay = useCallStore(state => state.callDisplay);
//     const incomingCalls = useCallStore(state => state.incomingCalls);
//     const {acceptCall, endCall, dismissRejectedCall, dismissErrorCall} = useCallStore(
//         state => state.actions,
//     );
//
//     const isCalling = incomingCalls.some(peer => peer.id === client.id);
//     const beingCalled = callDisplay?.type === "outgoing" && callDisplay.peer.id === client.id;
//     const inCall = callDisplay?.type === "accepted" && callDisplay.peer.id === client.id;
//     const isRejected = callDisplay?.type === "rejected" && callDisplay.peer.id === client.id;
//     const isError = callDisplay?.type === "error" && callDisplay.peer.id === client.id;
//
//     const handleClick = useAsyncDebounce(async () => {
//         if (isCalling) {
//             if (callDisplay !== undefined) return;
//
//             try {
//                 await invokeStrict("signaling_accept_call", {callId: client.id});
//             } catch {}
//         } else if (beingCalled || inCall) {
//             try {
//                 await invokeStrict("signaling_end_call", {callId: client.id});
//                 endCall();
//             } catch {}
//         } else if (isRejected) {
//             dismissRejectedCall();
//         } else if (isError) {
//             dismissErrorCall();
//         } else if (callDisplay === undefined) {
//             await startCall({client: client.id as ClientId}); // TODO
//         }
//     });
//
//     const [stationName, stationType] = splitDisplayName(client);
//     const showFrequency = client.frequency !== "";
//
//     return (
//         <Button
//             color={
//                 inCall
//                     ? "green"
//                     : (isCalling || isRejected) && blink
//                       ? "green"
//                       : isError && blink
//                         ? "red"
//                         : "gray"
//             }
//             className="w-25 h-full rounded leading-4.5! p-1.5"
//             highlight={beingCalled || isRejected ? "green" : undefined}
//             onClick={handleClick}
//         >
//             <p className="w-full truncate" title={client.displayName}>
//                 {stationName}
//             </p>
//             {stationType !== "" && <p>{stationType}</p>}
//             {showFrequency && <p title={client.frequency}>{client.frequency}</p>}
//         </Button>
//     );
// }
//
// export default DirectAccessClientKey;
