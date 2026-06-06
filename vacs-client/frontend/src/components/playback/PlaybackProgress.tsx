import {usePlaybackStore} from "../../stores/playback-store.ts";
import {ClipMeta, clipUnixMs} from "../../types/playback.ts";
import {toUTCTimeString} from "../../utils/date.ts";

type PlaybackProgressProps = {
    clip: ClipMeta | undefined;
};

function PlaybackProgress({clip}: PlaybackProgressProps) {
    const progress = usePlaybackStore(state => state.status?.progress ?? 0);

    const time = !clip
        ? "No playback"
        : toUTCTimeString(new Date(clipUnixMs(clip.startedAt) + progress * clip.durationMs));

    return (
        <>
            <p className="py-1 font-semibold">{time}</p>
            <div className="shrink-0 w-full h-4 border bg-gray-300">
                <div className="h-full bg-blue-700" style={{width: `${progress * 100}%`}} />
            </div>
        </>
    );
}

export default PlaybackProgress;
