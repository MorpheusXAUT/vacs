import {create} from "zustand/react";

type AuthStatus = "loading" | "authenticated" | "unauthenticated";

type AuthState = {
    cid: string;
    status: AuthStatus;
    setAuthenticated: (cid: string) => void;
    setUnauthenticated: () => void;
};

export const useAuthStore = create<AuthState>()(set => ({
    cid: "",
    status: "loading",
    setAuthenticated: cid => set({cid, status: "authenticated"}),
    setUnauthenticated: () => set({cid: "", status: "unauthenticated"}),
}));
