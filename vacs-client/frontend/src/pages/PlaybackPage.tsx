import {clsx} from "clsx";
import {useCallback, useEffect, useRef, useState} from "preact/hooks";
import PlaybackActions from "../components/playback/PlaybackActions.tsx";
import PlaybackList from "../components/playback/PlaybackList.tsx";
import Button, {ButtonColor} from "../components/ui/Button.tsx";
import {invokeSafe, invokeStrict} from "../error.ts";
import {useCapabilitiesStore} from "../stores/capabilities-store.ts";
import {openSettingsSubmenu} from "../stores/navigation-store.ts";
import {useSettingsStore} from "../stores/settings-store.ts";
import {EventCallback, listen, UnlistenFn} from "../transport";
import {ClipMeta, sortClips} from "../types/playback.ts";
import {CloseButton} from "./SettingsPage.tsx";
import PlaybackProgress from "../components/playback/PlaybackProgress.tsx";
import speaker from "../assets/speaker.svg";
import {ComponentChildren} from "preact";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {useEventCallback} from "../hooks/event-callback-hook.ts";
import {shouldStopBlinking, useBlinkStore} from "../stores/blink-store.ts";
import {useCallStore} from "../stores/call-store.ts";
import {useRadioStore} from "../stores/radio-store.ts";
import {isPlaybackRoot, PlaybackDevice, usePlaybackStore} from "../stores/playback-store.ts";
import {instanceId} from "../transport/store-sync.ts";

function PlaybackPage() {
    const capPlayback = useCapabilitiesStore(state => state.playback);
    const capPlatform = useCapabilitiesStore(state => state.platform);

    const playbackEnabled = useSettingsStore(state => state.playbackEnabled);

    return (
        <div
            className={clsx(
                "z-10 absolute h-[calc(100%+3px)] w-[44rem] -top-px right-[-2px]",
                "bg-blue-700 px-2 pb-2 flex flex-col rounded-md",
            )}
        >
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Playback</p>
            {capPlayback && playbackEnabled ? (
                <PlaybackPageInner />
            ) : !playbackEnabled ? (
                <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex flex-col justify-center items-center text-slate-600">
                    <p>Radio playback is not enabled.</p>
                    <p>
                        Enable it in the{" "}
                        <span
                            className="text-blue-700 cursor-pointer"
                            onClick={() => {
                                void invokeSafe("audio_play_ui_click");
                                openSettingsSubmenu("settings-advanced");
                            }}
                        >
                            advanced settings
                        </span>
                        .
                    </p>
                </div>
            ) : (
                <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex justify-center items-center text-slate-600">
                    Radio playback is not yet supported on {capPlatform}.
                </div>
            )}
        </div>
    );
}

