import {DirectAccessPageDTO} from "../types/profile.ts";
import {create} from "zustand/react";

type ProfileState = {
    selectedPage: DirectAccessPageDTO | undefined;
    setSelectedPage: (page: DirectAccessPageDTO | undefined) => void;
};

export const useProfileStore = create<ProfileState>()(set => ({
    selectedPage: undefined,
    setSelectedPage: page => set({selectedPage: page}),
}));
