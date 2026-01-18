import {DirectAccessPage, Profile} from "../types/profile.ts";
import {create} from "zustand/react";
import {ProfileId} from "../types/generic.ts";

type ProfileState = {
    profile: Profile | undefined;
    page: DirectAccessPage | undefined;
    setProfile: (profile: Profile | undefined) => void;
    setPage: (page: DirectAccessPage | undefined) => void;
};

export const useProfileStore = create<ProfileState>()(set => ({
    profile: undefined,
    page: undefined,
    setProfile: profile => set({profile}),
    setPage: page => set({page: page}),
}));

export const useProfileType = (): "geo" | "tabbed" | "unknown" | undefined => {
    return useProfileStore(state => {
        if (state.profile === undefined) return undefined;
        if (state.profile.geo !== undefined) return "geo";
        if (state.profile.tabbed !== undefined) return "tabbed";
        return "unknown";
    });
};

export const INVALID_PROFILE: Profile = {
    id: "INVALID" as ProfileId,
};

export const TABBED_PROFILE: Profile = {
    id: "TABBED" as ProfileId,
    tabbed: {
        "DA 1": {} as DirectAccessPage,
        "DA 2": {} as DirectAccessPage,
    },
};
