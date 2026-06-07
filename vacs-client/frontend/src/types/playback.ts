export type TapId = {frequency: number} | "headset" | "speaker" | "merged";

// SystemTime serializes as `{ secs_since_epoch, nanos_since_epoch }`.
export type ClipMeta = {
    id: number;
    path: string;
    callsigns: string[];
    frequency: number | null;
    startedAt: {secs_since_epoch: number; nanos_since_epoch: number};
    endedAt: {secs_since_epoch: number; nanos_since_epoch: number};
    durationMs: number;
};

export function clipUnixMs(t: ClipMeta["startedAt"]): number {
    return t.secs_since_epoch * 1000 + Math.floor(t.nanos_since_epoch / 1_000_000);
}

export function sortClips(list: ClipMeta[]): ClipMeta[] {
    return [...list].sort((a, b) => clipUnixMs(b.startedAt) - clipUnixMs(a.startedAt));
}
