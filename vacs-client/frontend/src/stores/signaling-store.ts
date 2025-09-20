import {ClientInfo} from "../types/client-info.ts";
import {create} from "zustand/react";

type ConnectionState = "connecting" | "connected" | "disconnected";

type SignalingState = {
    connectionState: ConnectionState;
    displayName: string;
    clients: ClientInfo[];
    setConnectionState: (state: ConnectionState) => void;
    setDisplayName: (displayName: string) => void;
    setClients: (clients: ClientInfo[]) => void;
    addClient: (client: ClientInfo) => void;
    removeClient: (cid: string) => void;
}

export const useSignalingStore = create<SignalingState>()((set, get) => ({
    connectionState: "disconnected",
    displayName: "",
    clients: [],
    setConnectionState: (connectionState) => set({connectionState}),
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