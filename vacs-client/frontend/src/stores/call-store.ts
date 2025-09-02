import {create} from "zustand/react";

type CallDisplay = {
    type: "outgoing" | "accepted" | "rejected" | "error",
    peerId: string,
    errorReason?: string;
};

type CallState = {
    blink: boolean,
    blinkTimeoutId: number | undefined,
    callDisplay?: CallDisplay,
    incomingCalls: string[],
    actions: {
        setOutgoingCall: (peerId: string) => void,
        acceptCall: (peerId: string) => void,
        endCall: () => void,
        addIncomingCall: (peerId: string) => void,
        removePeer: (peerId: string) => void,
        rejectPeer: (peerId: string) => void,
        dismissRejectedPeer: () => void,
        errorPeer: (peerId: string, reason: string) => void,
        dismissErrorPeer: () => void,
        reset: () => void,
    },
};

export const useCallStore = create<CallState>()((set, get) => ({
    blink: false,
    blinkTimeoutId: undefined,
    callDisplay: undefined,
    incomingCalls: [],
    actions: {
        setOutgoingCall: (peerId) => {
            set({callDisplay: {type: "outgoing", peerId: peerId}});
        },
        acceptCall: (peerId) => {
            const incomingCalls = get().incomingCalls.filter(id => id !== peerId);

            if (shouldStopBlinking(incomingCalls, get().callDisplay)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined, incomingCalls: []});
            }
            set({callDisplay: {type: "accepted", peerId: peerId}, incomingCalls});
        },
        endCall: () => {
            set({callDisplay: undefined});
        },
        addIncomingCall: (peerId) => {
            const incomingCalls = get().incomingCalls.filter(id => id !== peerId);

            if (get().blinkTimeoutId === undefined) {
                startBlink(set);
            }

            set({incomingCalls: [...incomingCalls, peerId]});
        },
        removePeer: (peerId) => {
            const incomingCalls = get().incomingCalls.filter(id => id !== peerId);

            if (shouldStopBlinking(incomingCalls, get().callDisplay)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined, incomingCalls: []});
            } else {
                set({incomingCalls});
            }

            const callDisplay = get().callDisplay;
            if (callDisplay?.peerId === peerId && callDisplay?.type !== "error") {
                set({callDisplay: undefined});
            }
        },
        rejectPeer: (peerId) => {
            const callDisplay = get().callDisplay;

            if (callDisplay === undefined || callDisplay.peerId !== peerId || callDisplay.type !== "outgoing") {
                get().actions.removePeer(peerId);
                return;
            }

            set({callDisplay: {type: "rejected", peerId: peerId}});

            if (get().blinkTimeoutId === undefined) {
                startBlink(set);
            }
        },
        dismissRejectedPeer: () => {
            set({callDisplay: undefined});

            if (shouldStopBlinking(get().incomingCalls, undefined)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined});
            }
        },
        errorPeer: (peerId, reason) => {
            const callDisplay = get().callDisplay;

            if (callDisplay === undefined || callDisplay.peerId !== peerId || callDisplay.type === "rejected") {
                get().actions.removePeer(peerId);
                return;
            }

            set({callDisplay: {type: "error", peerId: peerId, errorReason: reason}});

            if (get().blinkTimeoutId === undefined) {
                startBlink(set);
            }
        },
        dismissErrorPeer: () => {
            set({callDisplay: undefined});

            if (shouldStopBlinking(get().incomingCalls, undefined)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined});
            }
        },
        reset: () => {
            clearTimeout(get().blinkTimeoutId);
            set({callDisplay: undefined, incomingCalls: [], blink: false, blinkTimeoutId: undefined})
        }
    },
}));

const shouldStopBlinking = (incomingCalls: string[], callDisplay?: CallDisplay) => {
    return incomingCalls.length === 0 && (callDisplay === undefined || (callDisplay.type !== "rejected" && callDisplay.type !== "error"));
}

const startBlink = (set: StateSetter) => {
    const toggleBlink = (blink: boolean) => {
        const timeoutId = setTimeout(() => {
            toggleBlink(!blink);
        }, 500);
        set({blinkTimeoutId: timeoutId, blink: blink});
    }
    toggleBlink(true);
}

type StateSetter = {
    (partial: (CallState | Partial<CallState> | ((state: CallState) => (CallState | Partial<CallState>))), replace?: false): void
    (state: (CallState | ((state: CallState) => CallState)), replace: true): void
};