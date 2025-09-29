import {ClientInfo} from "../types/client-info.ts";
import {create} from "zustand/react";

type ConnectionState = "connecting" | "connected" | "disconnected";

type SignalingState = {
    connectionState: ConnectionState;
    displayName: string;
    frequency: string;
    clients: ClientInfo[];
    setConnectionState: (state: ConnectionState) => void;
    setClientInfo: (info: Omit<ClientInfo, "id">) => void;
    setClients: (clients: ClientInfo[]) => void;
    addClient: (client: ClientInfo) => void;
    getClientInfo: (cid: string) => ClientInfo;
    removeClient: (cid: string) => void;
}

export const useSignalingStore = create<SignalingState>()((set, get) => ({
    connectionState: "disconnected",
    displayName: "",
    frequency: "",
    clients: [],
    setConnectionState: (connectionState) => set({connectionState}),
    setClientInfo: (info) => set({displayName: info.displayName, frequency: info.frequency}),
    setClients: (clients) => set({clients}),
    addClient: (client) => {
        const clients = get().clients.filter(c => c.id !== client.id);
        set({clients: [...clients, client]});
    },
    getClientInfo: (cid) => {
        const client = get().clients.find(c => c.id === cid);
        if (client === undefined) {
            return {id: cid, displayName: cid, frequency: ""};
        }
        return client;
    },
    removeClient: (cid) => {
        set({clients: get().clients.filter(client => client.id !== cid)});
    }
}));