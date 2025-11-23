import {ClientInfo, sortClients} from "../types/client-info.ts";
import {create} from "zustand/react";

type ConnectionState = "connecting" | "connected" | "disconnected";

type SignalingState = {
    connectionState: ConnectionState;
    id: string;
    displayName: string;
    frequency: string;
    clients: ClientInfo[];
    setConnectionState: (state: ConnectionState) => void;
    setClientInfo: (info: ClientInfo) => void;
    setClients: (clients: ClientInfo[]) => void;
    addClient: (client: ClientInfo) => void;
    getClientInfo: (cid: string) => ClientInfo;
    removeClient: (cid: string) => void;
}

export const useSignalingStore = create<SignalingState>()((set, get) => ({
    connectionState: "disconnected",
    id: "",
    displayName: "",
    frequency: "",
    clients: [],
    setConnectionState: (connectionState) => set({connectionState}),
    setClientInfo: (info) => set({id: info.id, displayName: info.displayName, frequency: info.frequency}),
    setClients: (clients) => set({clients: sortClients(clients)}),
    addClient: (client) => {
        const clients = get().clients.filter(c => c.id !== client.id);
        set({clients: sortClients([...clients, client])});
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