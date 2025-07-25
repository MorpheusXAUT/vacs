import Button from "../components/ui/Button.tsx";
import VolumeSlider from "../components/ui/VolumeSlider.tsx";
import mic from "../assets/mic.svg";
import headphones from "../assets/headphones.svg";
import mousePointerClick from "../assets/mouse-pointer-click.svg";
import bellRing from "../assets/bell-ring.svg";
import Select from "../components/ui/Select.tsx";
import {invoke} from "@tauri-apps/api/core";
import {navigate} from "wouter/use-browser-location";
import {useAuthStore} from "../stores/auth-store.ts";

function SettingsPage() {
    const isAuthenticated = useAuthStore(state => state.status === "authenticated");

    const handleLogoutClick = async () => {
        await invoke("logout");
        navigate("/");
    }

    return (
        <div className="h-full w-full bg-blue-700 border-t-0 px-2 pb-2 flex flex-col overflow-auto">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Settings</p>
            <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex flex-col">
                <div className="w-full grow border-b-2 border-zinc-200 flex flex-row">
                    <div className="h-full w-60 border-r-2 border-zinc-200 flex flex-col">
                        <p className="w-full text-center border-b-2 border-zinc-200 uppercase font-semibold">Operator</p>
                        <div className="w-full grow px-3 py-1.5 flex flex-row gap-3.5">
                            <div className="w-full flex flex-col items-center">
                                <img src={headphones} className="pt-1 h-12 w-12" alt="" />
                                <p className="font-bold text-center pt-3 pb-1">Headset</p>
                                <VolumeSlider />
                            </div>
                            <div className="w-full flex flex-col items-center">
                                <img src={mic} className="pt-1 h-12 w-12" alt="" />
                                <p className="font-bold text-center pt-3 pb-1">Microphone</p>
                                <VolumeSlider />
                            </div>
                        </div>
                    </div>
                    <div className="h-full w-60 border-r-2 border-zinc-200 flex flex-col">
                        <p className="w-full text-center border-b-2 border-zinc-200 uppercase font-semibold">Settings Touch Panel</p>
                        <div className="w-full grow px-3 py-1.5 flex flex-row gap-3.5">
                            <div className="w-full flex flex-col items-center">
                                <img src={mousePointerClick} className="pt-1 h-12 w-12" alt="" />
                                <p className="font-bold text-center pt-1 leading-4">Click<br/>Volume</p>
                                <VolumeSlider />
                            </div>
                            <div className="w-full flex flex-col items-center">
                                <img src={bellRing} className="pt-1 h-12 w-12" alt="" />
                                <p className="font-bold text-center pt-1 leading-4">Chime<br/>Volume</p>
                                <VolumeSlider />
                            </div>
                        </div>
                    </div>
                    <div className="h-full grow border-r-2 border-zinc-200 flex flex-col">
                        <p className="w-full text-center border-b-2 border-zinc-200 uppercase font-semibold">Devices</p>
                        <div className="w-full grow px-3 py-1.5 flex flex-col">
                            <p className="w-full text-center font-semibold">Headset</p>
                            <Select></Select>
                            <p className="w-full text-center font-semibold">Microphone</p>
                            <Select></Select>
                        </div>
                    </div>
                </div>
                <div className="h-20 w-full flex flex-row gap-2 justify-between p-2 [&>button]:px-1 [&>button]:shrink-0">
                    <Button color="gray" className="rounded !text-base">Side<br/>tones</Button>
                    {isAuthenticated && <Button color="gray" className="rounded !text-base" onClick={handleLogoutClick}>Logout</Button>}
                </div>
            </div>
        </div>
    );
}

export default SettingsPage;