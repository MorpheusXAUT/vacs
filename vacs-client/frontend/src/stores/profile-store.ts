import {
    DirectAccessKey,
    DirectAccessPage,
    GeoPageContainer,
    isGeoPageButton,
    isGeoPageContainer,
    Profile,
} from "../types/profile.ts";
import {create} from "zustand/react";
import {useShallow} from "zustand/react/shallow";

type ProfileState = {
    profile: Profile | undefined;
    page: DirectAccessPage | undefined;
    testProfilePath: string | undefined;
    setProfile: (profile: Profile | undefined) => void;
    setPage: (page: DirectAccessPage | undefined) => void;
    setTestProfilePath: (path: string | undefined) => void;
    reset: () => void;
};

export const useProfileStore = create<ProfileState>()((set, _, store) => ({
    profile: undefined,
    page: undefined,
    testProfilePath: undefined,
    setProfile: profile => set({profile}),
    setPage: page => set({page: page}),
    setTestProfilePath: path => set({testProfilePath: path}),
    reset: () => set(store.getInitialState()),
}));

export const useProfileType = (): "geo" | "tabbed" | "unknown" | undefined => {
    return useProfileStore(state => {
        if (state.profile === undefined) return undefined;
        if (state.profile.geo !== undefined) return "geo";
        if (state.profile.tabbed !== undefined) return "tabbed";
        return "unknown";
    });
};

const profileToStationKeys = (profile: Profile | undefined): DirectAccessKey[] => {
    if (profile?.tabbed !== undefined) {
        return profile.tabbed.flatMap(t => t.page.keys.filter(k => k.stationId !== undefined));
    }
    if (profile?.geo !== undefined) {
        return geoPageContainerToKeys(profile.geo).filter(k => k.stationId !== undefined);
    }
    return [];
};

export const getProfileStationKeysState = () => {
    return profileToStationKeys(useProfileStore.getState().profile);
};

export const useProfileStationKeys = () => {
    return useProfileStore(useShallow(state => profileToStationKeys(state.profile)));
};

const geoPageContainerToKeys = (container: GeoPageContainer): DirectAccessKey[] => {
    return container.children.flatMap(c => {
        if (isGeoPageContainer(c)) {
            return geoPageContainerToKeys(c);
        } else if (isGeoPageButton(c)) {
            return c.page?.keys ?? [];
        }
        return [];
    });
};
