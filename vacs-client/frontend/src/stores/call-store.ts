import {create} from "zustand/react";
import {CallOffer} from "../types/call.ts";

type CallState = {
    blink: boolean,
    blinkTimeoutId: number | undefined,
    callDisplay?: {
        type: "outgoing" | "accepted",
        peerId: string,
    },
    incomingCalls: CallOffer[],
    setOutgoingCall: (peerId: string) => void,
    acceptCall: (peerId: string) => void,
    endCall: () => void,
    addIncomingCall: (offer: CallOffer) => void,
    getSdpFromIncomingCall: (peerId: string) => string | undefined,
    removePeer: (peerId: string) => void,
};

export const useCallStore = create<CallState>()((set, get) => ({
    blink: false,
    blinkTimeoutId: undefined,
    callDisplay: undefined,
    incomingCalls: [],
    setOutgoingCall: (peerId) => {
        set({callDisplay: {type: "outgoing", peerId: peerId}});
    },
    acceptCall: (peerId) => {
        const incomingCalls = get().incomingCalls.filter(offer => offer.peerId !== peerId);

        if (incomingCalls.length === 0) {
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

        if (incomingCalls.length >= 5) {
            // TODO reject call
            return;
        }

        if (get().blinkTimeoutId === undefined) {
            const toggleBlink = () => {
                const timeoutId = setTimeout(() => {
                    set({blink: !get().blink});
                    toggleBlink();
                }, 500);
                set({blinkTimeoutId: timeoutId});
            }
            toggleBlink();
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
}));