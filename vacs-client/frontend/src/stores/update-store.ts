import {create} from "zustand/react";

type UpdateState = {
    overlayVisible: boolean;
    mandatoryDialogVisible: boolean;
    downloadDialogVisible: boolean;
    currentVersion: string;
    newVersion?: string;
    actions: {
        setVersions: (currentVersion: string, newVersion?: string) => void;
        openMandatoryDialog: () => void;
        openDownloadDialog: () => void;
        closeOverlay: () => void;
    };
};

export const useUpdateStore = create<UpdateState>()(set => ({
    overlayVisible: true,
    mandatoryDialogVisible: false,
    downloadDialogVisible: false,
    currentVersion: "",
    newVersion: undefined,
    actions: {
        setVersions: (currentVersion: string, newVersion?: string) => {
            set({currentVersion, newVersion});
        },
        openMandatoryDialog: () => {
            set({overlayVisible: true, mandatoryDialogVisible: true, downloadDialogVisible: false});
        },
        openDownloadDialog: () => {
            set({overlayVisible: true, downloadDialogVisible: true, mandatoryDialogVisible: false});
        },
        closeOverlay: () => {
            set({overlayVisible: false, mandatoryDialogVisible: false});
        },
    },
}));
