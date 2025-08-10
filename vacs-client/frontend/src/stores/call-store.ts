import {create} from "zustand/react";
import {CallOffer} from "../types/call.ts";
import {invokeSafe} from "../error.ts";

type CallDisplay = {
    type: "outgoing" | "accepted" | "rejected" | "error",
    peerId: string,
};

type CallState = {
    blink: boolean,
    blinkTimeoutId: number | undefined,
    callDisplay?: CallDisplay,
    incomingCalls: CallOffer[],
    actions: {
        setOutgoingCall: (peerId: string) => void,
        acceptCall: (peerId: string) => void,
        endCall: () => void,
        addIncomingCall: (offer: CallOffer) => void,
        getSdpFromIncomingCall: (peerId: string) => string | undefined,
        removePeer: (peerId: string) => void,
        rejectPeer: (peerId: string) => void,
        dismissRejectedPeer: () => void,
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
            const incomingCalls = get().incomingCalls.filter(offer => offer.peerId !== peerId);

            if (shouldStopBlinking(incomingCalls, get().callDisplay)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined, incomingCalls: []});
            }

            set({callDisplay: {type: "accepted", peerId: peerId}, incomingCalls});
        },
        endCall: () => {
            set({callDisplay: undefined});
        },
        addIncomingCall: (offer) => {
            const incomingCalls = get().incomingCalls.filter(o => o.peerId !== offer.peerId);

            if (incomingCalls.length >= 1) {
                void invokeSafe("signaling_reject_call", {peerId: offer.peerId});
                return;
            }

            if (get().blinkTimeoutId === undefined) {
                startBlink(set, get);
            }

            set({incomingCalls: [...incomingCalls, offer]});
        },
        getSdpFromIncomingCall: (peerId: string) => {
            const call = get().incomingCalls.find(c => c.peerId === peerId);
            if (call === undefined) {
                return undefined;
            }
            return call.sdp;
        },
        removePeer: (peerId) => {
            const incomingCalls = get().incomingCalls.filter(offer => offer.peerId !== peerId);

            if (incomingCalls.length === 0) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined, incomingCalls: []});
            } else {
                set({incomingCalls});
            }

            if (get().callDisplay?.peerId === peerId) {
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
                startBlink(set, get);
            }
        },
        dismissRejectedPeer: () => {
            set({callDisplay: undefined});

            if (shouldStopBlinking(get().incomingCalls, undefined)) {
                clearTimeout(get().blinkTimeoutId);
                set({blink: false, blinkTimeoutId: undefined, incomingCalls: []});
            }
        }
    },
}));

const shouldStopBlinking = (incomingCalls: CallOffer[], callDisplay?: CallDisplay) => {
    return incomingCalls.length === 0 && (callDisplay === undefined || callDisplay.type !== "rejected");
}

const startBlink = (set: StateSetter, get: StateGetter) => {
    const toggleBlink = () => {
        const timeoutId = setTimeout(() => {
            set({blink: !get().blink});
            toggleBlink();
        }, 500);
        set({blinkTimeoutId: timeoutId});
    }
    toggleBlink();
}

type StateSetter = {
    (partial: (CallState | Partial<CallState> | ((state: CallState) => (CallState | Partial<CallState>))), replace?: false): void
    (state: (CallState | ((state: CallState) => CallState)), replace: true): void
};

type StateGetter = () => CallState;