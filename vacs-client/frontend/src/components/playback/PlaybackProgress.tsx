import {useMemo} from "preact/hooks";
import {ClipMeta, clipUnixMs} from "../../types/playback.ts";
import {toUTCTimeString} from "../../utils/date.ts";

type PlaybackProgressProps = {
    clip: ClipMeta | undefined;
    progress: number;
};

function PlaybackProgress({clip, progress}: PlaybackProgressProps) {
    const time = useMemo(() => {
        if (!clip) return "No playback";

        const start = clipUnixMs(clip.started_at);
        const end = clipUnixMs(clip.ended_at);

        const step = (end - start) / 100;
        const now = start + progress * step;

        return toUTCTimeString(new Date(now));
    }, [progress, clip]);

    return (
        <>
            <p className="py-1 font-semibold">{time}</p>
            <div className="shrink-0 w-full h-4 border bg-gray-300">
                <div className="h-full bg-blue-700" style={{width: `${progress}%`}} />
            </div>
        </>
    );
}

export default PlaybackProgress;