function PlaybackPageInner() {
    const [clips, setClips] = useState<ClipMeta[]>([]);

    const selected = usePlaybackStore(state => state.selected);
    const status = usePlaybackStore(state => state.status);
    const playbackDevice = usePlaybackStore(state => state.playbackDevice);
    const {setSelected, setStatus, setPlaybackDevice} = usePlaybackStore(state => state.actions);

    const intendedClipChangeRef = useRef(false);

    const active = status !== undefined;
    const selectedClip: ClipMeta | undefined = clips[selected];
    const prevClip: ClipMeta | undefined = clips[selected + 1];
    const nextClip: ClipMeta | undefined = clips[selected - 1];

    const {blink, startBlink, stopBlink} = useBlinkStore(state => state);

    const handlePlay = useAsyncDebounce(
        useCallback(
            async (id: number, deviceType: PlaybackDevice, continuously = false) => {
                try {
                    await invokeStrict("playback_play", {
                        id,
                        deviceType,
                    });
                    setStatus({id, status: "playing", continuously, progress: 0});
                } catch {}
            },
            [setStatus],
        ),
    );

    const handlePause = useAsyncDebounce(
        useCallback(async () => {
            try {
                await invokeStrict("playback_pause");
                setStatus(prev => {
                    if (prev === undefined) return prev;
                    return {
                        ...prev,
                        status: "paused",
                    };
                });
                startBlink();
            } catch {}
        }, [setStatus, startBlink]),
    );

    const handleContinue = useAsyncDebounce(
        useCallback(async () => {
            try {
                await invokeStrict("playback_continue");
                setStatus(prev => {
                    if (prev === undefined) return prev;
                    return {
                        ...prev,
                        status: "playing",
                    };
                });
                if (
                    shouldStopBlinking(
                        useCallStore.getState().incomingCalls.length,
                        useCallStore.getState().callDisplay,
                        useRadioStore.getState().cpl,
                        false,
                    )
                ) {
                    stopBlink();
                }
            } catch {}
        }, [setStatus, stopBlink]),
    );

    const handleStop = useAsyncDebounce(
        useCallback(
            async (setState = true) => {
                try {
                    await invokeStrict("playback_stop");
                    if (setState) setStatus(undefined);
                } catch {}
            },
            [setStatus],
        ),
    );

    useEffect(() => {
        if (!isPlaybackRoot()) return;
        const status = usePlaybackStore.getState().status;
        if (
            selectedClip !== undefined &&
            status !== undefined &&
            status.id !== selectedClip.id &&
            !intendedClipChangeRef.current
        ) {
            void handleStop();
        }
        intendedClipChangeRef.current = false;
    }, [selectedClip, handleStop]);

    const handleProgressUpdate: EventCallback<number> = useEventCallback(event => {
        setStatus(prev => {
            if (prev === undefined) return prev;
            return {
                ...prev,
                progress: event.payload * 100,
            };
        });
        if (event.payload === 1 && isPlaybackRoot()) {
            if (status?.continuously && nextClip !== undefined) {
                intendedClipChangeRef.current = true;
                setSelected(prev => prev - 1);
                void handlePlay(nextClip?.id, playbackDevice, true);
            } else {
                setStatus(undefined);
            }
        }
    });

    useEffect(() => {
        const unlisten = listen<number>("playback:progress", handleProgressUpdate);

        return () => unlisten.then(fn => fn());
    }, [handleProgressUpdate]);

    useEffect(() => {
        usePlaybackStore.getState().actions.setOpenInstanceIds(prev => [...prev, instanceId]);

        const fetch = async () => {
            const list = await invokeSafe<ClipMeta[]>("playback_list");
            if (list === undefined) return;
            setClips(sortClips(list));
        };
        void fetch();

        const unlistenFns: Promise<UnlistenFn>[] = [];
        unlistenFns.push(
            listen<{recorded: ClipMeta; evicted: ClipMeta[]}>("playback:clips-modified", event => {
                if (!isPlaybackRoot()) return;

                let status = usePlaybackStore.getState().status;
                setClips(prev => {
                    let playingEvicted = false;
                    for (const evictedClip of event.payload.evicted) {
                        prev = prev.filter(clip => clip.id !== evictedClip.id);
                        if (status?.id === evictedClip.id) {
                            void handleStop();
                            playingEvicted = true;
                        }
                    }

                    if (prev.length > 0 && status !== undefined && !playingEvicted) {
                        setSelected(prev => prev + 1);
                    }
                    return sortClips([...prev, event.payload.recorded]);
                });
            }),
        );

        return () => {
            usePlaybackStore.getState().actions.setOpenInstanceIds(prev => {
                const next = prev.filter(id => id !== instanceId);
                if (next.length === 0) void handleStop();
                return next;
            });
            unlistenFns.forEach(fn => fn.then(f => f()));
        };
    }, [handleStop, setSelected]);

    return (
        <div className="w-full grow rounded-b-sm bg-[#B5BBC6] grid grid-cols-[6.5rem_auto] p-2 gap-2 overflow-auto">
            <div className="h-full w-full flex flex-col justify-between items-center">
                <div className="w-full flex flex-col items-center bg-gray-300 border rounded-md">
                    <p className="w-full border-b text-center font-semibold">Filter</p>
                    <Button color="gray" className="h-15 my-2 uppercase">
                        <p>
                            Speech <br /> Only
                        </p>
                    </Button>
                    <Button color="blue" className="h-15 mt-2 uppercase rounded-b-none!">
                        Radio
                    </Button>
                    <Button color="gray" className="h-15 mb-2 uppercase rounded-t-none!">
                        Phone
                    </Button>
                </div>
                <PlaybackActions
                    clips={clips}
                    selected={selected}
                    setClips={setClips}
                    deleteDisabled={status !== undefined}
                />
            </div>
            <div className="h-full w-full flex flex-col p-px">
                <PlaybackList clips={clips} selected={selected} setSelected={setSelected} />
                <div className="relative w-full h-full flex flex-col items-center pr-16">
                    <PlaybackProgress
                        clip={active ? clips[selected] : undefined}
                        progress={status?.progress ?? 0}
                    />
                    <div className="flex-1 min-h-0 w-full flex items-end justify-center mt-[0.625rem]">
                        <div className="h-min w-min grid grid-flow-col grid-rows-2 gap-y-3 gap-x-2">
                            <PlaybackControlButton
                                color={
                                    status?.continuously === false &&
                                    (status?.status === "playing" || blink)
                                        ? "blue"
                                        : "gray"
                                }
                                disabled={selectedClip === undefined || status?.continuously}
                                onClick={async () => {
                                    if (status === undefined) {
                                        void handlePlay(selectedClip.id, playbackDevice);
                                    } else if (status.status === "playing") {
                                        await handlePause();
                                    } else {
                                        await handleContinue();
                                    }
                                }}
                                className={clsx(
                                    status?.continuously === false &&
                                        (status?.status === "playing" || blink) &&
                                        "text-white",
                                )}
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
                                        if (active) void handlePlay(selectedClip.id, next); // TODO stop instead of play
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
                                color={status?.continuously ? "blue" : "gray"}
                                disabled={
                                    selectedClip === undefined ||
                                    status?.continuously === false ||
                                    (nextClip === undefined && status?.continuously)
                                }
                                onClick={() => {
                                    void handlePlay(selectedClip?.id, playbackDevice, true);
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
                                            status?.continuously
                                                ? "stroke-blue-700"
                                                : "stroke-gray-300"
                                        }
                                        stroke-width="4"
                                    />
                                </svg>
                            </PlaybackControlButton>
                            <PlaybackControlButton disabled={!active} onClick={handleStop}>
                                <div
                                    className={clsx(
                                        "h-8 aspect-square",
                                        active ? "bg-black" : "bg-gray-600",
                                    )}
                                ></div>
                            </PlaybackControlButton>
                            <PlaybackControlButton
                                disabled={!active || prevClip === undefined}
                                onClick={async () => {
                                    await handleStop(false);
                                    intendedClipChangeRef.current = true;
                                    setSelected(prev => prev + 1);
                                    void handlePlay(prevClip?.id, playbackDevice);
                                }}
                            >
                                <svg
                                    height="32"
                                    viewBox="0 0 48 74"
                                    fill="none"
                                    xmlns="http://www.w3.org/2000/svg"
                                >
                                    <path
                                        d="M48 0V74L11 37V74H0V0H11V37L48 0Z"
                                        fill="currentColor"
                                    />
                                </svg>
                            </PlaybackControlButton>
                            <PlaybackControlButton
                                disabled={!active}
                                onClick={() => {
                                    void invokeSafe("playback_seek", {millis: -1000});
                                }}
                            >
                                <svg
                                    width="32"
                                    height="32"
                                    viewBox="0 0 74 74"
                                    fill="none"
                                    xmlns="http://www.w3.org/2000/svg"
                                >
                                    <path
                                        d="M74 0V74L37 37V74L0 37L37 0V37L74 0Z"
                                        fill="currentColor"
                                    />
                                </svg>
                            </PlaybackControlButton>
                            <PlaybackControlButton
                                disabled={!active || nextClip === undefined}
                                onClick={async () => {
                                    await handleStop(false);
                                    intendedClipChangeRef.current = true;
                                    setSelected(prev => prev - 1);
                                    void handlePlay(nextClip?.id, playbackDevice);
                                }}
                            >
                                <svg
                                    transform="rotate(180)"
                                    height="32"
                                    viewBox="0 0 48 74"
                                    fill="none"
                                    xmlns="http://www.w3.org/2000/svg"
                                >
                                    <path
                                        d="M48 0V74L11 37V74H0V0H11V37L48 0Z"
                                        fill="currentColor"
                                    />
                                </svg>
                            </PlaybackControlButton>
                            <PlaybackControlButton
                                disabled={!active}
                                onClick={() => {
                                    void invokeSafe("playback_seek", {millis: 1000});
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
                                    <path
                                        d="M74 0V74L37 37V74L0 37L37 0V37L74 0Z"
                                        fill="currentColor"
                                    />
                                </svg>
                            </PlaybackControlButton>
                        </div>
                    </div>
                    <CloseButton className="h-17 w-19! absolute bottom-0 right-0" />
                </div>
            </div>
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

export default PlaybackPage;
