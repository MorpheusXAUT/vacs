import {useCallback, useEffect, useRef} from "preact/hooks";
import {useAsyncDebounce} from "./debounce-hook.ts";
import {useEventCallback} from "./event-callback-hook.ts";
import {invokeStrict} from "../error.ts";
import {shouldStopBlinking, useBlinkStore} from "../stores/blink-store.ts";
import {useCallStore} from "../stores/call-store.ts";
import {useRadioStore} from "../stores/radio-store.ts";
import {isPlaybackRoot, PlaybackDevice, usePlaybackStore} from "../stores/playback-store.ts";
import {EventCallback, listen} from "../transport";
import {ClipMeta} from "../types/playback.ts";

type Params = {
    selectedClip: ClipMeta | undefined;
    prevClip: ClipMeta | undefined;
    nextClip: ClipMeta | undefined;
};

export function usePlaybackControls({selectedClip, prevClip, nextClip}: Params) {
    const status = usePlaybackStore(state => state.status);
    const playbackDevice = usePlaybackStore(state => state.playbackDevice);
    const {setSelected, setStatus, setPlaybackDevice} = usePlaybackStore(state => state.actions);
    const {blink, startBlink, stopBlink} = useBlinkStore(state => state);

    const intendedClipChangeRef = useRef(false);

    const handleStart = useAsyncDebounce(
        async (id: number, deviceType: PlaybackDevice, continuously: boolean = false) => {
            try {
                await invokeStrict("playback_start", {id, deviceType});
                setStatus({id, status: "playing", continuously, progress: 0});
            } catch {}
        },
    );

    const handlePause = useAsyncDebounce(async () => {
        try {
            await invokeStrict("playback_pause");
            setStatus(prev => {
                if (prev === undefined) return prev;
                return {...prev, status: "paused"};
            });
            startBlink();
        } catch {}
    });

    const handleContinue = useAsyncDebounce(async () => {
        try {
            await invokeStrict("playback_continue");
            setStatus(prev => {
                if (prev === undefined) return prev;
                return {...prev, status: "playing"};
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
    });

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

    const handleSeek = useAsyncDebounce(async (durationSecs: number = 1) => {
        const millis = 1000 * durationSecs;
        try {
            await invokeStrict("playback_seek", {millis});
            setStatus(prev => {
                if (prev === undefined || prev.status === "playing") return prev;
                const diff = millis / selectedClip!.durationMs;
                console.log(diff);
                return {
                    ...prev,
                    progress: Math.min(Math.max(prev.progress + diff, 0), 1),
                };
            });
        } catch {}
    });

    const handleDeviceSwitch = useAsyncDebounce(async () => {
        const deviceType = playbackDevice === "Output" ? "Speaker" : "Output";
        if (status === undefined) {
            setPlaybackDevice(deviceType);
        } else {
            try {
                await invokeStrict("playback_start", {
                    id: status.id,
                    deviceType,
                    initialProgress: status.progress,
                    startPaused: status.status === "paused",
                });
                setPlaybackDevice(deviceType);
            } catch {}
        }
    });

    const handlePlayPause = useCallback(async () => {
        if (status === undefined) {
            if (selectedClip !== undefined) void handleStart(selectedClip.id, playbackDevice);
        } else if (status.status === "playing") {
            await handlePause();
        } else {
            await handleContinue();
        }
    }, [status, selectedClip, handleStart, playbackDevice, handlePause, handleContinue]);

    const handleSeekBack = useCallback(() => void handleSeek(-1), [handleSeek]);
    const handleSeekForward = useCallback(() => void handleSeek(1), [handleSeek]);

    const handleStartContinuously = useCallback(() => {
        if (selectedClip !== undefined) void handleStart(selectedClip.id, playbackDevice, true);
    }, [selectedClip, handleStart, playbackDevice]);

    const handlePrev = useCallback(async () => {
        await handleStop(false);
        intendedClipChangeRef.current = true;
        setSelected(prev => prev + 1);
        if (prevClip !== undefined) void handleStart(prevClip.id, playbackDevice);
    }, [handleStop, setSelected, handleStart, prevClip, playbackDevice]);

    const handleNext = useCallback(async () => {
        await handleStop(false);
        intendedClipChangeRef.current = true;
        setSelected(prev => prev - 1);
        if (nextClip !== undefined) void handleStart(nextClip.id, playbackDevice);
    }, [handleStop, setSelected, handleStart, nextClip, playbackDevice]);

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
            return {...prev, progress: event.payload};
        });
        if (event.payload === 1 && isPlaybackRoot()) {
            if (status?.continuously && nextClip !== undefined) {
                intendedClipChangeRef.current = true;
                setSelected(prev => prev - 1);
                void handleStart(nextClip.id, playbackDevice, true);
            } else {
                setStatus(undefined);
            }
        }
    });

    useEffect(() => {
        const unlisten = listen<number>("playback:progress", handleProgressUpdate);
        return () => unlisten.then(fn => fn());
    }, [handleProgressUpdate]);

    return {
        status,
        blink,
        playbackDevice,
        active: status !== undefined,
        handleStop,
        handlePlayPause,
        handleSeekBack,
        handleSeekForward,
        handleDeviceSwitch,
        handleStartContinuously,
        handlePrev,
        handleNext,
    };
}
