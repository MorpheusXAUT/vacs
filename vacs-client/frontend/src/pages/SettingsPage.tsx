import Button from "../components/ui/Button.tsx";
import {navigate} from "wouter/use-browser-location";
import {useAuthStore} from "../stores/auth-store.ts";
import {invokeSafe, invokeStrict} from "../error.ts";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {useSignalingStore} from "../stores/signaling-store.ts";
import DeviceSelector from "../components/DeviceSelector.tsx";
import VolumeSettings from "../components/VolumeSettings.tsx";
import AudioHostSelector from "../components/AudioHostSelector.tsx";
import {useEffect, useState} from "preact/hooks";
import {getCurrentWindow} from "@tauri-apps/api/window";
import {useUpdateStore} from "../stores/update-store.ts";
import TransmitModeSettings from "../components/TransmitModeSettings.tsx";
import {useCapabilitiesStore} from "../stores/capabilities-store.ts";

function SettingsPage() {
    return (
        <div className="h-full w-full bg-blue-700 border-t-0 px-2 pb-2 flex flex-col overflow-auto">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Settings</p>
            <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex flex-col">
                <div className="w-full grow border-b-2 border-zinc-200 flex flex-row">
                    <VolumeSettings/>
                    <div className="h-full grow flex flex-col">
                        <p className="w-full text-center border-b-2 border-zinc-200 uppercase font-semibold">Devices</p>
                        <div className="w-full px-3 py-1.5 flex flex-col">
                            <AudioHostSelector/>
                            <DeviceSelector deviceType="Output"/>
                            <DeviceSelector deviceType="Input"/>
                        </div>
                        <TransmitModeSettings/>
                    </div>
                </div>
                <div
                    className="h-20 w-full flex flex-row gap-2 justify-between p-2 [&>button]:px-1 [&>button]:shrink-0">
                    <div className="h-full flex flex-row gap-2 items-center">
                        <WindowStateButtons/>
                        <UpdateButton/>
                        <Button color="gray" className="w-24 h-full rounded text-sm"
                                onClick={() => invokeSafe("app_open_logs_folder")}>Open logs<br/>folder</Button>
                    </div>
                    <AppControlButtons/>
                </div>
            </div>
        </div>
    );
}

function AppControlButtons() {
    const connected = useSignalingStore(state => state.connectionState === "connected");
    const isAuthenticated = useAuthStore(state => state.status === "authenticated");

    const handleLogoutClick = useAsyncDebounce(async () => {
        try {
            await invokeStrict("auth_logout");
            navigate("/");
        } catch {
        }
    });

    const handleDisconnectClick = useAsyncDebounce(async () => {
        try {
            await invokeStrict("signaling_disconnect");
            navigate("/");
        } catch {
        }
    });

    const handleQuitClick = useAsyncDebounce(async () => {
        await invokeSafe("app_quit");
    });

    return (
        <div className="h-full flex flex-row gap-2">
            <Button color="salmon" className="w-auto px-3 text-sm rounded" disabled={!connected}
                    onClick={handleDisconnectClick}>Disconnect</Button>
            <Button color="salmon" className="text-sm rounded" disabled={!isAuthenticated}
                    onClick={handleLogoutClick}>Logout</Button>
            <Button color="salmon" className="text-sm rounded ml-3"
                    onClick={handleQuitClick}>Quit</Button>
        </div>
    );
}

function UpdateButton() {
    const [noNewVersion, setNoNewVersion] = useState<boolean>(false);
    const newVersion = useUpdateStore(state => state.newVersion);
    const {
        setVersions: setUpdateVersions,
        openMandatoryDialog,
        openDownloadDialog,
        closeOverlay
    } = useUpdateStore(state => state.actions);

    const handleOnClick = useAsyncDebounce(async () => {
        if (newVersion !== undefined) {
            try {
                openDownloadDialog();
                await invokeStrict("app_update");
            } catch {
                closeOverlay();
            }
        } else {
            const checkUpdateResult = await invokeSafe<{
                currentVersion: string,
                newVersion?: string,
                required: boolean
            }>("app_check_for_update");
            if (checkUpdateResult === undefined) return;

            setUpdateVersions(checkUpdateResult.currentVersion, checkUpdateResult.newVersion);

            if (checkUpdateResult.required) {
                openMandatoryDialog();
            } else {
                if (checkUpdateResult.newVersion === undefined) {
                    setNoNewVersion(true);
                }
                closeOverlay();
            }
        }
    });

    return (
        <Button
            color={newVersion === undefined ? "gray" : "green"}
            className="w-24 h-full rounded text-sm"
            onClick={handleOnClick}
            disabled={noNewVersion}
        >
            {newVersion === undefined ? (noNewVersion ? <p>No Update<br/>available</p> : <p>Check for<br/>Updates</p>) :
                <p>Update &<br/>Restart</p>}
        </Button>
    );
}

function WindowStateButtons() {
    const [alwaysOnTop, setAlwaysOnTop] = useState<boolean>(false);
    const [fullscreen, setFullscreen] = useState<boolean>(false);

    const capAlwaysOnTop = useCapabilitiesStore(state => state.alwaysOnTop);
    const capPlatform = useCapabilitiesStore(state => state.platform);

    const toggleAlwaysOnTop = useAsyncDebounce(async () => {
        const isAlwaysOnTop = await invokeSafe<boolean>("app_set_always_on_top", {alwaysOnTop: !alwaysOnTop});
        setAlwaysOnTop(alwaysOnTop => isAlwaysOnTop ?? alwaysOnTop);
    });

    const toggleFullscreen = useAsyncDebounce(async () => {
        const isFullscreen = await invokeSafe<boolean>("app_set_fullscreen", {fullscreen: !fullscreen});
        setFullscreen(isFullscreen ?? false);
    });

    const handleResetWindowSizeClick = useAsyncDebounce(async () => {
        try {
            await invokeStrict("app_reset_window_size");
            setFullscreen(false);
        } catch {
        }
    });

    useEffect(() => {
        const fetchIsAlwaysOnTop = async () => {
            const isAlwaysOnTop = await getCurrentWindow().isAlwaysOnTop();
            setAlwaysOnTop(alwaysOnTop => isAlwaysOnTop ?? alwaysOnTop);
        };
        const fetchIsFullscreen = async () => {
            const isFullscreen = await getCurrentWindow().isFullscreen();
            setFullscreen(isFullscreen);
        };

        void fetchIsAlwaysOnTop();
        void fetchIsFullscreen();
    }, []);

    return (
        <div className="h-full flex flex-row gap-2">
            <Button
                color={alwaysOnTop ? "blue" : "cyan"}
                className="h-full rounded text-sm"
                onClick={toggleAlwaysOnTop}
                disabled={!capAlwaysOnTop}
                title={!capAlwaysOnTop ? `Unfortunately, always-on-top is not yet supported on ${capPlatform}` : undefined}
            >
                <p>Always<br/>on Top</p>
            </Button>
            <Button
                color={fullscreen ? "blue" : "cyan"}
                className="h-full rounded text-sm"
                onClick={toggleFullscreen}
            >
                <p>Full<br/>Screen</p>
            </Button>
            <Button
                color="gray"
                className="h-full rounded text-sm"
                onClick={handleResetWindowSizeClick}
            >
                <p>Reset Size</p>
            </Button>
        </div>
    );
}

export default SettingsPage;
