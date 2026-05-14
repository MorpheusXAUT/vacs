import {useEffect, useMemo, useState} from "preact/hooks";
import {listen} from "../../transport";
import {ClipMeta, clipUnixMs} from "../../types/replay.ts";
import {toUTCTimeString} from "../../utils/date.ts";

type PlaybackProgressProps = {
    clip: ClipMeta | undefined;
    setPlaying: (playing: boolean) => void;
};

function PlaybackProgress({clip, setPlaying}: PlaybackProgressProps) {
    const [progress, setProgress] = useState(0);

    useEffect(() => {
        const unlisten = listen<number>("replay:progress", event => {
            setProgress(event.payload * 100);
            if (event.payload === 1) {
                setPlaying(false);
                setProgress(0);
            }
        });

        return () => unlisten.then(fn => fn());
    }, [setPlaying]);

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
