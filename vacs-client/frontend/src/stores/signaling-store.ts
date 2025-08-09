import {ClientInfo} from "../types/client-info.ts";
import {create} from "zustand/react";

type SignalingState = {
    connected: boolean;
    displayName: string;
    clients: ClientInfo[];
    setConnected: (connected: boolean) => void;
    setDisplayName: (displayName: string) => void;
    setClients: (clients: ClientInfo[]) => void;
    addClient: (client: ClientInfo) => void;
    removeClient: (cid: string) => void;
}

export const useSignalingStore = create<SignalingState>()((set, get) => ({
    connected: false,
    displayName: "",
    clients: [],
    setConnected: (connected) => set({connected}),
    setDisplayName: (displayName) => set({displayName}),
    setClients: (clients) => set({clients}),
    addClient: (client) => {
        const clients = get().clients.filter(c => c.id !== client.id);
        set({clients: [...clients, client]});
    },
    removeClient: (cid) => {
        set({clients: get().clients.filter(client => client.id !== cid)});
    }
}));