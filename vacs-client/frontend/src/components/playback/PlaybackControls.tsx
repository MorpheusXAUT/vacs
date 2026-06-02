import {clsx} from "clsx";
import {ComponentChildren} from "preact";
import speaker from "../../assets/speaker.svg";
import {PlaybackDevice, PlaybackStatus} from "../../stores/playback-store.ts";
import {ClipMeta} from "../../types/playback.ts";
import Button, {ButtonColor} from "../ui/Button.tsx";
import {useBlinkStore} from "../../stores/blink-store.ts";

type PlaybackControlsProps = {
    selectedClip: ClipMeta | undefined;
    prevClip: ClipMeta | undefined;
    nextClip: ClipMeta | undefined;
    status: PlaybackStatus | undefined;
    playbackDevice: PlaybackDevice;
    active: boolean;
    onPlayPause: (continuously: boolean) => void;
    onStop: () => void;
    onSeekBack: () => void;
    onSeekForward: () => void;
    onDeviceSwitch: () => void;
    onPrev: () => void;
    onNext: () => void;
};

export function PlaybackControls({
    selectedClip,
    prevClip,
    nextClip,
    status,
    playbackDevice,
    active,
    onPlayPause,
    onStop,
    onSeekBack,
    onSeekForward,
    onDeviceSwitch,
    onPrev,
    onNext,
}: PlaybackControlsProps) {
    const playSingle = status?.continuously === false;
    const isPlaying = status?.status === "playing";

    const blink = useBlinkStore(state => state.blink);

    return (
        <div className="h-min w-min grid grid-flow-col grid-rows-2 gap-y-3 gap-x-2">
            <PlaybackControlButton
                color={playSingle && (isPlaying || blink) ? "blue" : "gray"}
                disabled={selectedClip === undefined || status?.continuously}
                onClick={() => onPlayPause(false)}
                className={clsx(playSingle && (isPlaying || blink) && "text-white")}
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
            <PlaybackControlButton onClick={onDeviceSwitch}>
                {playbackDevice === "Output" ? "H" : <img src={speaker} alt="S" className="h-7" />}
            </PlaybackControlButton>
            <PlaybackControlButton
                color={status?.continuously && (isPlaying || blink) ? "blue" : "gray"}
                disabled={
                    selectedClip === undefined ||
                    (status !== undefined && !status.continuously) ||
                    (nextClip === undefined && status === undefined)
                }
                onClick={() => onPlayPause(true)}
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
                            status?.continuously && (isPlaying || blink)
                                ? "stroke-blue-700"
                                : "stroke-gray-300"
                        }
                        stroke-width="4"
                    />
                </svg>
            </PlaybackControlButton>
            <PlaybackControlButton disabled={!active} onClick={onStop}>
                <div className={clsx("h-8 aspect-square", active ? "bg-black" : "bg-gray-600")} />
            </PlaybackControlButton>
            <PlaybackControlButton disabled={!active || prevClip === undefined} onClick={onPrev}>
                <svg height="32" viewBox="0 0 48 74" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path d="M48 0V74L11 37V74H0V0H11V37L48 0Z" fill="currentColor" />
                </svg>
            </PlaybackControlButton>
            <PlaybackControlButton disabled={!active} onClick={onSeekBack}>
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
            <PlaybackControlButton disabled={!active || nextClip === undefined} onClick={onNext}>
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
            <PlaybackControlButton disabled={!active} onClick={onSeekForward}>
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
    );
}

type PlaybackControlButtonProps = {
    color?: ButtonColor;
    className?: string;
    disabled?: boolean;
    onClick?: () => void;
    children?: ComponentChildren;
};

function PlaybackControlButton({
    color,
    className,
    disabled,
    onClick,
    children,
}: PlaybackControlButtonProps) {
    return (
        <Button
            color={color ?? "gray"}
            className={clsx(
                "h-17 flex items-center justify-center",
                disabled && "text-gray-600",
                className,
            )}
            disabled={disabled}
            onClick={onClick}
        >
            {children}
        </Button>
    );
}
