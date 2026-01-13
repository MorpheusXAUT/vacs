import {clsx} from "clsx";
import Button from "./ui/Button.tsx";
import {useState} from "preact/hooks";
import {invokeSafe} from "../error.ts";
import {useProfileStore} from "../stores/profile-store.ts";
import {useRoute} from "wouter";
import {navigate} from "wouter/use-browser-location";

function Tabs() {
    const profile = useProfileStore(state => state.profile);
    const [active, setActive] = useState<number>(1);
    const [offset, setOffset] = useState<number>(0);

    if (profile === undefined || profile.tabbed === undefined) return <></>;

    const tabs = Object.keys(profile.tabbed);

    const visibleTabs = (() => {
        const visibleTabs = tabs.slice(offset, offset + 4);
        const toFill = Math.min(tabs.length, 4) - visibleTabs.length;
        for (let i = 0; i < toFill; i++) visibleTabs.push("");
        return visibleTabs;
    })();

    return (
        <div className="h-full flex flex-row">
            {tabs.length > 4 && (
                <Button
                    color="gray"
                    className={clsx("w-20 h-full mr-1")}
                    onClick={() =>
                        setOffset(offset => (offset >= tabs.length - 4 ? 0 : offset + 4))
                    }
                >
                    <p>
                        &lt;-&gt;
                        <br />
                        DA 5-8
                    </p>
                </Button>
            )}
            {visibleTabs.map((tab, index) => (
                <TabButton
                    key={index}
                    label={tab}
                    active={active === index + 1}
                    onClick={() => setActive(index + 1)}
                />
            ))}
        </div>
    );
}

type TabButtonProps = {
    label: string;
    active?: boolean;
    onClick?: () => void;
};

function TabButton(props: TabButtonProps) {
    const [settingsOpen] = useRoute("/settings/*?");

    return (
        <div className="w-20 relative">
            <button
                className={clsx(
                    "absolute -top-[calc(0.5rem+2px)] h-[calc(100%+0.5rem+2px)] w-20 rounded-b-lg border-t-0 font-semibold flex justify-center items-center cursor-pointer",
                    "border-4 outline-2 outline-gray-700 -outline-offset-2",
                    props.active && !settingsOpen
                        ? "active-tab border-transparent border-b-gray-300 bg-linear-0/oklch from-gray-300 to-[#B5BBC6]"
                        : "bg-gray-300 border-l-gray-100 border-r-gray-700 border-b-gray-700 active:border-r-gray-100 active:border-b-gray-100 active:border-t-gray-700 active:border-l-gray-700 active:*:translate-y-px active:*:translate-x-px",
                )}
                disabled={props.active && !settingsOpen}
                onClick={() => {
                    void invokeSafe("audio_play_ui_click");
                    props.onClick?.();
                    if (settingsOpen) navigate("/");
                }}
            >
                <p>{props.label}</p>
            </button>
        </div>
    );
}

export default Tabs;
