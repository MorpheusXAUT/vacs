export type PlaybackListEntry = {
    type: "Rx" | "Tx" | "Ph";
    idk: boolean;
    time: string;
    target: string;
};
