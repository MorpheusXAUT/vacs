import {ClientInfo} from "../types/client-info.ts";
import {create} from "zustand/react";

type SignalingState = {
    connected: boolean;
    clients: ClientInfo[];
    setConnected: (connected: boolean) => void;
    setClients: (clients: ClientInfo[]) => void;
}

export const useSignalingStore = create<SignalingState>()((set) => ({
    connected: false,
    clients: [],
    setConnected: (connected) => set({connected}),
    setClients: (clients) => set({clients}),
}));