import Button, {ButtonColor} from "../ui/Button.tsx";
import {ClipMeta} from "../../types/replay.ts";
import {ComponentChildren} from "preact";
import {clsx} from "clsx";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import PlaybackProgress from "./PlaybackProgress.tsx";
import {useCallback, useEffect, useRef, useState} from "preact/hooks";
import {invokeStrict} from "../../error.ts";
import speaker from "../../assets/speaker.svg";
import {StateSetter} from "../../types/generic.ts";

type PlaybackControlsProps = {
    clip: ClipMeta | undefined;
    prevClip: ClipMeta | undefined;
    nextClip: ClipMeta | undefined;
    setSelectedClip: StateSetter<number>;
};

function PlaybackControls(props: PlaybackControlsProps) {
    const disabled = props.clip === undefined;
    const [playing, setPlaying] = useState(false);
    const playingRef = useRef(playing);
    const [playbackDevice, setPlaybackDevice] = useState<"Headset" | "Speaker">("Headset");

    const clipIdRef = useRef(props.clip?.id);
    clipIdRef.current = props.clip?.id;

    const intendedClipChangeRef = useRef(false);

    const handlePlay = useAsyncDebounce(
        useCallback(
            async (id: number | undefined, device: "Headset" | "Speaker") => {
                try {
                    await invokeStrict("replay_play", {
                        id,
                        device,
                    });
                    setPlaying(true);
                } catch {}
            },
            [playbackDevice],
        ),
    );

    const handleStop = useAsyncDebounce(
        useCallback(async (setState = true) => {
            try {
                await invokeStrict("replay_stop");
                if (setState) setPlaying(false);
            } catch {}
        }, []),
    );

    // TODO: Check with RL behaviour. Does playback stop when selected clip changes?
    // TODO: Stop when playback overlay is closed?
    useEffect(() => {
        if (props.clip !== undefined && playingRef.current && !intendedClipChangeRef.current) {
            void handleStop();
        }
        intendedClipChangeRef.current = false;
    }, [props.clip, handleStop]);

    useEffect(() => {
        playingRef.current = playing;
    }, [playing]);

    return (
        <>
            <PlaybackProgress clip={playing ? props.clip : undefined} setPlaying={setPlaying} />
            <div className="flex-1 min-h-0 w-full flex items-end justify-center">
                <div className="h-min w-min grid grid-flow-col grid-rows-2 gap-y-3 gap-x-2">
                    <PlaybackControlButton
                        color={playing ? "blue" : "gray"}
                        disabled={disabled}
                        onClick={() => handlePlay(clipIdRef.current, playbackDevice)}
                        className={clsx(playing && "text-white")}
                    >
                        <svg
                            width="32"
                            height="32"
                            viewBox="0 0 74 74"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path d="M0 37V0L74 37L0 74V37Z" fill="currentColor" />
                        </svg>
                    </PlaybackControlButton>
                    <PlaybackControlButton
                        onClick={() => {
                            setPlaybackDevice(prev => {
                                const next = prev === "Headset" ? "Speaker" : "Headset";
                                if (playing) void handlePlay(clipIdRef.current, next);
                                return next;
                            });
                        }}
                    >
                        {playbackDevice === "Headset" ? (
                            "H"
                        ) : (
                            <img src={speaker} alt="S" className="h-7" />
                        )}
                    </PlaybackControlButton>
                    <PlaybackControlButton disabled={playing || props.nextClip === undefined}>
                        <svg
                            height="40"
                            viewBox="0 0 96 110"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path d="M0 37V0L74 37L0 74V37Z" fill="currentColor" />
                            <path
                                d="M95.8945 68.2109L99.4717 70L95.8945 71.7891L19 110.236V29.7637L95.8945 68.2109Z"
                                fill="currentColor"
                                className="stroke-gray-300"
                                stroke-width="4"
                            />
                        </svg>
                    </PlaybackControlButton>
                    <PlaybackControlButton disabled={!playing} onClick={handleStop}>
                        <div
                            className={clsx(
                                "h-8 aspect-square",
                                playing ? "bg-black" : "bg-gray-600",
                            )}
                        ></div>
                    </PlaybackControlButton>
                    <PlaybackControlButton
                        disabled={!playing || props.prevClip === undefined}
                        onClick={async () => {
                            await handleStop(false);
                            intendedClipChangeRef.current = true;
                            props.setSelectedClip(prev => prev + 1);
                            void handlePlay(props.prevClip?.id, playbackDevice);
                        }}
                    >
                        <svg
                            height="32"
                            viewBox="0 0 48 74"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path d="M48 0V74L11 37V74H0V0H11V37L48 0Z" fill="currentColor" />
                        </svg>
                    </PlaybackControlButton>
                    <PlaybackControlButton disabled={!playing}>
                        <svg
                            width="32"
                            height="32"
                            viewBox="0 0 74 74"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path d="M74 0V74L37 37V74L0 37L37 0V37L74 0Z" fill="currentColor" />
                        </svg>
                    </PlaybackControlButton>
                    <PlaybackControlButton
                        disabled={!playing || props.nextClip === undefined}
                        onClick={async () => {
                            await handleStop(false);
                            intendedClipChangeRef.current = true;
                            props.setSelectedClip(prev => prev - 1);
                            void handlePlay(props.nextClip?.id, playbackDevice);
                        }}
                    >
                        <svg
                            transform="rotate(180)"
                            height="32"
                            viewBox="0 0 48 74"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path d="M48 0V74L11 37V74H0V0H11V37L48 0Z" fill="currentColor" />
                        </svg>
                    </PlaybackControlButton>
                    <PlaybackControlButton disabled={!playing}>
                        <svg
                            transform="rotate(180)"
                            width="32"
                            height="32"
                            viewBox="0 0 74 74"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path d="M74 0V74L37 37V74L0 37L37 0V37L74 0Z" fill="currentColor" />
                        </svg>
                    </PlaybackControlButton>
                </div>
            </div>
        </>
    );
}

type PlaybackControlButtonProps = {
    color?: ButtonColor;
    className?: string;
    disabled?: boolean;
    onClick?: () => void;
    children?: ComponentChildren;
};

function PlaybackControlButton(props: PlaybackControlButtonProps) {
    return (
        <Button
            color={props.color ?? "gray"}
            className={clsx(
                "h-17 flex items-center justify-center",
                props.disabled && "text-gray-600",
                props.className,
            )}
            disabled={props.disabled}
            onClick={props.onClick}
        >
            {props.children}
        </Button>
    );
}

export default PlaybackControls;
