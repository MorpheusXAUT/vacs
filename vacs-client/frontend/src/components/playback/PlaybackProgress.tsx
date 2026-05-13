import {useEffect, useMemo, useState} from "preact/hooks";
import {listen} from "../../transport";
import {ClipMeta, clipUnixMs} from "../../types/replay.ts";

type PlaybackProgressProps = {
    clip: ClipMeta | undefined;
    setPlaying: (playing: boolean) => void;
};

function PlaybackProgress(props: PlaybackProgressProps) {
    const [progress, setProgress] = useState(0);

    useEffect(() => {
        const unlisten = listen<number>("replay:progress", event => {
            setProgress(event.payload * 100);
            if (event.payload === 1) {
                props.setPlaying(false);
                setProgress(0);
            }
        });

        return () => {
            unlisten.then(fn => fn());
        };
    }, []);

    const time = useMemo(() => {
        if (!props.clip) return "No playback";
        const start = clipUnixMs(props.clip.started_at);
        const end = clipUnixMs(props.clip.ended_at);

        const step = (end - start) / 100;

        const now = start + progress * step;

        return new Date(now).toLocaleTimeString();
    }, [progress, props.clip]);

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
