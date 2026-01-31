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

export const useProfileStationKeys = () => {
    return useProfileStore(
        useShallow(state => {
            if (state.profile?.tabbed !== undefined) {
                return state.profile.tabbed.flatMap(t =>
                    t.page.keys.filter(k => k.stationId !== undefined),
                );
            }
            if (state.profile?.geo !== undefined) {
                return geoPageContainerToKeys(state.profile.geo).filter(
                    k => k.stationId !== undefined,
                );
            }
            return [];
        }),
    );
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
