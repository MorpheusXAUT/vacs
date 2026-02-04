import {create} from "zustand/react";
import {ClientId} from "../types/generic.ts";

type AuthStatus = "loading" | "authenticated" | "unauthenticated";

type AuthState = {
    cid: ClientId | undefined;
    status: AuthStatus;
    setAuthenticated: (cid: ClientId) => void;
    setUnauthenticated: () => void;
};

export const useAuthStore = create<AuthState>()(set => ({
    cid: undefined,
    status: "loading",
    setAuthenticated: cid => set({cid, status: "authenticated"}),
    setUnauthenticated: () => set({cid: undefined, status: "unauthenticated"}),
}));
