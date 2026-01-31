import {ClientInfo} from "../types/client-info.ts";
import {create} from "zustand/react";

type ClientsState = {
    clients: ClientInfo[];
    setClients: (clients: ClientInfo[]) => void;
    addClient: (client: ClientInfo) => void;
    getClientInfo: (cid: string) => ClientInfo;
    removeClient: (cid: string) => void;
};

export const useClientsStore = create<ClientsState>()((set, get) => ({
    clients: [],
    setClients: clients => set({clients}),
    addClient: client => set({clients: [...get().clients, client]}),
    getClientInfo: cid => {
        const client = get().clients.find(c => c.id === cid);
        if (client === undefined) {
            return {
                id: cid,
                displayName: cid,
                positionId: undefined,
                alias: undefined,
                frequency: "",
            };
        }
        return client;
    },
    removeClient: cid => {
        set({
            clients: get().clients.filter(client => client.id !== cid),
        });
    },
}));
