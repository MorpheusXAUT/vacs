import Button, {ButtonColor} from "../ui/Button.tsx";
import {ClipMeta} from "../../types/replay.ts";
import {ComponentChildren} from "preact";
import {clsx} from "clsx";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import PlaybackProgress from "./PlaybackProgress.tsx";
import {useCallback, useEffect, useState} from "preact/hooks";
import {invokeStrict} from "../../error.ts";

type PlaybackControlsProps = {
    clip: ClipMeta | undefined;
};

function PlaybackControls(props: PlaybackControlsProps) {
    const disabled = props.clip === undefined;
    const [playing, setPlaying] = useState(false);

    const handlePlay = useAsyncDebounce(async () => {
        try {
            void invokeStrict("replay_play", {id: props.clip?.id});
            setPlaying(true);
        } catch {}
    });

    const handleStop = useAsyncDebounce(
        useCallback(async () => {
            try {
                void invokeStrict("replay_stop");
                setPlaying(false);
            } catch {}
        }, [setPlaying]),
    );

    // TODO: Check with RL behaviour. Does playback stop when selected clip changes?
    // TODO: Stop when playback overlay is closed?
    useEffect(() => {
        if (props.clip !== undefined) {
            void handleStop();
        }
    }, [props.clip, handleStop]);

    return (
        <>
            <PlaybackProgress clip={playing ? props.clip : undefined} setPlaying={setPlaying} />
            <div className="flex-1 min-h-0 w-full flex items-end justify-center">
                <div className="h-min w-min grid grid-flow-col grid-rows-2 gap-y-3 gap-x-2">
                    <PlaybackControlButton
                        color={playing ? "blue" : "gray"}
                        disabled={disabled}
                        onClick={handlePlay}
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
                    <PlaybackControlButton>H/S</PlaybackControlButton>
                    <PlaybackControlButton disabled={true}>
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
                    <PlaybackControlButton disabled={true}>
                        <svg
                            height="32"
                            viewBox="0 0 48 74"
                            fill="none"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <path d="M48 0V74L11 37V74H0V0H11V37L48 0Z" fill="currentColor" />
                        </svg>
                    </PlaybackControlButton>
                    <PlaybackControlButton disabled={true}>
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
                    <PlaybackControlButton disabled={true}>
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
                    <PlaybackControlButton disabled={true}>
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
