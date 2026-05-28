import Button, {ButtonColor} from "../ui/Button.tsx";
import {ClipMeta} from "../../types/replay.ts";
import {ComponentChildren} from "preact";
import {clsx} from "clsx";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import PlaybackProgress from "./PlaybackProgress.tsx";
import {useCallback, useEffect, useRef, useState} from "preact/hooks";
import {invokeSafe, invokeStrict} from "../../error.ts";
import speaker from "../../assets/speaker.svg";
import {StateSetter} from "../../types/generic.ts";

type PlaybackControlsProps = {
    clip: ClipMeta | undefined;
    prevClip: ClipMeta | undefined;
    nextClip: ClipMeta | undefined;
    setSelectedClip: StateSetter<number>;
    playing: boolean;
    playingRef: {current: boolean};
    setPlaying: StateSetter<boolean>;
};

function PlaybackControls(props: PlaybackControlsProps) {
    const {playing, playingRef, setPlaying} = props;
    const [playContinuously, setPlayContinuously] = useState(false);

    const [playbackDevice, setPlaybackDevice] = useState<"Output" | "Speaker">("Output");

    const clipIdRef = useRef(props.clip?.id);
    clipIdRef.current = props.clip?.id;

    const intendedClipChangeRef = useRef(false);

    const handlePlay = useAsyncDebounce(
        useCallback(async (id: number | undefined, deviceType: "Output" | "Speaker") => {
            try {
                await invokeStrict("replay_play", {
                    id,
                    deviceType,
                });
                setPlaying(true);
            } catch {}
        }, []),
    );

    const handleStop = useAsyncDebounce(
        useCallback(async (setState = true) => {
            try {
                await invokeStrict("replay_stop");
                if (setState) setPlaying(false);
                setPlayContinuously(false);
            } catch {}
        }, []),
    );

    useEffect(() => {
        if (props.clip !== undefined && playingRef.current && !intendedClipChangeRef.current) {
            void handleStop();
        }
        intendedClipChangeRef.current = false;
    }, [props.clip, handleStop]);

    useEffect(() => {
        return () => handleStop();
    }, [handleStop]);

    return (
        <>
            <PlaybackProgress
                clip={playing ? props.clip : undefined}
                stopPlaying={() => {
                    if (playContinuously && props.nextClip !== undefined) {
                        intendedClipChangeRef.current = true;
                        props.setSelectedClip(prev => prev - 1);
                        void handlePlay(props.nextClip?.id, playbackDevice);
                    } else {
                        setPlaying(false);
                        setPlayContinuously(false);
                    }
                }}
            />
            <div className="flex-1 min-h-0 w-full flex items-end justify-center mt-[0.625rem]">
                <div className="h-min w-min grid grid-flow-col grid-rows-2 gap-y-3 gap-x-2">
                    <PlaybackControlButton
                        color={playing && !playContinuously ? "blue" : "gray"}
                        disabled={props.clip === undefined || (playing && playContinuously)}
                        onClick={() => handlePlay(clipIdRef.current, playbackDevice)}
                        className={clsx(playing && !playContinuously && "text-white")}
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
                                const next = prev === "Output" ? "Speaker" : "Output";
                                if (playing) void handlePlay(clipIdRef.current, next);
                                return next;
                            });
                        }}
                    >
                        {playbackDevice === "Output" ? (
                            "H"
                        ) : (
                            <img src={speaker} alt="S" className="h-7" />
                        )}
                    </PlaybackControlButton>
                    <PlaybackControlButton
                        color={playing && playContinuously ? "blue" : "gray"}
                        disabled={
                            props.clip === undefined ||
                            (playing && !playContinuously) ||
                            (props.nextClip === undefined && !playContinuously)
                        }
                        onClick={() => {
                            setPlayContinuously(true);
                            void handlePlay(props.clip?.id, playbackDevice);
                        }}
                    >
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
                                className={
                                    playing && playContinuously
                                        ? "stroke-blue-700"
                                        : "stroke-gray-300"
                                }
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
                    <PlaybackControlButton
                        disabled={!playing}
                        onClick={() => {
                            void invokeSafe("replay_seek", {millis: -1000});
                        }}
                    >
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
                    <PlaybackControlButton
                        disabled={!playing}
                        onClick={() => {
                            void invokeSafe("replay_seek", {millis: 1000});
                        }}
                    >
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
