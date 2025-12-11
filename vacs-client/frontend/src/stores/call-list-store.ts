import {create} from "zustand/react";

export type CallListItem = {type: "IN" | "OUT"; time: string; name: string; number: string};

type CallListState = {
    callList: CallListItem[];
    actions: {
        addCall: (call: CallListItem) => void;
        clearCallList: () => void;
    };
};

export const useCallListStore = create<CallListState>()((set, get) => ({
    callList: [],
    actions: {
        addCall: (call: CallListItem) => {
            set({callList: [call, ...get().callList]});
        },
        clearCallList: () => {
            set({callList: []});
        },
    },
}));
