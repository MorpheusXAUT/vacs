import {create} from "zustand/react";
import {ClientInfo} from "../types/client-info.ts";
import {useSignalingStore} from "./signaling-store.ts";
import {invokeSafe} from "../error.ts";
import {useErrorOverlayStore} from "./error-overlay-store.ts";
import {useAuthStore} from "./auth-store.ts";

type ConnectionState = "connecting" | "connected" | "disconnected";

type CallDisplay = {
    type: "outgoing" | "accepted" | "rejected" | "error";
    peer: ClientInfo;
    errorReason?: string;
    connectionState?: ConnectionState;
};

type CallState = {
    blink: boolean;
    blinkTimeoutId: number | undefined;
    callDisplay?: CallDisplay;
    incomingCalls: ClientInfo[];
    actions: {
        setOutgoingCall: (peer: ClientInfo) => void;
        acceptCall: (peer: ClientInfo) => void;
        endCall: () => void;
        addIncomingCall: (peer: ClientInfo) => void;
        removePeer: (peerId: string, callEnd?: boolean) => void;
        rejectPeer: (peerId: string) => void;
        dismissRejectedPeer: () => void;
        errorPeer: (peerId: string, reason: string) => void;
        dismissErrorPeer: () => void;
        setConnectionState: (peerId: string, connectionState: ConnectionState) => void;
        reset: () => void;
    };
};

export const useCallStore = create<CallState>()((set, get) => ({
    blink: false,
    blinkTimeoutId: undefined,
    callDisplay: undefined,
    incomingCalls: [],
    connecting: false,
    actions: {
        setOutgoingCall: peer => {
            set({callDisplay: {type: "outgoing", peer, connectionState: undefined}});
        },
        acceptCall: peer => {
            const incomingCalls = get().incomingCalls.filter(info => info.id !== peer.id);

            if (shouldStopBlinking(incomingCalls.length, get().callDisplay)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined, incomingCalls: []});
            }
            set({
                callDisplay: {type: "accepted", peer, connectionState: "connecting"},
                incomingCalls,
            });
        },
        endCall: () => {
            set({callDisplay: undefined});
        },
        addIncomingCall: peer => {
            const incomingCalls = get().incomingCalls.filter(info => info.id !== peer.id);

            if (get().blinkTimeoutId === undefined) {
                startBlink(set);
            }

            set({incomingCalls: [...incomingCalls, peer]});
        },
        removePeer: (peerId, callEnd) => {
            const incomingCalls = get().incomingCalls.filter(info => info.id !== peerId);

            if (shouldStopBlinking(incomingCalls.length, get().callDisplay)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined, incomingCalls: []});
            } else {
                set({incomingCalls});
            }

            const callDisplay = get().callDisplay;
            if (
                callDisplay?.peer.id === peerId &&
                callDisplay?.type !== "error" &&
                (!callEnd || callDisplay?.type !== "outgoing")
            ) {
                set({callDisplay: undefined});
            }
        },
        rejectPeer: peerId => {
            const callDisplay = get().callDisplay;

            if (
                callDisplay === undefined ||
                callDisplay.peer.id !== peerId ||
                callDisplay.type !== "outgoing"
            ) {
                get().actions.removePeer(peerId);
                return;
            }

            set({
                callDisplay: {type: "rejected", peer: callDisplay.peer, connectionState: undefined},
            });

            if (get().blinkTimeoutId === undefined) {
                startBlink(set);
            }
        },
        dismissRejectedPeer: () => {
            set({callDisplay: undefined});

            if (shouldStopBlinking(get().incomingCalls.length, undefined)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined});
            }
        },
        errorPeer: (peerId, reason) => {
            const callDisplay = get().callDisplay;

            if (
                callDisplay === undefined ||
                callDisplay.peer.id !== peerId ||
                callDisplay.type === "rejected"
            ) {
                get().actions.removePeer(peerId);
                return;
            }

            set({
                callDisplay: {
                    type: "error",
                    peer: callDisplay.peer,
                    errorReason: reason,
                    connectionState: undefined,
                },
            });

            if (get().blinkTimeoutId === undefined) {
                startBlink(set);
            }
        },
        dismissErrorPeer: () => {
            set({callDisplay: undefined});

            if (shouldStopBlinking(get().incomingCalls.length, undefined)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined});
            }
        },
        setConnectionState: (peerId, connectionState) => {
            const callDisplay = get().callDisplay;

            if (callDisplay === undefined || callDisplay.peer.id !== peerId) {
                return;
            }

            set({callDisplay: {...callDisplay, connectionState}});
        },
        reset: () => {
            clearTimeout(get().blinkTimeoutId);
            set({
                callDisplay: undefined,
                incomingCalls: [],
                blink: false,
                blinkTimeoutId: undefined,
            });
        },
    },
}));

const shouldStopBlinking = (incomingCallsLength: number, callDisplay?: CallDisplay) => {
    return (
        incomingCallsLength === 0 &&
        (callDisplay === undefined ||
            (callDisplay.type !== "rejected" && callDisplay.type !== "error"))
    );
};

const startBlink = (set: StateSetter) => {
    const toggleBlink = (blink: boolean) => {
        const timeoutId = setTimeout(() => {
            toggleBlink(!blink);
        }, 500);
        set({blinkTimeoutId: timeoutId, blink: blink});
    };
    toggleBlink(true);
};

type StateSetter = {
    (
        partial:
            | CallState
            | Partial<CallState>
            | ((state: CallState) => CallState | Partial<CallState>),
        replace?: false,
    ): void;
    (state: CallState | ((state: CallState) => CallState), replace: true): void;
};

export const startCall = async (peerOrPeerId: ClientInfo | string) => {
    const {setOutgoingCall} = useCallStore.getState().actions;
    const openErrorOverlay = useErrorOverlayStore.getState().open;
    const {getClientInfo} = useSignalingStore.getState();
    const {cid} = useAuthStore.getState();

    const peerId = typeof peerOrPeerId === "string" ? peerOrPeerId : peerOrPeerId.id;
    if (cid === peerId) {
        openErrorOverlay("Call error", "You cannot call yourself", false, 5000);
        return;
    }

    const peer = typeof peerOrPeerId === "string" ? getClientInfo(peerOrPeerId) : peerOrPeerId;

    setOutgoingCall(peer);
    await invokeSafe("signaling_start_call", {peerId});
};
