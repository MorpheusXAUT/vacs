import {create} from "zustand/react";

type FilterState = {
    filter: string;
    setFilter: (filter: string) => void;
};

export const useFilterStore = create<FilterState>()(set => ({
    filter: "",
    setFilter: filter => set({filter}),
}));
