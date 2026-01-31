import {create} from "zustand/react";
import {ClientId} from "../types/generic.ts";

type AuthStatus = "loading" | "authenticated" | "unauthenticated";

type AuthState = {
    cid: ClientId;
    status: AuthStatus;
    setAuthenticated: (cid: ClientId) => void;
    setUnauthenticated: () => void;
};

export const useAuthStore = create<AuthState>()(set => ({
    cid: "" as ClientId, // TODO: Maybe undefined?
    status: "loading",
    setAuthenticated: cid => set({cid, status: "authenticated"}),
    setUnauthenticated: () => set({cid: "" as ClientId, status: "unauthenticated"}),
}));
