import {create} from "zustand/react";

type Page = "phone" | "radio";
export type Menu = "settings" | "mission" | "telephone" | "playback";

type SettingsSubmenu =
    | "settings-transmit"
    | "settings-hotkeys"
    | "settings-call"
    | "settings-advanced";
type Submenu = SettingsSubmenu;

type NavigationState = {
    page: Page;
    menu: Menu | undefined;
    submenu: Submenu | undefined;
    setPage: (page: Page) => void;
    goToPage: (page: Page) => void;
    openMenu: (menu: Menu) => void;
    closeMenu: () => void;
    openSettingsSubmenu: (submenu: SettingsSubmenu) => void;
};

export const useNavigationStore = create<NavigationState>()(set => ({
    page: "phone",
    menu: undefined,
    submenu: undefined,
    setPage: page => set({page}),
    goToPage: page => {
        set({page, menu: undefined, submenu: undefined});
    },
    openMenu: menu => set({menu, submenu: undefined}),
    closeMenu: () => set({menu: undefined, submenu: undefined}),
    openSettingsSubmenu: (submenu: SettingsSubmenu) => set({menu: "settings", submenu}),
}));

export const setPage = (page: Page) => useNavigationStore.getState().setPage(page);

export const goToPage = (page: Page) => useNavigationStore.getState().goToPage(page);

export const openMenu = (menu: Menu) => useNavigationStore.getState().openMenu(menu);

export const closeMenu = () => useNavigationStore.getState().closeMenu();

export const openSettingsSubmenu = (submenu: SettingsSubmenu) =>
    useNavigationStore.getState().openSettingsSubmenu(submenu);
